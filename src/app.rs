use crate::config::Config;
use crate::file_manager::{FileEntry, browse_dir};
use crate::formatter::Formatter;
use crate::keymap::{Action, Keymap};
use crate::utils::open_in_editor;
use std::collections::HashMap;

pub enum KeypressResult {
    Continue,
    Quit,
    OpenedEditor,
}

/// Application state of the file browser
///
/// Stores the directory entires, the current selection, the directory positions
/// and the configuration as a reference.
pub struct AppState<'a> {
    current_dir: std::path::PathBuf,
    entries: Vec<FileEntry>,
    selected: usize,
    config: &'a Config,
    dir_positions: HashMap<std::path::PathBuf, usize>,
    keymap: Keymap,
}

impl<'a> AppState<'a> {
    pub fn new(config: &'a Config) -> std::io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let mut entries = browse_dir(&current_dir)?;

        let formatter = Formatter::new(
            config.dirs_first(),
            config.show_hidden(),
            config.show_system(),
            config.case_insensitive(),
        );

        formatter.filter_entries(&mut entries);

        Ok(Self {
            current_dir,
            entries,
            selected: 0,
            config,
            dir_positions: HashMap::new(),
            keymap: Keymap::from_config(config),
        })
    }

    pub fn visible_entries(&self) -> &[FileEntry] {
        &self.entries
    }

    pub fn has_visible_entries(&self) -> bool {
        !self.entries.is_empty()
    }

    // pub fn selected_entry(&self) -> Option<&FileEntry> {
    //     self.entries.get(self.selected)
    // }

    pub fn visible_selected(&self) -> Option<usize> {
        if self.entries.is_empty() {
            None
        } else {
            Some(self.selected)
        }
    }

    pub fn config(&self) -> &Config {
        self.config
    }

    fn save_current_pos(&mut self) {
        self.dir_positions
            .insert(self.current_dir.clone(), self.selected);
    }

    pub fn handle_keypress(&mut self, key: &str) -> KeypressResult {
        match self.keymap.lookup(key) {
            Some(Action::GoParent) => self.handle_go_parent(),
            Some(Action::GoIntoDir) => self.handle_go_into_dir(),
            Some(Action::GoUp) => self.handle_go_up(),
            Some(Action::GoDown) => self.handle_go_down(),
            Some(Action::Open) => self.handle_open_file(),
            Some(Action::Quit) => self.handle_quit(),
            _ => KeypressResult::Continue,
        }
    }

    fn handle_go_parent(&mut self) -> KeypressResult {
        if let Some(parent) = self.current_dir.parent() {
            let parent_path = parent.to_path_buf();
            let exited_dir_name = self.current_dir.file_name().map(|n| n.to_os_string());
            self.save_current_pos();
            self.current_dir = parent_path;
            self.reload_entries(exited_dir_name);
        }
        KeypressResult::Continue
    }

    fn handle_go_into_dir(&mut self) -> KeypressResult {
        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir() {
                let dir_name = entry.name().clone();
                self.save_current_pos();
                self.current_dir = self.current_dir.join(&dir_name);
                self.reload_entries(None);
            }
        }
        KeypressResult::Continue
    }

    fn handle_go_up(&mut self) -> KeypressResult {
        if self.selected > 0 {
            self.selected -= 1;
        }
        KeypressResult::Continue
    }

    fn handle_go_down(&mut self) -> KeypressResult {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
        KeypressResult::Continue
    }

    fn handle_open_file(&mut self) -> KeypressResult {
        if let Some(entry) = self.entries.get(self.selected) {
            let path = self.current_dir.join(entry.name());
            if let Err(e) = open_in_editor(self.config.editor(), &path) {
                eprintln!("Error opening editor: {}", e);
            }
            return KeypressResult::OpenedEditor;
        }
        KeypressResult::Continue
    }

    fn handle_quit(&self) -> KeypressResult {
        KeypressResult::Quit
    }

    fn reload_entries(&mut self, focus_target: Option<std::ffi::OsString>) {
        if let Ok(mut entries) = browse_dir(&self.current_dir) {
            let formatter = Formatter::new(
                self.config.dirs_first(),
                self.config.show_hidden(),
                self.config.show_system(),
                self.config.case_insensitive(),
            );
            formatter.filter_entries(&mut entries);

            let next_selected = if let Some(target_name) = focus_target {
                entries
                    .iter()
                    .position(|e| e.name() == target_name.as_os_str())
                    .unwrap_or(0)
            } else if let Some(saved_idx) = self.dir_positions.get(&self.current_dir) {
                (*saved_idx).min(entries.len().saturating_sub(1))
            } else {
                0
            };
            self.entries = entries;
            self.selected = next_selected;
        }
    }
}
