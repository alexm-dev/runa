//! Navigation state and file list logic for runa.
//!
//! Manages the current directory, file entries, selection, markers and filters.
//! Provides helpers for pane navigation, selection, filtering, and bulk actions.

use crate::core::FileEntry;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Holds the navigation, selection and file list state of a pane.
pub(crate) struct NavState {
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
    pub(crate) fn new(path: PathBuf) -> Self {
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

    #[inline]
    pub(crate) fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    #[inline]
    pub(crate) fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    #[inline]
    pub(crate) fn markers(&self) -> &HashSet<PathBuf> {
        &self.markers
    }

    #[inline]
    pub(crate) fn filter(&self) -> &str {
        &self.filter
    }

    #[inline]
    pub(crate) fn selected_idx(&self) -> usize {
        self.selected
    }

    #[inline]
    pub(crate) fn request_id(&self) -> u64 {
        self.request_id
    }

    pub(crate) fn selected_entry(&self) -> Option<&FileEntry> {
        self.selected_shown_entry()
    }

    // Navigation functions

    /// Prepares a new request by incrementing the request ID.
    pub(crate) fn prepare_new_request(&mut self) -> u64 {
        self.request_id = self.request_id.wrapping_add(1);
        self.request_id
    }

    /// Moves the selection up by one entry, wrapping around if necessary.
    /// Returns `true` if the selection was moved, `false` if there are no entries.
    pub(crate) fn move_up(&mut self) -> bool {
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

    /// Moves the selection down by one entry, wrapping around if necessary.
    /// Returns `true` if the selection was moved, `false` if there are no entries.
    pub(crate) fn move_down(&mut self) -> bool {
        let len = self.shown_entries_len();
        if len == 0 {
            return false;
        }

        self.selected = (self.selected + 1) % len;
        true
    }

    /// Saves the current selection position for the current directory.
    pub(crate) fn save_position(&mut self) {
        if !self.entries.is_empty() {
            self.positions
                .insert(self.current_dir.clone(), self.selected);
        }
    }

    /// Returns a reference to the saved positions map.
    pub(crate) fn get_position(&self) -> &HashMap<PathBuf, usize> {
        &self.positions
    }

    /// Sets a new current directory path, clearing entries and selection.
    /// Increments the request ID to cancel pending requests.
    pub(crate) fn set_path(&mut self, path: PathBuf) {
        self.save_position();

        self.current_dir = path;
        // instantly ends all pending messages from the previous directory.
        self.request_id = self.request_id.wrapping_add(1);
    }

    /// Updates the navigation state from a worker thread's result.
    /// Sets the current directory, entries, and selection based on the provided focus.
    pub(crate) fn update_from_worker(
        &mut self,
        path: PathBuf,
        entries: Vec<FileEntry>,
        focus: Option<OsString>,
    ) {
        self.current_dir = path;
        self.entries = entries;

        self.restore_filter_for_current_dir();

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

        if !self.filter.is_empty() && !self.entries.is_empty() {
            let selected_entry_name = self
                .entries
                .get(self.selected)
                .map(|e| e.name().to_os_string());
            if let Some(name) = selected_entry_name {
                let filtered_idx = self
                    .shown_entries()
                    .position(|e| e.name() == name.as_os_str())
                    .unwrap_or(0);
                self.selected = filtered_idx
            } else {
                self.selected = 0;
            }
        }
    }

    /// Toggles the marker state of the currently selected entry.
    /// If the entry is in the clipboard, it is unmarked and removed from the clipboard.
    pub(crate) fn toggle_marker(&mut self, clipboard: &mut Option<HashSet<PathBuf>>) {
        if let Some(entry) = self.selected_shown_entry() {
            let path = self.current_dir().join(entry.name());

            if let Some(clip) = clipboard
                && clip.remove(&path)
            {
                self.markers.insert(path);
                return;
            }
            if !self.markers.remove(&path) {
                self.markers.insert(path);
            }
        }
    }

    /// Toggles the marker state of the currently selected entry and advances the selection.
    pub(crate) fn toggle_marker_advance(
        &mut self,
        clipboard: &mut Option<HashSet<PathBuf>>,
        jump: bool,
    ) {
        self.toggle_marker(clipboard);
        let count = self.shown_entries_len();

        if count == 0 {
            return;
        }

        if self.selected == count - 1 {
            if jump && count > 1 {
                self.selected = 0;
            }
        } else {
            self.selected = self.selected.wrapping_add(1)
        }
    }

    /// Clears all markers.
    pub(crate) fn clear_markers(&mut self) {
        self.markers.clear();
    }

    /// Returns the set of action targets, either marked entries or the selected entry.
    pub(crate) fn get_action_targets(&self) -> HashSet<PathBuf> {
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

    /// Returns an iterator over the entries that match the current filter.
    /// If the filter is empty, returns all entries.
    pub(crate) fn shown_entries(&self) -> Box<dyn Iterator<Item = &FileEntry> + '_> {
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

    /// Returns the number of entries that match the current filter.
    pub(crate) fn shown_entries_len(&self) -> usize {
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

    /// Returns a reference to the currently selected entry that matches the filter.
    pub(crate) fn selected_shown_entry(&self) -> Option<&FileEntry> {
        self.shown_entries().nth(self.selected)
    }

    /// Sets a new filter string, preserving the selected entry if possible.
    pub(crate) fn set_filter(&mut self, filter: String) {
        if self.filter == filter {
            return;
        }

        let target_name = self.selected_shown_entry().map(|e| e.name().to_os_string());
        self.filter = filter;
        self.save_filter_for_current_dir();

        let new_idx = if let Some(ref name) = target_name {
            self.shown_entries()
                .position(|e| e.name() == name.as_os_str())
        } else {
            None
        };

        self.selected = new_idx.unwrap_or(0);
    }

    /// Clears the current filter.
    pub(crate) fn clear_filters(&mut self) {
        self.filter.clear();
        self.save_filter_for_current_dir();
    }

    /// Saves the current filter for the current directory.
    fn save_filter_for_current_dir(&mut self) {
        if self.filter.is_empty() {
            self.filters.remove(&self.current_dir);
        } else {
            self.filters
                .insert(self.current_dir.clone(), self.filter.clone());
        }
    }

    /// Restores the saved filter for the current directory, if any.
    fn restore_filter_for_current_dir(&mut self) {
        self.filter = self
            .filters
            .get(&self.current_dir)
            .cloned()
            .unwrap_or_default();
    }
}

/// Integration tests for navigation
#[cfg(test)]
mod tests {
    use super::*;

    use crate::core::browse_dir;

    use rand::rng;
    use rand::seq::SliceRandom;
    use std::collections::HashSet;
    use std::error;
    use std::fs;
    use std::fs::File;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_navstate_rapid_navigation() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let file_count = 10;

        for i in 0..file_count {
            let file_path = dir.path().join(format!("testfile_{i}.txt"));
            File::create(&file_path)?;
        }

        let entries = browse_dir(dir.path())?;
        assert!(!entries.is_empty(), "sandbox should not be empty");

        let mut nav = NavState::new(dir.path().to_path_buf());
        nav.update_from_worker(dir.path().to_path_buf(), entries.clone(), None);

        assert_eq!(
            nav.entries().len(),
            file_count,
            "initial entry count mismatch"
        );

        let down_presses = 1000;
        for _ in 0..down_presses {
            assert!(nav.move_down(), "nav.move_down() failed during stress");
        }

        let expected_idx = down_presses % file_count;
        assert_eq!(
            nav.selected_idx(),
            expected_idx,
            "wrong index after DOWN stress"
        );

        let selected = nav.selected_entry().ok_or("no entry selected after DOWN")?;
        assert_eq!(selected.name_str(), entries[expected_idx].name_str());

        let up_presses = 1000;
        for _ in 0..up_presses {
            assert!(nav.move_up(), "nav.move_up() failed during stress");
        }

        // Mathematical wrap-around check
        let expected_idx_up = (expected_idx + file_count - (up_presses % file_count)) % file_count;
        assert_eq!(
            nav.selected_idx(),
            expected_idx_up,
            "wrong index after UP stress"
        );

        let selected_up = nav.selected_entry().ok_or("no entry selected after UP")?;
        assert_eq!(selected_up.name_str(), entries[expected_idx_up].name_str());

        // Ensure the internal state hasn't corrupted the entry list
        for (i, entry) in nav.entries().iter().enumerate() {
            assert_eq!(
                entry.name_str(),
                entries[i].name_str(),
                "data corruption at index {i}"
            );
        }
        Ok(())
    }

    #[test]
    fn test_navstate_navigation() -> Result<(), Box<dyn error::Error>> {
        let base = tempdir()?;
        let base_path = base.path().to_path_buf();
        let subdir_path = base_path.join("subdir");
        let subsubdir_path = subdir_path.join("subsub");

        fs::create_dir(&subdir_path)?;
        fs::create_dir(&subsubdir_path)?;
        File::create(base_path.join("file_base.txt"))?;
        File::create(subdir_path.join("file_sub.txt"))?;
        File::create(subsubdir_path.join("file_subsub.txt"))?;

        let base_entries = browse_dir(&base_path)?;
        let sub_entries = browse_dir(&subdir_path)?;
        let subsub_entries = browse_dir(&subsubdir_path)?;

        let mut nav = NavState::new(base_path.clone());
        let repetitions = 500;

        for i in 0..repetitions {
            nav.set_path(subdir_path.clone());
            nav.update_from_worker(subdir_path.clone(), sub_entries.clone(), None);

            assert_eq!(nav.current_dir(), &subdir_path);
            assert!(
                nav.entries().iter().any(|e| e.name() == "subsub"),
                "Iter {i} missing subsub"
            );

            let parent_path = nav.current_dir().parent().ok_or("No parent dir")?;
            assert_eq!(parent_path, base_path, "Iter {i} parent mismatch");

            nav.set_path(subsubdir_path.clone());
            nav.update_from_worker(subsubdir_path.clone(), subsub_entries.clone(), None);

            assert_eq!(nav.current_dir(), &subsubdir_path);
            assert!(nav.entries().iter().any(|e| e.name() == "file_subsub.txt"));

            nav.set_path(subdir_path.clone());
            nav.update_from_worker(subdir_path.clone(), sub_entries.clone(), None);
            assert_eq!(nav.current_dir(), &subdir_path);

            nav.set_path(base_path.clone());
            nav.update_from_worker(base_path.clone(), base_entries.clone(), None);

            assert_eq!(nav.current_dir(), &base_path);
            assert!(nav.entries().iter().any(|e| e.name() == "subdir"));
        }
        Ok(())
    }

    #[test]
    fn test_navstate_selection_persistence() -> Result<(), Box<dyn error::Error>> {
        let base = tempdir()?;
        let base_path = base.path().to_path_buf();
        let subdir_path = base_path.join("subdir");

        fs::create_dir_all(&subdir_path)?;
        for i in 0..20 {
            File::create(subdir_path.join(format!("file_{}.txt", i)))?;
        }

        let base_entries = browse_dir(&base_path)?;
        let sub_entries = browse_dir(&subdir_path)?;

        let mut nav = NavState::new(base_path.clone());
        let repetitions = 200;

        nav.set_path(subdir_path.clone());
        nav.update_from_worker(subdir_path.clone(), sub_entries.clone(), None);

        for _ in 0..5 {
            nav.move_down();
        }
        assert_eq!(nav.selected_idx(), 5, "Initial move failed");

        for i in 0..repetitions {
            nav.set_path(base_path.clone());
            nav.update_from_worker(base_path.clone(), base_entries.clone(), None);

            nav.move_down();

            // Return to Subdir
            nav.set_path(subdir_path.clone());
            nav.update_from_worker(subdir_path.clone(), sub_entries.clone(), None);

            assert_eq!(
                nav.selected_idx(),
                5,
                "Selection lost at iteration {}. Should have stayed at 5.",
                i
            );
        }
        Ok(())
    }

    #[test]
    fn test_navstate_filter_persistence() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let base_path = dir.path().to_path_buf();

        let names = vec![
            "main.rs",
            "lib.rs",
            "cargo.toml",
            "readme.md",
            "app.rs",
            "ui.rs",
            "file_manager.rs",
            "config.json",
            "styles.css",
        ];
        for name in &names {
            fs::write(base_path.join(name), "")?;
        }

        let mut entries = browse_dir(&base_path)?;
        entries.shuffle(&mut rng());

        let mut nav = NavState::new(base_path.clone());
        nav.update_from_worker(base_path.clone(), entries, None);

        let target_name = "file_manager.rs";

        let actual_start_pos = nav
            .shown_entries()
            .position(|e| e.name_str() == target_name)
            .ok_or("Target not found in nav state")?;

        for _ in 0..actual_start_pos {
            nav.move_down();
        }

        let selected = nav.selected_entry().ok_or("No entry selected")?;
        assert_eq!(selected.name_str(), target_name);

        let input_buffer = "file".to_string();
        nav.set_filter(input_buffer);

        let final_entry = nav.selected_entry().ok_or("Selection lost after filter")?;

        assert_eq!(
            final_entry.name_str(),
            target_name,
            "Selection persistence failed! Found {} instead. Filter was 'file'.",
            final_entry.name_str()
        );
        Ok(())
    }

    #[test]
    fn test_navstate_marker_persistence() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let base_path = dir.path().to_path_buf();

        let names = vec!["apple.txt", "banana.txt", "crab.txt"];
        for name in &names {
            fs::write(base_path.join(name), "")?;
        }

        let mut entries = browse_dir(&base_path)?;
        // Shuffle to ensure we arent relying on alphabetical order
        entries.shuffle(&mut rng());

        let mut nav = NavState::new(base_path.clone());
        nav.update_from_worker(base_path.clone(), entries, None);

        let mut clipboard: Option<HashSet<PathBuf>> = None;

        let to_mark = vec!["apple.txt", "banana.txt"];
        for target in to_mark {
            // Find it in the current view
            let pos = nav
                .shown_entries()
                .position(|e| e.name_str() == target)
                .ok_or("target not found")?;

            // Reset to top and move down to simulate real user navigation
            while nav.selected_idx() > 0 {
                nav.move_up();
            }
            for _ in 0..pos {
                nav.move_down();
            }

            let selected = nav.selected_entry().ok_or("No entry selected to mark")?;
            assert_eq!(selected.name_str(), target);
            nav.toggle_marker(&mut clipboard);
        }

        assert_eq!(nav.markers().len(), 2);

        nav.set_filter("crab".to_string());

        assert_eq!(nav.shown_entries_len(), 1);
        let crab_selected = nav.selected_entry().ok_or("No crab selected")?;
        assert_eq!(crab_selected.name_str(), "crab.txt");

        let targets = nav.get_action_targets();
        assert_eq!(
            targets.len(),
            2,
            "Should target 2 marked files even if hidden"
        );
        assert!(targets.contains(&base_path.join("apple.txt")));
        assert!(targets.contains(&base_path.join("banana.txt")));
        assert!(
            !targets.contains(&base_path.join("cherry.txt")),
            "Should ignore selection when markers exist"
        );

        nav.clear_filters();
        // Navigate to apple.txt
        let apple_pos = nav
            .shown_entries()
            .position(|e| e.name_str() == "apple.txt")
            .ok_or("Apple not found")?;
        while nav.selected_idx() < apple_pos {
            nav.move_down();
        }
        while nav.selected_idx() > apple_pos {
            nav.move_up();
        }

        nav.toggle_marker(&mut clipboard);
        assert_eq!(nav.markers().len(), 1);
        assert!(nav.markers().contains(&base_path.join("banana.txt")));
        Ok(())
    }
}
