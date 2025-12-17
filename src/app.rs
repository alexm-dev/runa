use crate::config::Config;
use crate::file_manager::{FileEntry, browse_dir};
use crate::formatter::Formatter;
use crate::utils::open_in_editor;
use std::collections::HashMap;

pub enum KeypressResult {
    Continue,
    Quit,
    OpenedEditor,
}

/// Action enum used for the keymap hashmap for easier reading of keypress handling
/// Will be extened upon
#[derive(Copy, Clone)]
enum Action {
    GoParent,
    GoIntoDir,
    GoUp,
    GoDown,
    Open,
    Quit,
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
    keymap: HashMap<String, Action>,
}

impl<'a> AppState<'a> {
    pub fn new(config: &'a Config) -> std::io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let mut entries = browse_dir(&current_dir)?;

        let formatter = Formatter::new(
            config.dirs_first,
            config.show_hidden,
            config.show_system,
            config.case_insensitive,
        );

        formatter.filter_entries(&mut entries);

        Ok(Self {
            current_dir,
            entries,
            selected: 0,
            config,
            dir_positions: HashMap::new(),
            keymap: Self::build_keymap(config),
        })
    }

    pub fn entries(&self) -> &Vec<FileEntry> {
        &self.entries
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn config(&self) -> &Config {
        self.config
    }

    // pub fn dir_positions(&self) -> &HashMap<std::path::PathBuf, usize> {
    //     &self.dir_positions
    // }
    //
    // pub fn keymap(&self) -> &HashMap<String, Action> {
    //     &self.keymap
    // }

    fn build_keymap(config: &Config) -> HashMap<String, Action> {
        let mut map = HashMap::new();
        for key in &config.keys.go_parent {
            map.insert(key.clone(), Action::GoParent);
        }
        for key in &config.keys.go_into_dir {
            map.insert(key.clone(), Action::GoIntoDir);
        }
        for key in &config.keys.go_up {
            map.insert(key.clone(), Action::GoUp);
        }
        for key in &config.keys.go_down {
            map.insert(key.clone(), Action::GoDown);
        }
        for key in &config.keys.open_file {
            map.insert(key.clone(), Action::Open);
        }
        for key in &config.keys.quit {
            map.insert(key.clone(), Action::Quit);
        }
        map
    }

    fn save_current_pos(&mut self) {
        self.dir_positions
            .insert(self.current_dir.clone(), self.selected);
    }

    pub fn handle_keypress(&mut self, key: &str) -> KeypressResult {
        if let Some(action) = self.keymap.get(key) {
            match action {
                Action::GoParent => self.handle_go_parent(),
                Action::GoIntoDir => self.handle_go_into_dir(),
                Action::GoUp => self.handle_go_up(),
                Action::GoDown => self.handle_go_down(),
                Action::Open => self.handle_open_file(),
                Action::Quit => self.handle_quit(),
            }
        } else {
            KeypressResult::Continue
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
            let path = self.current_dir.join(&entry.name());
            if let Err(e) = open_in_editor(&self.config.editor, &path) {
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
                self.config.dirs_first,
                self.config.show_hidden,
                self.config.show_system,
                self.config.case_insensitive,
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
