use crate::file_manager::FileEntry;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub struct NavState {
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    selected: usize,
    positions: HashMap<PathBuf, usize>,
    markers: HashSet<PathBuf>,
    filter: String,
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
            request_id: 0,
        }
    }

    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }
    pub fn markers(&self) -> &HashSet<PathBuf> {
        &self.markers
    }
    pub fn filer(&self) -> &str {
        &self.filter
    }

    pub fn selected_idx(&self) -> usize {
        self.selected
    }
    pub fn request_id(&self) -> u64 {
        self.request_id
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected)
    }

    pub fn prepare_new_request(&mut self) -> u64 {
        self.request_id += 1;
        self.request_id
    }

    pub fn move_up(&mut self) -> bool {
        let len = self.entries.len();
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
        let len = self.entries.len();
        if len == 0 {
            return false;
        }

        self.selected = (self.selected + 1) % len;
        true
    }

    pub fn save_position(&mut self) {
        self.positions
            .insert(self.current_dir.clone(), self.selected);
    }

    pub fn get_position(&self) -> &HashMap<PathBuf, usize> {
        &self.positions
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.current_dir = path;
        self.entries.clear();
        self.selected = 0;
        // instantly kills all pending messages from the previous directory.
        self.request_id += 1;
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
        if let Some(entry) = self.selected_entry() {
            let path = self.current_dir().join(entry.name());
            if !self.markers.remove(&path) {
                self.markers.insert(path);
            }
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

    pub fn filtered_entries(&self) -> Vec<&FileEntry> {
        if self.filter.is_empty() {
            self.entries.iter().collect()
        } else {
            let filter_lower = self.filter.to_lowercase();
            self.entries
                .iter()
                .filter(|e| {
                    e.name()
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&filter_lower)
                })
                .collect()
        }
    }

    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
        self.selected = 0;
    }
}
