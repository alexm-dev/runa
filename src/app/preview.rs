//! State and helpers for displaying and managing the preview pane in runa.
//!
//! Tracks the state of the file/directory preview for the UI, including loaded preview
//! data, debounce for background rendering, selection within the preview and request tracking

use crate::core::FileEntry;
use ansi_to_tui::IntoText;
use ratatui::text::Text;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Preview content for the preview pane
///
/// Holds loaded lines for file preview, directory entries for folder preview or empty if nothing.
/// Used to display or render file/folder content in the preview pane
pub(crate) enum PreviewData {
    Directory(Vec<FileEntry>),
    File(Text<'static>),
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
pub(crate) struct PreviewState {
    data: PreviewData,
    selected_idx: usize,
    current_path: Option<PathBuf>,
    request_id: u64,
    pending: bool,
    last_input_time: Instant,
}

impl PreviewState {
    // Getters/ Accessors

    #[inline]
    pub(crate) fn data(&self) -> &PreviewData {
        &self.data
    }

    #[inline]
    pub(crate) fn selected_idx(&self) -> usize {
        self.selected_idx
    }

    #[inline]
    pub(crate) fn request_id(&self) -> u64 {
        self.request_id
    }

    pub(crate) fn current_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }

    // Setters / mutators

    /// Sets the selected index, clamped to the length of the current data
    pub(crate) fn set_selected_idx(&mut self, idx: usize) {
        let len = match &self.data {
            PreviewData::Directory(entries) => entries.len(),
            PreviewData::File(_) => 1,
            PreviewData::Empty => 0,
        };
        self.selected_idx = idx.min(len.saturating_sub(1));
    }

    /// Marks the preview as pending and updates the last input time
    pub(crate) fn mark_pending(&mut self) {
        self.pending = true;
        self.last_input_time = Instant::now();
    }

    // Debounce timing for preview render
    pub(crate) fn should_trigger(&self) -> bool {
        self.pending && self.last_input_time.elapsed().as_millis() > 35
    }

    /// Prepares a new preview request for the given path
    /// Increments the request ID, sets the current path and marks as not pending
    pub(crate) fn prepare_new_request(&mut self, path: PathBuf) -> u64 {
        self.request_id = self.request_id.wrapping_add(1);
        self.current_path = Some(path);
        self.pending = false;
        self.request_id
    }

    /// Updates the preview content with new file lines
    /// Only applies the update if the request ID matches the latest
    pub(crate) fn update_content(&mut self, lines: Vec<String>, request_id: u64) {
        if request_id == self.request_id {
            let raw = lines.join("\n");
            let text: Text<'static> = raw.into_text().unwrap_or_else(|_| Text::from(raw));
            self.data = PreviewData::File(text);
        }
    }

    /// Updates the preview content with new directory entries
    /// Only applies the update if the request ID matches the latest
    pub(crate) fn update_from_entries(&mut self, entries: Vec<FileEntry>, request_id: u64) {
        if request_id == self.request_id {
            self.data = PreviewData::Directory(entries);
            self.selected_idx = 0;
        }
    }

    /// Sets an error message as the preview content
    pub(crate) fn set_error(&mut self, err: String) {
        self.data = PreviewData::File(Text::from(err));
    }

    /// Clears the preview state
    pub(crate) fn clear(&mut self) {
        self.data = PreviewData::Empty;
        self.selected_idx = 0;
        self.current_path = None;
        self.pending = false;
    }
}

impl PreviewData {
    /// Determines if the preview data is empty
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        match self {
            PreviewData::Directory(v) => v.is_empty(),
            PreviewData::File(text) => text.lines.is_empty(),
            PreviewData::Empty => true,
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
