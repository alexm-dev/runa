use crate::app::keymap::FileAction;
use crate::app::state::{AppState, KeypressResult};
use crate::app::{Clipboard, Workers};
use crate::utils::open_in_editor;

use std::time::Duration;

/// AppState input and action handlers
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
                self.actions.action_clear_clipboard(clipboard);
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
        if let Some(entry) = self.nav.selected_shown_entry() {
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
}
