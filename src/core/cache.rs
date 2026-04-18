//! Cache for sharing of entries betweeen navigation changes.
//!
//! Caches an Arc slice of FileEntry and the sort_column needed
//! to share entry states between panes.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use dashmap::{DashMap, mapref::entry::Entry};

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
struct DirCacheKey {
    path: PathBuf,
    sort: SortConfig,
    list: DirListOptions,
}

impl DirCacheKey {
    fn new(path: &Path, sort: SortConfig, list_options: &DirListOptions) -> Self {
        Self {
            path: path.to_owned(),
            sort,
            list: list_options.clone(),
        }
    }
}

pub(crate) type DirCacheValue = Arc<(Arc<[FileEntry]>, Option<Arc<[Arc<str>]>>, u64, Instant)>;

pub(crate) struct DirCache {
    inner: DashMap<DirCacheKey, DirCacheValue>,
}

impl DirCache {
    pub(crate) fn new() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }

    pub(crate) fn get(
        &self,
        path: &Path,
        sort: SortConfig,
        list_options: &DirListOptions,
    ) -> Option<DirCacheValue> {
        let key = DirCacheKey::new(path, sort, list_options);
        self.inner.get(&key).map(|v| Arc::clone(&*v))
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
        if self.inner.len() > DIR_CACHE_CAP {
            let mut oldest_key = None;
            let mut oldest_time = Instant::now();

            for entry in self.inner.iter() {
                let time = entry.value().3;
                if time < oldest_time {
                    oldest_time = time;
                    oldest_key = Some(entry.key().clone());
                }
            }

            if let Some(key) = oldest_key {
                self.inner.remove(&key);
            }
        }

        let key = DirCacheKey::new(path, sort, list_options);
        let new_value: DirCacheValue = Arc::new((entries, sort_column, request_id, Instant::now()));
        match self.inner.entry(key) {
            Entry::Occupied(mut occ) => {
                let existing = occ.get();
                let exisiting_req = existing.2;
                if request_id >= exisiting_req {
                    occ.insert(new_value);
                }
            }
            Entry::Vacant(vac) => {
                vac.insert(new_value);
            }
        }
    }

    pub(crate) fn invalidate_path(&self, path: &Path) {
        self.inner.retain(|key, _| key.path != path);
    }
}
