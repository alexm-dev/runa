//! InfoState module for AppState usage.
//!
//! [InfoState] struct to wrap the [CachedFileInfo] from `core::file_info` and
//! manage the state of info requests, pending paths, and selected file info.
//!
//! This module provides methods to prepare new info requests, check pending paths,
//! debounce requests, and manage the selected file info state.

use crate::core::file_info::FileInfo;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(crate) struct InfoState {
    request_id: u64,
    pending: Option<(u64, PathBuf)>,
    selected_info: Option<Arc<FileInfo>>,
    last_request_time: Instant,
}

impl InfoState {
    pub(crate) fn new() -> Self {
        Self {
            request_id: 0,
            pending: None,
            selected_info: None,
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
        if let Some((pid, p)) = &self.pending {
            *pid == id && p == path
        } else {
            false
        }
    }

    pub(crate) fn selected_info(&self) -> Option<&FileInfo> {
        self.selected_info.as_deref()
    }

    pub(crate) fn selected_info_arc(&self) -> Option<&Arc<FileInfo>> {
        self.selected_info.as_ref()
    }

    pub(crate) fn set_selected_info(&mut self, info: Option<Arc<FileInfo>>) {
        self.selected_info = info;
    }

    pub(crate) fn clear_pending(&mut self) {
        self.pending = None;
    }

    pub(crate) fn clear(&mut self) {
        self.pending = None;
        self.selected_info = None;
    }
}
