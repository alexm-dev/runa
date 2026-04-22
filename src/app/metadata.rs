//! File metadata module for AppState usage.
//!
//! [MetadataState] struct to wrap the [FileMetadata] and manage the state of worker requests,
//! pending paths, and selected file metadata.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::metadata::FileMetadataCache;
use crate::utils::timings::Throttler;

#[derive(Debug, Clone)]
pub(super) struct MetadataState {
    request_id: u64,
    pending: Option<(u64, PathBuf)>,
    selected: Option<Arc<FileMetadataCache>>,
    last_request_time: Throttler,
}

impl MetadataState {
    pub(super) fn new() -> Self {
        Self {
            request_id: 0,
            pending: None,
            selected: None,
            last_request_time: Throttler::new(),
        }
    }

    pub(super) fn prepare_new_request(&mut self) -> u64 {
        let id = self.request_id;
        self.request_id = self.request_id.wrapping_add(1);
        id
    }

    pub(super) fn is_pending_path(&self, path: &Path) -> bool {
        self.pending.as_ref().is_some_and(|(_, p)| p == path)
    }

    pub(super) fn can_request(&self, debounce_ms: u64) -> bool {
        self.last_request_time.can_trigger(debounce_ms)
    }

    pub(super) fn touch(&mut self) {
        self.last_request_time.touch();
    }

    pub(super) fn set_pending(&mut self, id: u64, path: PathBuf) {
        self.pending = Some((id, path));
    }

    pub(super) fn matches_pending(&self, id: u64, path: &Path) -> bool {
        matches!(&self.pending, Some((pid, p)) if *pid == id && p == path )
    }

    pub(super) fn selected(&self) -> Option<&FileMetadataCache> {
        self.selected.as_deref()
    }

    pub(super) fn selected_arc(&self) -> Option<&Arc<FileMetadataCache>> {
        self.selected.as_ref()
    }

    pub(super) fn set_selected(&mut self, meta: Option<Arc<FileMetadataCache>>) {
        self.selected = meta;
    }

    pub(super) fn clear_pending(&mut self) {
        self.pending = None;
    }

    pub(super) fn clear(&mut self) {
        self.pending = None;
        self.selected = None;
    }
}
