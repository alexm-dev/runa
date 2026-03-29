//! Navigation actions handler for AppState
//!
//! This module contains the implementation of navigation-related actions for the AppState.
//!
//! It includes handling of actions such as moving up/down the file list,
//! navigating into directories, going to the parent directory,
//! toggling markers, clearing filters, and more.
//!
//! The functions in this module are responsible for updating
//! the navigation state, requesting previews, and managing the file information cache
//! as needed based on user interactions.

use crate::app::keymap::NavAction;
use crate::app::state::{AppState, KeypressResult};
use crate::app::{Clipboard, NavState, Workers};
use crate::core::formatter::format_display_path;
use crate::utils::{clean_display_path, expand_home_path_buf, get_home};

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

impl<'a> AppState<'a> {
    /// Handles navigation actions (up, down, into dir, etc).
    /// Returns a [KeypressResult] indicating how the action was handled.
    pub(in crate::app) fn handle_nav_action(
        &mut self,
        workers: &Workers,
        action: NavAction,
        clipboard: &mut Clipboard,
    ) -> KeypressResult {
        match action {
            NavAction::GoUp => {
                self.move_nav_if_possible(workers, |nav| nav.move_up());
                self.refresh_show_info_if_open();
            }
            NavAction::GoDown => {
                self.move_nav_if_possible(workers, |nav| nav.move_down());
                self.refresh_show_info_if_open();
            }
            NavAction::GoParent => {
                let res = self.handle_go_parent(workers);
                self.refresh_show_info_if_open();
                return res;
            }
            NavAction::GoIntoDir => {
                let res = self.handle_go_into_dir(workers);
                self.refresh_show_info_if_open();
                return res;
            }
            NavAction::ToggleMarker => {
                let marker_jump = self.config.display().toggle_marker_jump();
                self.nav
                    .toggle_marker_advance(&mut clipboard.entries, marker_jump);
                self.request_preview(workers);
                self.update_file_info_cache(workers);
            }
            NavAction::ClearMarker => {
                self.nav.clear_markers();
                self.request_preview(workers);
            }
            NavAction::ClearFilter => {
                self.nav.clear_filters();
                self.request_preview(workers);
            }
            NavAction::ClearAll => {
                self.nav.clear_markers();
                self.nav.clear_filters();
                clipboard.clear();
                self.request_preview(workers);
            }
            NavAction::GoToBottom => {
                self.nav.last_selected();
                self.refresh_show_info_if_open();
                self.update_file_info_cache(workers);
                self.request_preview(workers);
            }
            NavAction::ScrollUp => {
                self.actions.scroll().scroll_up();
            }
            NavAction::ScrollDown => {
                self.actions.scroll().scroll_down();
            }
            NavAction::SelectAll => {
                clipboard.clear();
                self.nav.select_all();
                self.request_preview(workers);
                self.update_file_info_cache(workers);
            }
            _ => {}
        }
        KeypressResult::Continue
    }

    /// Calls the provided function to move navigation if possible.
    ///
    /// If the movement was successful (f returns true), marks the preview as pending refresh.
    /// Used to encapsulate common logic for nav actions that change selection or directory.
    fn move_nav_if_possible<F>(&mut self, workers: &Workers, f: F)
    where
        F: FnOnce(&mut NavState) -> bool,
    {
        if f(&mut self.nav) {
            self.refresh_show_info_if_open();
            self.update_file_info_cache(workers);
            if self.config.display().instant_preview() {
                self.request_preview(workers);
            } else {
                self.preview.mark_pending();
            }
        }
    }

    /// Handles the go to parent directory action.
    ///
    /// If the current directory has a parent, navigates to it, saves the current position,
    /// and requests loading of the new directory and its parent content.
    fn handle_go_parent(&mut self, workers: &Workers) -> KeypressResult {
        let current = self.nav.current_dir();

        let Some(parent) = current.parent() else {
            return KeypressResult::Continue;
        };

        let parent_path = parent.to_path_buf();

        if std::fs::metadata(&parent_path).is_err() {
            self.push_overlay_message(
                "Parent directory is unreachable".to_string(),
                Duration::from_secs(3),
            );
            return KeypressResult::Consumed;
        }

        let exited_name = current.file_name().map(|n| n.to_os_string());
        self.navigate_to(parent_path, exited_name, workers);
        KeypressResult::Continue
    }

    /// Handles the go into directory action.
    ///
    /// If the selected entry is a directory, navigates into it, saves the current position,
    /// and requests loading of the new directory and its parent content.
    fn handle_go_into_dir(&mut self, workers: &Workers) -> KeypressResult {
        let Some(entry) = self.nav.selected_shown_entry() else {
            return KeypressResult::Continue;
        };

        let entry_path = self.nav.current_dir().join(entry.name());

        let Ok(meta) = std::fs::metadata(&entry_path) else {
            return KeypressResult::Continue;
        };

        if !meta.is_dir() {
            return KeypressResult::Continue;
        }

        match std::fs::read_dir(&entry_path) {
            Ok(_) => {
                self.navigate_to(entry_path, None, workers);
                KeypressResult::Continue
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                let msg = format!("Permission Denied: {}", e);
                self.push_overlay_message(msg, std::time::Duration::from_secs(3));
                KeypressResult::Consumed
            }
            Err(_) => KeypressResult::Continue,
        }
    }

    /// Handles the find action.
    ///
    /// If a result is selected in the find results, navigates to its path.
    /// If the path is a directory, navigates into it.
    /// If the path is a file, navigates to its parent directory and focuses on the file.
    pub(super) fn handle_find(&mut self, workers: &Workers) {
        let Some(r) = self
            .actions
            .find_results()
            .get(self.actions.find_selected())
        else {
            return;
        };

        let path = r.path();

        let (target_dir, focus) = if path.is_dir() {
            (path.to_path_buf(), None)
        } else {
            let Some(parent) = path.parent() else {
                return;
            };
            let file_name = path.file_name().map(|n| n.to_os_string());
            (parent.to_path_buf(), file_name)
        };

        if let Err(e) = std::fs::read_dir(&target_dir) {
            let msg = format!("Access Denied: {}", e);
            self.push_overlay_message(msg, Duration::from_secs(3));
            return;
        }
        self.navigate_to(target_dir, focus, workers);
        self.exit_input_mode();
    }

    /// Handles the move action
    ///
    /// Checks if the directory for files to be moved to exists
    /// Also normalizes relative paths for easier moving of files.
    pub(super) fn handle_move(&mut self, workers: &Workers) {
        let dest_dir = self.actions.input_buffer();
        if dest_dir.trim().is_empty() {
            self.push_overlay_message(
                "Move failed: target directory cannot be empty".to_string(),
                Duration::from_secs(3),
            );
            return;
        }

        let input_path = expand_home_path_buf(dest_dir.trim());
        let resolved_path = if input_path.is_absolute() {
            input_path
        } else {
            self.nav.current_dir().join(input_path)
        };

        let absolute_dest = match resolved_path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                let norm_msg = format_display_path(&resolved_path);
                self.push_overlay_message(
                    format!("Move failed: {}: {}", e, norm_msg),
                    Duration::from_secs(3),
                );
                return;
            }
        };

        if !absolute_dest.is_dir() {
            self.push_overlay_message(
                "Move failed: not a directory".into(),
                Duration::from_secs(3),
            );
            return;
        }

        if let Err(e) = std::fs::read_dir(&absolute_dest) {
            let norm_msg = format_display_path(&absolute_dest);
            self.push_overlay_message(
                format!("Move failed: Permission denied in {}: {}", norm_msg, e),
                Duration::from_secs(3),
            );
            return;
        }

        if !absolute_dest.is_dir() {
            let norm_msg = format_display_path(&absolute_dest);
            self.push_overlay_message(
                format!("Move failed: not a directory: {}", norm_msg),
                Duration::from_secs(3),
            );
            return;
        }

        let targets = self.nav.get_action_targets();
        for src in &targets {
            if let Ok(absolute_src) = src.canonicalize()
                && absolute_dest.starts_with(&absolute_src)
            {
                let msg = if absolute_dest == absolute_src {
                    "Move failed: source and destination are the same".to_string()
                } else {
                    let normalized = format_display_path(&absolute_src);
                    let display_path = clean_display_path(&normalized);
                    format!(
                        "Move failed: cannot move directory into its own sub directory: {}",
                        display_path
                    )
                };
                self.push_overlay_message(msg, Duration::from_secs(3));
                return;
            }
        }

        let fileop_tx = workers.fileop_tx();
        let move_msg = format!(
            "Files moved to: {}",
            clean_display_path(&absolute_dest.to_string_lossy())
        );

        self.actions
            .actions_move(&mut self.nav, absolute_dest, fileop_tx);

        self.exit_input_mode();
        self.push_overlay_message(move_msg, Duration::from_secs(3));
    }

    pub(super) fn handle_go_to_home(&mut self, workers: &Workers) {
        if let Some(home_path) = get_home() {
            self.navigate_to(home_path.clone(), None, workers);
        }
    }

    pub(super) fn handle_go_to_path(&mut self, workers: &Workers) {
        let path = self.actions.input_buffer();
        if path.trim().is_empty() {
            self.push_overlay_message("Error: No path entered".to_string(), Duration::from_secs(3));
            return;
        }

        let expaned = expand_home_path_buf(path);
        let abs_path = if expaned.is_absolute() {
            expaned
        } else {
            self.nav.current_dir().join(expaned)
        };

        if let Ok(meta) = std::fs::metadata(&abs_path) {
            if meta.is_dir() {
                self.navigate_to(abs_path.clone(), None, workers);
            } else {
                self.push_overlay_message(
                    "Error: Not a directory".to_string(),
                    Duration::from_secs(3),
                );
            }
        } else {
            self.push_overlay_message("Error: Invalid path".to_string(), Duration::from_secs(3));
        }
    }

    pub(super) fn handle_go_to_top(&mut self, workers: &Workers) {
        self.nav.first_selected();
        self.request_preview(workers);
    }

    pub(super) fn navigate_to(
        &mut self,
        path: PathBuf,
        focus: Option<OsString>,
        workers: &Workers,
    ) {
        self.nav.save_position();
        self.nav.set_path(path);
        self.request_dir_load(workers, focus);
        self.request_parent_content(workers);
    }
}
