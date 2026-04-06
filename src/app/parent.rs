//! State and helpers for displaying and managing the parent pane in runa.
//!
//! Tracks entries, selection, worker requests for the parent pane view above the current working
//! directory

use crate::app::NavigationData;
use crate::app::nav::SortConfig;
use crate::core::FileEntry;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Holds the state of the parent directory pane
///
/// Stores the list of entries in the parent directory, the selected entry (index)
/// and tracks the request IDs to coordinate updates.
#[derive(Default)]
pub(crate) struct ParentState {
    entries: Arc<[FileEntry]>,
    sort_column: Option<Arc<[Arc<str>]>>,
    selected_idx: Option<usize>,
    last_path: Option<PathBuf>,
    last_sort: Option<SortConfig>,
    request_id: u64,
}

impl ParentState {
    crate::getters! {
        request_id: u64,
        entries: &Arc<[FileEntry]>,
        sort_column: &Option<Arc<[Arc<str>]>>,
        selected_idx: Option<usize>,
    }

    pub(crate) fn last_path(&self) -> Option<&Path> {
        self.last_path.as_deref()
    }

    #[inline]
    pub(super) fn is_cached(&self, parent_path: &Path, sort: SortConfig) -> bool {
        matches!(
            self.last_path(),
            Some(last) if last == parent_path
        ) && self.last_sort == Some(sort)
            && !self.entries.is_empty()
    }

    pub(super) fn prepare_new_request(&mut self, path: &Path, sort: SortConfig) -> u64 {
        self.request_id = self.request_id.wrapping_add(1);
        self.last_path = Some(path.to_path_buf());
        self.last_sort = Some(sort);
        self.request_id
    }

    /// Updates the state with new entries
    ///
    /// Only applies the update if request ID is the latest
    pub(super) fn update_from_entries(
        &mut self,
        entries: Vec<FileEntry>,
        current_name: &str,
        req_id: u64,
        parent_path: &Path,
        sort: SortConfig,
        sort_column: Option<Vec<Arc<str>>>,
    ) {
        if req_id < self.request_id {
            return;
        }
        // Find the index of the folder we are currently inside to highlight it
        self.selected_idx = entries.iter().position(|e| e.name_str() == current_name);
        self.entries = Arc::from(entries);
        self.last_path = Some(parent_path.to_path_buf());
        self.last_sort = Some(sort);
        self.sort_column = sort_column.map(Arc::from);
        self.request_id = req_id;
    }

    pub(crate) fn try_share_directory(
        &self,
        parent_path: &Path,
        sort: SortConfig,
    ) -> NavigationData {
        if self.last_path.as_deref() == Some(parent_path)
            && !self.entries.is_empty()
            && self.last_sort == Some(sort)
        {
            Some((
                Arc::clone(&self.entries),
                self.sort_column.as_ref().map(Arc::clone),
            ))
        } else {
            None
        }
    }

    /// Clears all entries, resets the selected entry index,
    /// resets the last path and increases the request_id
    pub(super) fn clear(&mut self) {
        self.entries = Arc::default();
        self.selected_idx = None;
        self.last_path = None;
        self.last_sort = None;
        self.request_id = self.request_id.wrapping_add(1);
    }
}
