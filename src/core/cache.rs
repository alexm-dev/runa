//! Cache for sharing of entries betweeen navigation changes.
//!
//! Caches an Arc slice of FileEntry and the sort_column needed
//! to share entry states between panes.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use dashmap::{DashMap, mapref::entry::Entry};

use crate::core::{FileEntry, sort::SortConfig};
use crate::utils::text::StrBuffer;

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
    path: Arc<Path>,
    sort: SortConfig,
    list: DirListOptions,
}

impl DirCacheKey {
    fn new(path: &Path, sort: SortConfig, list_options: &DirListOptions) -> Self {
        Self {
            path: Arc::from(path),
            sort,
            list: list_options.clone(),
        }
    }
}

pub(crate) type DirCacheValue = Arc<(Arc<[FileEntry]>, Option<Arc<StrBuffer>>, u64, Instant)>;

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
        self.inner.get(&key).map(|v| Arc::clone(v.value()))
    }

    pub(crate) fn insert_if_newer(
        &self,
        path: &Path,
        sort: SortConfig,
        list_options: &DirListOptions,
        entries: Arc<[FileEntry]>,
        sort_column: Option<Arc<StrBuffer>>,
        request_id: u64,
    ) {
        if self.inner.len() >= DIR_CACHE_CAP {
            let to_remove = self
                .inner
                .iter()
                .min_by_key(|entry| entry.value().3)
                .map(|entry| entry.key().clone());
            if let Some(key) = to_remove {
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
        self.inner.retain(|key, _| &*key.path != path);
    }
}
