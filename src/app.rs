use crate::config::Config;
use crate::file_manager::{FileEntry, browse_dir};
use crate::formatter::Formatter;
use crate::utils::open_in_editor;

/// Application state of the file browser
///
/// Stores the directory entires, the current selection
/// and the configuration as a reference.
pub struct AppState<'a> {
    pub current_dir: std::path::PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected: usize,
    pub config: &'a Config,
}

impl<'a> AppState<'a> {
    pub fn new(config: &'a Config) -> std::io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let mut entries = browse_dir(&current_dir)?;

        let formatter = Formatter::new(
            config.dirs_first,
            config.show_hidden,
            config.case_insensitive,
        );

        formatter.filter_hidden(&mut entries);

        Ok(Self {
            current_dir,
            entries,
            selected: 0,
            config,
        })
    }

    /// Handles a keypress
    ///
    /// Returns false if the application should exit
    pub fn handle_keypress(&mut self, key: &str) -> bool {
        if self.handle_navigation(key) {
            return true;
        }
        if self.handle_traverse(key) {
            return true;
        }
        if self.handle_open(key) {
            return true;
        }
        if self.handle_quit(key) {
            return false;
        }
        true
    }

    fn handle_navigation(&mut self, key: &str) -> bool {
        if self.config.keys.go_up.iter().any(|k| k == key) {
            if self.selected > 0 {
                self.selected -= 1;
            }
            return true;
        }
        if self.config.keys.go_down.iter().any(|k| k == key) {
            if self.selected + 1 < self.entries.len() {
                self.selected += 1;
            }
            return true;
        }
        false
    }

    fn handle_traverse(&mut self, key: &str) -> bool {
        if self.config.keys.go_parent.iter().any(|k| k == key) {
            if let Some(parent) = self.current_dir.parent() {
                let exited_dir_name = self.current_dir.file_name().map(|n| n.to_os_string());

                self.current_dir = parent.to_path_buf();
                self.reload_entries(exited_dir_name);
            }
            return true;
        }

        if self.config.keys.go_into_dir.iter().any(|k| k == key) {
            if let Some(entry) = self.entries.get(self.selected) {
                if entry.is_dir {
                    self.current_dir = self.current_dir.join(&entry.name);
                    self.reload_entries(None);
                }
            }
            return true;
        }
        false
    }

    fn handle_open(&mut self, key: &str) -> bool {
        if self.config.keys.open_file.iter().any(|k| k == key) {
            if let Some(entry) = self.entries.get(self.selected) {
                if !entry.is_dir {
                    let path = self.current_dir.join(&entry.name);

                    if let Err(e) = open_in_editor(&self.config.editor, &path) {
                        eprintln!("Error opening editor: {}", e);
                    }
                    return true;
                }
            }
        }
        false
    }

    fn handle_quit(&self, key: &str) -> bool {
        self.config.keys.quit.iter().any(|k| k == key)
    }

    fn reload_entries(&mut self, focus_target: Option<std::ffi::OsString>) {
        if let Ok(mut entries) = browse_dir(&self.current_dir) {
            let formatter = Formatter::new(
                self.config.dirs_first,
                self.config.show_hidden,
                self.config.case_insensitive,
            );
            formatter.filter_hidden(&mut entries);
            self.entries = entries;

            if let Some(target_name) = focus_target {
                self.selected = self
                    .entries
                    .iter()
                    .position(|e| e.name == target_name)
                    .unwrap_or(0);
            } else {
                self.selected = 0;
            }
        }
    }
}
