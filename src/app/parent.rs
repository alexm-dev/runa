use crate::file_manager::FileEntry;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct ParentState {
    entries: Vec<FileEntry>,
    selected_idx: Option<usize>,
    last_path: Option<PathBuf>,
    request_id: u64,
}

impl ParentState {
    pub fn request_id(&self) -> u64 {
        self.request_id
    }
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }
    pub fn selected_idx(&self) -> Option<usize> {
        self.selected_idx
    }

    pub fn should_request(&self, parent_path: &Path) -> bool {
        if self.entries.is_empty() {
            return true;
        }
        Some(parent_path) != self.last_path.as_deref()
    }

    pub fn prepare_new_request(&mut self, path: PathBuf) -> u64 {
        self.request_id += 1;
        self.last_path = Some(path);
        self.request_id
    }

    pub fn update_from_entries(
        &mut self,
        entries: Vec<FileEntry>,
        current_name: &str,
        req_id: u64,
    ) {
        if req_id < self.request_id {
            return;
        }
        // Find the index of the folder we are currently inside to highlight it
        self.selected_idx = entries.iter().position(|e| e.name_str() == current_name);
        self.entries = entries;
        self.request_id = req_id;
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.selected_idx = None;
        self.last_path = None;
        self.request_id += 1;
    }
}
