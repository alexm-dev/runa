//! Navigation state and file list logic for runa.
//!
//! Manages the current directory, file entries, selection, markers and filters.
//! Provides helpers for pane navigation, selection, filtering, and bulk actions.

use crate::core::FileEntry;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Holds the navigation, selection and file list state of a pane.
pub struct NavState {
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    selected: usize,
    positions: HashMap<PathBuf, usize>,
    markers: HashSet<PathBuf>,
    filter: String,
    filters: HashMap<PathBuf, String>,
    request_id: u64,
}

impl NavState {
    pub fn new(path: PathBuf) -> Self {
        Self {
            current_dir: path,
            entries: Vec::new(),
            selected: 0,
            positions: HashMap::new(),
            markers: HashSet::new(),
            filter: String::new(),
            filters: HashMap::new(),
            request_id: 0,
        }
    }

    // Getters / Accessors

    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    pub fn markers(&self) -> &HashSet<PathBuf> {
        &self.markers
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn selected_idx(&self) -> usize {
        self.selected
    }

    pub fn request_id(&self) -> u64 {
        self.request_id
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.selected_shown_entry()
    }

    // Navigation functions

    pub fn prepare_new_request(&mut self) -> u64 {
        self.request_id = self.request_id.wrapping_add(1);
        self.request_id
    }

    pub fn move_up(&mut self) -> bool {
        let len = self.shown_entries_len();
        if len == 0 {
            return false;
        }

        if self.selected == 0 {
            self.selected = len - 1;
        } else {
            self.selected -= 1;
        }
        true
    }

    pub fn move_down(&mut self) -> bool {
        let len = self.shown_entries_len();
        if len == 0 {
            return false;
        }

        self.selected = (self.selected + 1) % len;
        true
    }

    pub fn save_position(&mut self) {
        if !self.entries.is_empty() {
            self.positions
                .insert(self.current_dir.clone(), self.selected);
        }
    }

    pub fn get_position(&self) -> &HashMap<PathBuf, usize> {
        &self.positions
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.save_position();

        self.current_dir = path;
        self.entries.clear();
        self.selected = 0;
        self.restore_filter_for_current_dir();
        // instantly ends all pending messages from the previous directory.
        self.request_id = self.request_id.wrapping_add(1);
    }

    pub fn set_selected(&mut self, idx: usize) {
        let max = self.shown_entries_len();
        self.selected = if max == 0 {
            0
        } else if idx >= max {
            max - 1
        } else {
            idx
        };
    }

    pub fn update_from_worker(
        &mut self,
        path: PathBuf,
        entries: Vec<FileEntry>,
        focus: Option<OsString>,
    ) {
        self.current_dir = path;
        self.entries = entries;

        if let Some(f) = focus {
            self.selected = self
                .entries
                .iter()
                .position(|e| e.name() == &f)
                .unwrap_or(0);
        } else {
            self.selected = self.positions.get(&self.current_dir).cloned().unwrap_or(0);
        }

        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
    }

    pub fn toggle_marker(&mut self) {
        if let Some(entry) = self.selected_shown_entry() {
            let path = self.current_dir().join(entry.name());
            if !self.markers.remove(&path) {
                self.markers.insert(path);
            }
        }
    }

    pub fn toggle_marker_advance(&mut self) {
        self.toggle_marker();
        let count = self.shown_entries_len();

        if count > 1 && self.selected + 1 < count {
            self.selected += 1;
        } else if count > 1 {
            self.selected = 0;
        }
    }

    pub fn clear_markers(&mut self) {
        self.markers.clear();
    }

    pub fn get_action_targets(&self) -> HashSet<PathBuf> {
        if self.markers.is_empty() {
            self.selected_entry()
                .map(|e| self.current_dir.join(e.name()))
                .into_iter()
                .collect()
        } else {
            self.markers.iter().cloned().collect()
        }
    }

    // Filter functions

    pub fn shown_entries(&self) -> Box<dyn Iterator<Item = &FileEntry> + '_> {
        if self.filter.is_empty() {
            Box::new(self.entries.iter())
        } else {
            let filter_lower = self.filter.to_lowercase();

            Box::new(
                self.entries
                    .iter()
                    .filter(move |e| e.lowercase_name().contains(&filter_lower)),
            )
        }
    }

    pub fn shown_entries_len(&self) -> usize {
        if self.filter.is_empty() {
            self.entries.len()
        } else {
            let filter_lower = self.filter.to_lowercase();
            self.entries
                .iter()
                .filter(|e| e.lowercase_name().contains(&filter_lower))
                .count()
        }
    }

    pub fn selected_shown_entry(&self) -> Option<&FileEntry> {
        self.shown_entries().nth(self.selected)
    }

    pub fn set_filter(&mut self, filter: String) {
        if self.filter == filter {
            return;
        }

        let target_name = self.selected_shown_entry().map(|e| e.name().to_os_string());
        self.save_filter_for_current_dir();

        self.filter = filter;

        let new_idx = if let Some(ref name) = target_name {
            self.shown_entries()
                .position(|e| e.name() == name.as_os_str())
        } else {
            None
        };

        self.selected = new_idx.unwrap_or(0);
    }

    pub fn clear_filters(&mut self) {
        self.filter.clear();
        self.save_filter_for_current_dir();
    }

    fn save_filter_for_current_dir(&mut self) {
        if self.filter.is_empty() {
            self.filters.remove(&self.current_dir);
        } else {
            self.filters
                .insert(self.current_dir.clone(), self.filter.clone());
        }
    }

    fn restore_filter_for_current_dir(&mut self) {
        self.filter = self
            .filters
            .get(&self.current_dir)
            .cloned()
            .unwrap_or_default();
    }
}
