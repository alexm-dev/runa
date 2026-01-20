//! State and helpers for displaying and managing the parent pane in runa.
//!
//! Tracks entries, selection, worker requests for the parent pane view above the current working
//! directory

use crate::core::FileEntry;
use std::path::{Path, PathBuf};

/// Holds the state of the parent directory pane
///
/// Stores the list of entries in the parent directory, the selected entry (index)
/// and tracks the request IDs to coordinate updates.
#[derive(Default)]
pub(crate) struct ParentState {
    entries: Vec<FileEntry>,
    selected_idx: Option<usize>,
    last_path: Option<PathBuf>,
    request_id: u64,
}

impl ParentState {
    // Getters / accessors

    #[inline]
    pub(crate) fn request_id(&self) -> u64 {
        self.request_id
    }

    #[inline]
    pub(crate) fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    #[inline]
    pub(crate) fn selected_idx(&self) -> Option<usize> {
        self.selected_idx
    }

    pub(crate) fn last_path(&self) -> Option<&Path> {
        self.last_path.as_deref()
    }

    /// Determines if a worker request should be issued for the given parent directory.
    ///
    /// Returns true if entries are empty or if the path has changed since the last refresh
    pub(crate) fn should_request(&self, parent_path: &Path) -> bool {
        if self.entries.is_empty() {
            return true;
        }
        Some(parent_path) != self.last_path.as_deref()
    }

    /// Updates the state with new entries
    ///
    /// Only applies the update if request ID is the latest
    pub(crate) fn update_from_entries(
        &mut self,
        entries: Vec<FileEntry>,
        current_name: &str,
        req_id: u64,
        parent_path: &Path,
    ) {
        if req_id < self.request_id {
            return;
        }
        // Find the index of the folder we are currently inside to highlight it
        self.selected_idx = entries.iter().position(|e| e.name_str() == current_name);
        self.entries = entries;
        self.last_path = Some(parent_path.to_path_buf());
        self.request_id = req_id;
    }

    /// Clears all entries, resets the selected entry index,
    /// resets the last path and increases the request_id
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.selected_idx = None;
        self.last_path = None;
        self.request_id = self.request_id.wrapping_add(1);
    }
}
