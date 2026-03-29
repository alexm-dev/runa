//! File action handlers for AppState.
//!
//! Defines [handle_file_action] method which dispatches file actions
//! to the central [app::state::handle_keypress].
//!
//! Also defines handlers for each file action (open, delete, copy, etc)
//! which perform the necessary state updates
//! and call the appropriate methods in the actions module.
//!
//! This module is responsible for connecting file actions
//! to the underlying logic that performs those actions,

use crate::app::actions::{ActionMode, InputMode};
use crate::app::keymap::FileAction;
use crate::app::state::{AppState, KeypressResult};
use crate::app::{Clipboard, Workers};
use crate::utils::open_in_editor;

use std::sync::Arc;
use std::time::Duration;

/// AppState file action handlers
impl<'a> AppState<'a> {
    /// Handles file actions (open, delete, copy, etc).
    /// Returns a [KeypressResult] indicating how the action was handled.
    pub(in crate::app) fn handle_file_action(
        &mut self,
        workers: &Workers,
        action: FileAction,
        clipboard: &mut Clipboard,
    ) -> KeypressResult {
        match action {
            FileAction::Open => return self.handle_open_file(workers),
            FileAction::Delete => {
                let is_trash = self.config.general().move_to_trash();
                self.prompt_delete(is_trash);
            }
            FileAction::AlternateDelete => {
                let is_trash = !self.config.general().move_to_trash();
                self.prompt_delete(is_trash);
            }
            FileAction::Copy => {
                let nav = &mut self.nav;
                self.actions.action_copy(nav, clipboard, false);
                self.handle_timed_message(Duration::from_secs(15));
            }
            FileAction::Paste => {
                let fileop_tx = workers.fileop_tx();
                self.actions
                    .action_paste(&mut self.nav, clipboard, fileop_tx);
            }
            FileAction::Rename => self.prompt_rename(),
            FileAction::Create => self.prompt_create_file(),
            FileAction::CreateDirectory => self.prompt_create_folder(),
            FileAction::Filter => self.prompt_filter(),
            FileAction::ShowInfo => self.toggle_file_info(),
            FileAction::Find => self.prompt_find(),
            FileAction::MoveFile => self.prompt_move(),
            FileAction::ClearClipboard => {
                clipboard.clear();
                self.request_preview(workers);
            }
        }
        KeypressResult::Continue
    }

    /// Handles the open file action.
    ///
    /// If a file is selected, attempts to open it in the configured editor.
    /// If an error occurs, prints it to stderr.
    fn handle_open_file(&mut self, workers: &Workers) -> KeypressResult {
        let editor = self.config.editor();
        if !editor.exists() {
            let msg = format!("Editor '{}' not found", editor.cmd());
            self.push_overlay_message(msg, Duration::from_secs(3));
            return KeypressResult::Continue;
        }
        if let Some(entry) = self.nav.selected_entry() {
            let path = self.nav.current_dir().join(entry.name());
            match open_in_editor(self.config.editor(), &path) {
                Ok(_) => {
                    self.request_preview(workers);
                    KeypressResult::OpenedEditor
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    self.push_overlay_message(error_msg, Duration::from_secs(3));
                    KeypressResult::Recovered
                }
            }
        } else {
            KeypressResult::Continue
        }
    }

    /// Creates a new file with the name in the input buffer.
    /// Calls actions::action_create with `is_folder` set to false.
    pub(super) fn create_file(&mut self, workers: &Workers) {
        if self.actions.input_buffer().is_empty() {
            return;
        }

        let target = self.nav.current_dir().join(self.actions.input_buffer());
        if target.exists() {
            let name = target
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let prompt_text = format!("Overwrite file '{}' ? [Y/n]", name);
            self.enter_input_mode(
                InputMode::ConfirmOverwrite {
                    is_dir: false,
                    old: None,
                    new: Arc::new(target),
                },
                prompt_text,
                None,
            );
            return;
        }

        let fileop_tx = workers.fileop_tx();
        self.actions.action_create(&mut self.nav, false, fileop_tx);
    }

    /// Creates a new folder with the name in the input buffer.
    /// Calls actions::action_create with `is_folder` set to true.
    pub(super) fn create_folder(&mut self, workers: &Workers) {
        if self.actions.input_buffer().is_empty() {
            return;
        }

        let target = self.nav.current_dir().join(self.actions.input_buffer());
        if target.exists() {
            let name = target
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let prompt_text = format!("Overwrite directory '{}' ? [Y/n]", name);
            self.enter_input_mode(
                InputMode::ConfirmOverwrite {
                    is_dir: true,
                    old: None,
                    new: Arc::new(target),
                },
                prompt_text,
                None,
            );
            return;
        }

        let fileop_tx = workers.fileop_tx();
        self.actions.action_create(&mut self.nav, true, fileop_tx);
    }

    /// Renames the selected entry to the name in the input buffer.
    /// Calls actions::action_rename.
    pub(super) fn rename_entry(&mut self, workers: &Workers) {
        if self.actions.input_buffer().is_empty() {
            return;
        }
        if let Some(entry) = self.nav.selected_entry() {
            let old_path = self.nav.current_dir().join(entry.name());
            let new_path = old_path.with_file_name(self.actions.input_buffer());

            if old_path == new_path {
                return;
            }

            let is_case_rename = old_path.to_string_lossy().to_lowercase()
                == new_path.to_string_lossy().to_lowercase();

            if new_path.exists() && !is_case_rename {
                let name = new_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let prompt_text = format!("Overwrite '{}' ? [Y/n]", name);
                self.enter_input_mode(
                    InputMode::ConfirmOverwrite {
                        is_dir: new_path.is_dir(),
                        old: Some(Arc::new(old_path)),
                        new: Arc::new(new_path),
                    },
                    prompt_text,
                    None,
                );
                return;
            }

            let fileop_tx = workers.fileop_tx();
            self.actions.action_rename(&mut self.nav, fileop_tx);
        }
    }

    /// Applies the filter in the input buffer to the navigation state.
    /// Calls actions::action_filter and requests a preview refresh.
    pub(super) fn apply_filter(&mut self, workers: &Workers) {
        self.actions.action_filter(&mut self.nav);
        self.request_preview(workers);
    }

    /// Confirms deletion of the selected items.
    /// Calls actions::action_delete.
    pub(super) fn confirm_delete(&mut self, workers: &Workers) {
        let move_to_trash = if let ActionMode::Input {
            mode: InputMode::ConfirmDelete { is_trash },
            ..
        } = self.actions.mode()
        {
            *is_trash
        } else {
            self.config.general().move_to_trash()
        };

        let fileop_tx = workers.fileop_tx();

        self.actions
            .action_delete(&mut self.nav, fileop_tx, move_to_trash);
    }

    pub(super) fn confirm_overwrite(&mut self, workers: &Workers) {
        if let ActionMode::Input {
            mode: InputMode::ConfirmOverwrite { is_dir, old, new },
            ..
        } = self.actions.mode()
        {
            let fileop_tx = workers.fileop_tx();
            if let Some(old_arc) = old {
                self.actions
                    .action_rename_overwrite(old_arc.clone(), new.clone(), fileop_tx);
            } else {
                self.actions
                    .action_create_overwrite(new.clone(), *is_dir, fileop_tx);
            }
        }
    }
}
