//! File metadata module for AppState usage.
//!
//! [MetadataState] struct to wrap the [FileMetadata] and manage the state of worker requests,
//! pending paths, and selected file metadata.

use crate::core::metadata::FileMetadataCache;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(crate) struct MetadataState {
    request_id: u64,
    pending: Option<(u64, PathBuf)>,
    selected: Option<Arc<FileMetadataCache>>,
    last_request_time: Instant,
}

impl MetadataState {
    pub(crate) fn new() -> Self {
        Self {
            request_id: 0,
            pending: None,
            selected: None,
            last_request_time: Instant::now() - Duration::from_secs(1),
        }
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
        matches!(&self.pending, Some((pid, p)) if *pid == id && p == path )
    }

    pub(crate) fn selected(&self) -> Option<&FileMetadataCache> {
        self.selected.as_deref()
    }

    pub(crate) fn selected_arc(&self) -> Option<&Arc<FileMetadataCache>> {
        self.selected.as_ref()
    }

    pub(crate) fn set_selected(&mut self, meta: Option<Arc<FileMetadataCache>>) {
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
