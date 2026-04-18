//! Cache for sharing of entries betweeen navigation changes.
//!
//! Caches an Arc slice of FileEntry and the sort_column needed
//! to share entry states between panes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;

use crate::app::nav::SortConfig;
use crate::core::FileEntry;

const DIR_CACHE_CAP: usize = 30;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct DirListOptions {
    pub(crate) dirs_first: bool,
    pub(crate) show_hidden: bool,
    pub(crate) show_symlink: bool,
    pub(crate) show_system: bool,
    pub(crate) case_insensitive: bool,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct BaseKey {
    path: PathBuf,
    list: DirListOptions,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct SortKey {
    path: PathBuf,
    sort: SortConfig,
    list: DirListOptions,
}

type BaseEntries = Arc<[FileEntry]>;
type Permutation = Arc<[u32]>;
type BaseValue = (BaseEntries, u64, Instant);
type SortValue = (Permutation, Option<Arc<[Arc<str>]>>, u64, Instant);

pub(crate) type DirCacheValue = Arc<(Arc<[FileEntry]>, Option<Arc<[Arc<str>]>>, u64, Instant)>;

pub(crate) struct DirCache {
    base: DashMap<BaseKey, BaseValue>,
    sort: DashMap<SortKey, SortValue>,
}

impl DirCache {
    pub(crate) fn new() -> Self {
        Self {
            base: DashMap::new(),
            sort: DashMap::new(),
        }
    }

    pub(crate) fn get(
        &self,
        path: &Path,
        sort: SortConfig,
        list_options: &DirListOptions,
    ) -> Option<DirCacheValue> {
        let base_key = BaseKey {
            path: path.to_owned(),
            list: list_options.clone(),
        };

        let base_val = self.base.get(&base_key)?;
        let (base_entries, base_req, base_ts) = base_val.value();

        let sort_key = SortKey {
            path: path.to_owned(),
            sort,
            list: list_options.clone(),
        };

        if let Some(sort_val) = self.sort.get(&sort_key) {
            let (perm, sort_col, sort_req, sort_ts) = sort_val.value();
            if perm.len() == base_entries.len() {
                let mut vec: Vec<FileEntry> = Vec::with_capacity(perm.len());
                for &idx in perm.iter() {
                    let idx = idx as usize;
                    vec.push(base_entries[idx].clone());
                }
                let sorted_arc = Arc::from(vec.into_boxed_slice());
                return Some(Arc::new((
                    sorted_arc,
                    sort_col.clone(),
                    *sort_req,
                    *sort_ts,
                )));
            } else {
                self.sort.remove(&sort_key);
                return Some(Arc::new((
                    Arc::clone(base_entries),
                    Some(Arc::new([])),
                    *base_req,
                    *base_ts,
                )));
            }
        }

        Some(Arc::new((
            Arc::clone(base_entries),
            None,
            *base_req,
            *base_ts,
        )))
    }

    pub(super) fn insert_base(
        &self,
        path: &Path,
        list_options: &DirListOptions,
        entries: BaseEntries,
        request_id: u64,
    ) {
        let key = BaseKey {
            path: path.to_owned(),
            list: list_options.clone(),
        };

        let now = Instant::now();
        self.base.insert(key, (entries, request_id, now));

        let mut to_remove = Vec::new();
        for kv in self.sort.iter() {
            if kv.key().path == path {
                to_remove.push(kv.key().clone());
            }
        }

        for k in to_remove {
            self.sort.remove(&k);
        }
    }

    pub(super) fn insert_sort_permutation(
        &self,
        path: &Path,
        sort: SortConfig,
        list_options: &DirListOptions,
        perm: Permutation,
        sort_column: Option<Arc<[Arc<str>]>>,
        request_id: u64,
    ) {
        let key = SortKey {
            path: path.to_owned(),
            sort,
            list: list_options.clone(),
        };
        let now = Instant::now();
        self.sort.insert(key, (perm, sort_column, request_id, now));

        while self.sort.len() > DIR_CACHE_CAP {
            let mut oldest_key: Option<SortKey> = None;
            let mut oldest_time = Instant::now();
            for entry in self.sort.iter() {
                let time = entry.value().3;
                if time < oldest_time {
                    oldest_time = time;
                    oldest_key = Some(entry.key().clone());
                }
            }
            if let Some(k) = oldest_key {
                self.sort.remove(&k);
            } else {
                break;
            }
        }
    }

    pub(crate) fn insert_if_newer(
        &self,
        path: &Path,
        sort: SortConfig,
        list_options: &DirListOptions,
        entries: Arc<[FileEntry]>,
        sort_column: Option<Arc<[Arc<str>]>>,
        request_id: u64,
    ) {
        self.insert_base(path, list_options, Arc::clone(&entries), request_id);

        let base_key = BaseKey {
            path: path.to_owned(),
            list: list_options.clone(),
        };

        if let Some(base_val) = self.base.get(&base_key) {
            let (base_entries, _rid, _ts) = base_val.value();
            if base_entries.len() == entries.len() {
                let mut map: HashMap<&str, usize> = HashMap::with_capacity(base_entries.len());
                for (i, fe) in base_entries.iter().enumerate() {
                    map.insert(fe.name_str(), i);
                }

                let mut perm_vec: Vec<u32> = Vec::with_capacity(entries.len());
                let mut ok = true;
                for e in entries.iter() {
                    if let Some(&idx) = map.get(e.name_str()) {
                        perm_vec.push(idx as u32);
                    } else {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    let perm = Arc::from(perm_vec.into_boxed_slice());
                    self.insert_sort_permutation(
                        path,
                        sort,
                        list_options,
                        perm,
                        sort_column,
                        request_id,
                    );
                }
            }
        }
    }

    pub(crate) fn invalidate_path(&self, path: &Path) {
        let mut to_remove_base = Vec::new();
        for kv in self.base.iter() {
            if kv.key().path == path {
                to_remove_base.push(kv.key().clone());
            }
        }
        for k in to_remove_base {
            self.base.remove(&k);
        }

        let mut to_remove_sort = Vec::new();
        for kv in self.sort.iter() {
            if kv.key().path == path {
                to_remove_sort.push(kv.key().clone());
            }
        }
        for k in to_remove_sort {
            self.sort.remove(&k);
        }
    }
}
