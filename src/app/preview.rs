//! State and helpers for displaying and managing the preview pane in runa.
//!
//! Tracks the state of the file/directory preview for the UI, including loaded preview
//! data, debounce for background rendering, selection within the preview and request tracking

use crate::core::FileEntry;
use std::path::PathBuf;
use std::time::Instant;

/// Preview content for the preview pane
///
/// Holds loaded lines for file preview, directory entries for folder preview or empty if nothing.
/// Used to display or render file/folder content in the preview pane
pub enum PreviewData {
    Directory(Vec<FileEntry>),
    File(Vec<String>),
    Empty,
}

/// State and helpers for managing the preview pane.
///
/// Holds:
/// - PreviewData
/// - the selected index
/// - the current path
/// - the workers request_id
/// - a pending flag to indicate if a preview request is pending
/// - a Directory generation int to correctly sync preview data with directory nav
/// - and the last input time to handle status notifaction.
pub struct PreviewState {
    data: PreviewData,
    selected_idx: usize,
    current_path: Option<PathBuf>,
    request_id: u64,
    pending: bool,
    last_input_time: Instant,
}

impl PreviewState {
    // Getters/ Accessors

    pub fn data(&self) -> &PreviewData {
        &self.data
    }

    pub fn selected_idx(&self) -> usize {
        self.selected_idx
    }

    pub fn request_id(&self) -> u64 {
        self.request_id
    }

    pub fn current_path(&self) -> Option<&PathBuf> {
        self.current_path.as_ref()
    }

    // Setters / mutators

    /// Sets the selected index, clamped to the length of the current data
    pub fn set_selected_idx(&mut self, idx: usize) {
        let len = match &self.data {
            PreviewData::Directory(entries) => entries.len(),
            PreviewData::File(lines) => lines.len(),
            PreviewData::Empty => 0,
        };
        self.selected_idx = idx.min(len.saturating_sub(1));
    }

    /// Marks the preview as pending and updates the last input time
    pub fn mark_pending(&mut self) {
        self.pending = true;
        self.last_input_time = Instant::now();
    }

    // Debounce timing for preview render
    pub fn should_trigger(&self) -> bool {
        self.pending && self.last_input_time.elapsed().as_millis() > 75
    }

    /// Prepares a new preview request for the given path
    /// Increments the request ID, sets the current path and marks as not pending
    pub fn prepare_new_request(&mut self, path: PathBuf) -> u64 {
        self.request_id = self.request_id.wrapping_add(1);
        self.current_path = Some(path);
        self.pending = false;
        self.request_id
    }

    /// Updates the preview content with new file lines
    /// Only applies the update if the request ID matches the latest
    pub fn update_content(&mut self, lines: Vec<String>, request_id: u64) {
        if request_id == self.request_id {
            self.data = PreviewData::File(lines);
        }
    }

    /// Updates the preview content with new directory entries
    /// Only applies the update if the request ID matches the latest
    pub fn update_from_entries(&mut self, entries: Vec<FileEntry>, request_id: u64) {
        if request_id == self.request_id {
            self.data = PreviewData::Directory(entries);
            self.selected_idx = 0;
        }
    }

    /// Sets an error message as the preview content
    pub fn set_error(&mut self, err: String) {
        self.data = PreviewData::File(vec![err]);
    }

    /// Clears the preview state
    pub fn clear(&mut self) {
        self.data = PreviewData::Empty;
        self.selected_idx = 0;
        self.current_path = None;
        self.pending = false;
    }
}

impl PreviewData {
    /// Determines if the preview data is empty
    pub fn is_empty(&self) -> bool {
        match self {
            PreviewData::Directory(v) => v.is_empty(),
            PreviewData::File(v) => v.is_empty(),
            PreviewData::Empty => true,
        }
    }

    /// Returns an iterator over the directory entries if the preview data is a directory
    pub fn iter(&self) -> impl Iterator<Item = &FileEntry> {
        match self {
            PreviewData::Directory(entries) => entries.iter(),
            _ => [].iter(),
        }
    }

    /// Returns the length of the directory entries if the preview data is a directory
    pub fn len(&self) -> usize {
        match self {
            PreviewData::Directory(entries) => entries.len(),
            _ => 0,
        }
    }
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            data: PreviewData::Empty,
            selected_idx: 0,
            current_path: None,
            request_id: 0,
            pending: false,
            last_input_time: Instant::now(),
        }
    }
}
