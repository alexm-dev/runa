//! InfoState module for AppState usage.
//!
//! [PropertyState] struct to wrap the [FileMetadata] and manage the state of worker requests,
//! pending paths, and selected file metadata.

use crate::core::metadata::FileMetadata;

#[cfg(unix)]
use {crate::core::file::unix_info::UserGroupCache, std::cell::RefCell};

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(crate) struct PropertyState {
    request_id: u64,
    pending: Option<(u64, PathBuf)>,
    selected: Option<Arc<FileMetadata>>,
    last_request_time: Instant,
    #[cfg(unix)]
    cache: RefCell<UserGroupCache>,
}

impl PropertyState {
    pub(crate) fn new() -> Self {
        Self {
            request_id: 0,
            pending: None,
            selected: None,
            last_request_time: Instant::now() - Duration::from_secs(1),
            #[cfg(unix)]
            cache: RefCell::new(UserGroupCache::new()),
        }
    }

    #[cfg(unix)]
    pub(crate) fn resolve_owner(&self, info: &FileMetadata) -> Option<Arc<str>> {
        Some(self.cache.borrow_mut().resolve_user(info.uid()))
    }

    #[cfg(unix)]
    pub(crate) fn resolve_group(&self, info: &FileMetadata) -> Option<Arc<str>> {
        Some(self.cache.borrow_mut().resolve_group(info.gid()))
    }

    pub(crate) fn prepare_new_request(&mut self) -> u64 {
        self.last_request_time = Instant::now();
        let id = self.request_id;
        self.request_id = self.request_id.wrapping_add(1);
        id
    }

    pub(crate) fn is_pending_path(&self, path: &Path) -> bool {
        self.pending.as_ref().is_some_and(|(_, p)| p == path)
    }

    pub(crate) fn can_request(&self, debounce_ms: u64) -> bool {
        self.last_request_time.elapsed() >= Duration::from_millis(debounce_ms)
    }

    pub(crate) fn set_pending(&mut self, id: u64, path: PathBuf) {
        self.pending = Some((id, path));
    }

    pub(crate) fn matches_pending(&self, id: u64, path: &Path) -> bool {
        if let Some((pid, p)) = &self.pending {
            *pid == id && p == path
        } else {
            false
        }
    }

    pub(crate) fn selected(&self) -> Option<&FileMetadata> {
        self.selected.as_deref()
    }

    pub(crate) fn selected_arc(&self) -> Option<&Arc<FileMetadata>> {
        self.selected.as_ref()
    }

    pub(crate) fn set_selected(&mut self, meta: Option<Arc<FileMetadata>>) {
        self.selected = meta;
    }

    pub(crate) fn clear_pending(&mut self) {
        self.pending = None;
    }

    pub(crate) fn clear(&mut self) {
        self.pending = None;
        self.selected = None;
    }
}
