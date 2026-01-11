//! Input action handler methods for runa.
//!
//! This module implements [AppState] methods that process key events, file/nav actions,
//! and input modes (rename, filter, etc).

use crate::app::NavState;
use crate::app::actions::{ActionMode, InputMode};
use crate::app::keymap::{FileAction, NavAction};
use crate::app::state::{AppState, KeypressResult};
use crate::core::FileInfo;
use crate::ui::overlays::Overlay;

use crossterm::event::{KeyCode::*, KeyEvent};
use std::time::{Duration, Instant};

/// AppState input and action handlers
impl<'a> AppState<'a> {
    // AppState core handlers

    /// Handles key events when in an input mode (rename, filter, etc).
    /// Returns a [KeypressResult] indicating how the key event was handled.
    ///
    /// If not in an input mode, returns [KeypressResult::Continue].
    /// Consumes keys related to input editing and mode confirmation/cancellation.
    pub fn handle_input_mode(&mut self, key: KeyEvent) -> KeypressResult {
        let mode = if let ActionMode::Input { mode, .. } = &self.actions().mode() {
            *mode
        } else {
            return KeypressResult::Continue;
        };

        match key.code {
            Enter => {
                match mode {
                    InputMode::NewFile => self.create_file(),
                    InputMode::NewFolder => self.create_folder(),
                    InputMode::Rename => self.rename_entry(),
                    InputMode::Filter => self.apply_filter(),
                    InputMode::ConfirmDelete => self.confirm_delete(),
                    InputMode::Find => self.handle_find(),
                }
                self.exit_input_mode();
                KeypressResult::Consumed
            }

            Esc => {
                self.exit_input_mode();
                KeypressResult::Consumed
            }

            Left => {
                self.actions.action_move_cursor_left();
                KeypressResult::Consumed
            }

            Up => {
                if matches!(mode, InputMode::Find) {
                    self.actions.find_state_mut().select_prev();
                    KeypressResult::Consumed
                } else {
                    KeypressResult::Continue
                }
            }

            Down => {
                if matches!(mode, InputMode::Find) {
                    self.actions.find_state_mut().select_next();
                    KeypressResult::Consumed
                } else {
                    KeypressResult::Continue
                }
            }

            Right => {
                self.actions.action_move_cursor_right();
                KeypressResult::Consumed
            }

            Home => {
                self.actions.action_cursor_home();
                KeypressResult::Consumed
            }

            End => {
                self.actions.action_cursor_end();
                KeypressResult::Consumed
            }

            Backspace => {
                self.actions.action_backspace_at_cursor();
                if matches!(mode, InputMode::Filter) {
                    self.apply_filter();
                }
                if matches!(mode, InputMode::Find) {
                    self.actions.find_debounce(Duration::from_millis(90));
                }
                KeypressResult::Consumed
            }

            Char(c) => match mode {
                InputMode::ConfirmDelete => {
                    self.process_confirm_delete_char(c);
                    KeypressResult::Consumed
                }
                InputMode::Filter => {
                    self.actions.action_insert_at_cursor(c);
                    self.apply_filter();
                    KeypressResult::Consumed
                }
                InputMode::Rename | InputMode::NewFile | InputMode::NewFolder => {
                    self.actions.action_insert_at_cursor(c);
                    KeypressResult::Consumed
                }
                InputMode::Find => {
                    self.actions.action_insert_at_cursor(c);
                    self.actions.find_debounce(Duration::from_millis(120));
                    KeypressResult::Consumed
                }
            },

            _ => KeypressResult::Consumed,
        }
    }

    /// Handles navigation actions (up, down, into dir, etc).
    /// Returns a [KeypressResult] indicating how the action was handled.
    ///
    /// # Arguments
    /// * `action` - The navigation action to handle.
    ///
    /// # Returns
    /// * [KeypressResult] indicating the result of the action.
    pub fn handle_nav_action(&mut self, action: NavAction) -> KeypressResult {
        match action {
            NavAction::GoUp => {
                self.move_nav_if_possible(|nav| nav.move_up());
                self.refresh_show_info_if_open();
            }
            NavAction::GoDown => {
                self.move_nav_if_possible(|nav| nav.move_down());
                self.refresh_show_info_if_open();
            }
            NavAction::GoParent => {
                let res = self.handle_go_parent();
                self.refresh_show_info_if_open();
                return res;
            }
            NavAction::GoIntoDir => {
                let res = self.handle_go_into_dir();
                self.refresh_show_info_if_open();
                return res;
            }
            NavAction::ToggleMarker => {
                let marker_jump = self.config.display().toggle_marker_jump();
                let clipboard = self.actions.clipboard_mut();
                self.nav.toggle_marker_advance(clipboard, marker_jump);
                self.request_preview();
            }
            NavAction::ClearMarker => {
                self.nav.clear_markers();
                self.request_preview();
            }
            NavAction::ClearFilter => {
                self.nav.clear_filters();
                self.request_preview();
            }
        }
        KeypressResult::Continue
    }

    /// Handles file actions (open, delete, copy, etc).
    /// Returns a [KeypressResult] indicating how the action was handled.
    ///
    /// # Arguments
    /// * `action` - The file action to handle.
    ///
    /// # Returns
    /// * [KeypressResult] indicating the result of the action.
    pub fn handle_file_action(&mut self, action: FileAction) -> KeypressResult {
        match action {
            FileAction::Open => return self.handle_open_file(),
            FileAction::Delete => self.prompt_delete(),
            FileAction::Copy => {
                self.actions.action_copy(&self.nav, false);
                self.handle_timed_message(Duration::from_secs(15));
            }
            FileAction::Paste => {
                let fileop_tx = self.workers.fileop_tx();
                self.actions.action_paste(&mut self.nav, fileop_tx);
            }
            FileAction::Rename => self.prompt_rename(),
            FileAction::Create => self.prompt_create_file(),
            FileAction::CreateDirectory => self.prompt_create_folder(),
            FileAction::Filter => self.prompt_filter(),
            FileAction::ShowInfo => self.toggle_file_info(),
            FileAction::Find => self.prompt_find(),
        }
        KeypressResult::Continue
    }

    /// Enters an input mode with the given parameters.
    ///
    /// # Arguments
    /// * `mode` - The input mode to enter.
    /// * `prompt` - The prompt text to display.
    /// * `initial` - Optional initial text for the input buffer.
    pub fn enter_input_mode(&mut self, mode: InputMode, prompt: String, initial: Option<String>) {
        let buffer = initial.unwrap_or_default();
        self.actions
            .enter_mode(ActionMode::Input { mode, prompt }, buffer);
    }

    // Handlers

    /// Calls the provided function to move navigation if possible.
    ///
    /// If the movement was successful (f returns true), marks the preview as pending refresh.
    /// Used to encapsulate common logic for nav actions that change selection or directory.
    /// # Arguments
    /// * `f` - A closure that takes a mutable reference to [NavState] and returns a bool indicating success.
    fn move_nav_if_possible<F>(&mut self, f: F)
    where
        F: FnOnce(&mut NavState) -> bool,
    {
        if f(&mut self.nav) {
            if self.config.display().instant_preview() {
                self.request_preview();
            } else {
                self.preview.mark_pending();
            }
        }
    }

    /// Handles the go to parent directory action.
    ///
    /// If the current directory has a parent, navigates to it, saves the current position,
    /// and requests loading of the new directory and its parent content.
    ///
    /// # Returns
    /// * [KeypressResult] indicating the result of the action.
    fn handle_go_parent(&mut self) -> KeypressResult {
        if let Some(parent) = self.nav.current_dir().parent() {
            let exited_name = self.nav.current_dir().file_name().map(|n| n.to_os_string());
            let parent_path = parent.to_path_buf();
            self.nav.save_position();
            self.nav.set_path(parent_path);

            self.request_dir_load(exited_name);
            self.request_parent_content();
        }
        KeypressResult::Continue
    }

    /// Handles the go into directory action.
    ///
    /// If the selected entry is a directory, navigates into it, saves the current position,
    /// and requests loading of the new directory and its parent content.
    ///
    /// # Returns
    /// * [KeypressResult] indicating the result of the action.
    fn handle_go_into_dir(&mut self) -> KeypressResult {
        if let Some(entry) = self.nav.selected_shown_entry() {
            let cur_path = self.nav.current_dir();
            let file_name = entry.name();
            let entry_path = cur_path.join(file_name);

            if entry.is_dir() && !entry.is_symlink() {
                self.nav.save_position();
                self.nav.set_path(entry_path);
                self.request_dir_load(None);
                self.request_parent_content();
                return KeypressResult::Continue;
            }

            if entry.is_symlink()
                && let Ok(target) = std::fs::read_link(&entry_path)
            {
                let resolved = if target.is_absolute() {
                    target
                } else {
                    entry_path
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new(""))
                        .join(target)
                };
                if let Ok(meta) = std::fs::metadata(&resolved)
                    && meta.is_dir()
                {
                    self.nav.save_position();
                    self.nav.set_path(resolved);
                    self.request_dir_load(None);
                    self.request_parent_content();
                    return KeypressResult::Continue;
                }
            }
        }
        KeypressResult::Continue
    }

    /// Handles the open file action.
    ///
    /// If a file is selected, attempts to open it in the configured editor.
    /// If an error occurs, prints it to stderr.
    ///
    /// # Returns
    /// * [KeypressResult] indicating the result of the action.
    fn handle_open_file(&mut self) -> KeypressResult {
        if let Some(entry) = self.nav.selected_shown_entry() {
            let path = self.nav.current_dir().join(entry.name());
            if let Err(e) = crate::utils::open_in_editor(self.config.editor(), &path) {
                eprintln!("Error: {}", e);
            }
            KeypressResult::OpenedEditor
        } else {
            KeypressResult::Continue
        }
    }

    /// Handles the find action.
    ///
    /// If a result is selected in the find results, navigates to its path.
    /// If the path is a directory, navigates into it.
    /// If the path is a file, navigates to its parent directory and focuses on the file.
    fn handle_find(&mut self) {
        let Some(r) = self
            .actions
            .find_results()
            .get(self.actions.find_selected())
        else {
            return;
        };
        let path = r.path();
        let is_dir = path.is_dir();

        if is_dir {
            self.nav.save_position();
            self.nav.set_path(path.to_path_buf());
            self.request_dir_load(None);
            self.request_parent_content();
            return;
        }

        let Some(parent) = path.parent() else {
            return;
        };
        let focus = path.file_name().map(|n| n.to_os_string());

        self.nav.save_position();
        self.nav.set_path(parent.to_path_buf());
        self.request_dir_load(focus);
        self.request_parent_content();
    }

    /// Handles displaying a timed message overlay.
    ///
    /// # Arguments
    /// * `duration` - The duration for which the message should be displayed.
    pub fn handle_timed_message(&mut self, duration: Duration) {
        self.notification_time = Some(Instant::now() + duration);
    }

    // Input processes

    /// Processes a character input for the confirm delete input mode.
    /// # Arguments
    /// * `c` - The character input to process.
    pub fn process_confirm_delete_char(&mut self, c: char) {
        if matches!(c, 'y' | 'Y') {
            self.confirm_delete();
        }
        self.exit_input_mode();
    }

    /// Exits the current input mode.
    /// Simple wrapper around actions::exit_mode.
    pub fn exit_input_mode(&mut self) {
        self.actions.exit_mode();
    }

    /// Creates a new file with the name in the input buffer.
    /// Calls actions::action_create with `is_folder` set to false.
    fn create_file(&mut self) {
        if !self.actions.input_buffer().is_empty() {
            let fileop_tx = self.workers.fileop_tx();
            self.actions.action_create(&mut self.nav, false, fileop_tx);
        }
    }

    /// Creates a new folder with the name in the input buffer.
    /// Calls actions::action_create with `is_folder` set to true.
    fn create_folder(&mut self) {
        if !self.actions.input_buffer().is_empty() {
            let fileop_tx = self.workers.fileop_tx();
            self.actions.action_create(&mut self.nav, true, fileop_tx);
        }
    }

    /// Renames the selected entry to the name in the input buffer.
    /// Calls actions::action_rename.
    fn rename_entry(&mut self) {
        let fileop_tx = self.workers.fileop_tx();
        self.actions.action_rename(&mut self.nav, fileop_tx);
    }

    /// Applies the filter in the input buffer to the navigation state.
    /// Calls actions::action_filter and requests a preview refresh.
    fn apply_filter(&mut self) {
        self.actions.action_filter(&mut self.nav);
        self.request_preview();
    }

    /// Confirms deletion of the selected items.
    /// Calls actions::action_delete.
    fn confirm_delete(&mut self) {
        let fileop_tx = self.workers.fileop_tx();
        self.actions.action_delete(&mut self.nav, fileop_tx);
    }

    // Prompt functions

    /// Prompts the user to confirm deletion of selected items.
    fn prompt_delete(&mut self) {
        let targets = self.nav.get_action_targets();
        if targets.is_empty() {
            return;
        }
        let prompt_text = format!(
            "Delete {} item{}? [Y/N]",
            targets.len(),
            if targets.len() > 1 { "s" } else { "" }
        );
        self.enter_input_mode(InputMode::ConfirmDelete, prompt_text, None);
    }

    /// Prompts the user to rename the selected entry.
    fn prompt_rename(&mut self) {
        if let Some(entry) = self.nav.selected_shown_entry() {
            let name = entry.name().to_string_lossy().to_string();
            self.enter_input_mode(InputMode::Rename, "Rename: ".to_string(), Some(name));
        }
    }

    /// Prompts the user to create a new file.
    fn prompt_create_file(&mut self) {
        self.enter_input_mode(InputMode::NewFile, "New File: ".to_string(), None);
    }

    /// Prompts the user to create a new folder.
    fn prompt_create_folder(&mut self) {
        self.enter_input_mode(InputMode::NewFolder, "New Folder: ".to_string(), None);
    }

    /// Prompts the user to enter a filter string.
    fn prompt_filter(&mut self) {
        let current_filter = self.nav.filter().to_string();
        self.enter_input_mode(
            InputMode::Filter,
            "Filter: ".to_string(),
            Some(current_filter),
        );
    }

    /// Prompts the user to enter a fuzzy find query.
    /// Requires the `fd` tool to be installed.
    /// If `fd` is not found, displays a temporary overlay message.
    fn prompt_find(&mut self) {
        if which::which("fd").is_err() {
            self.push_overlay_message(
                "Fuzzy Find requires the `fd` tool.".to_string(),
                Duration::from_secs(5),
            );
            return;
        }
        self.enter_input_mode(InputMode::Find, "".to_string(), None);
    }

    // Helpers

    /// Refreshes the file info overlay if it is currently open.
    pub fn refresh_show_info_if_open(&mut self) {
        let maybe_idx = self
            .overlays()
            .find_index(|o| matches!(o, Overlay::ShowInfo { .. }));

        if let Some(i) = maybe_idx
            && let Some(entry) = self.nav.selected_shown_entry()
        {
            let path = self.nav.current_dir().join(entry.name());
            if let Ok(file_info) = FileInfo::get_file_info(&path)
                && let Some(Overlay::ShowInfo { info }) = self.overlays_mut().get_mut(i)
            {
                *info = file_info;
            }
        }
    }

    /// Shows the file info overlay for the currently selected entry.
    fn show_file_info(&mut self) {
        if let Some(entry) = self.nav.selected_shown_entry() {
            let path = self.nav.current_dir().join(entry.name());
            if let Ok(file_info) = FileInfo::get_file_info(&path) {
                self.overlays_mut()
                    .push(Overlay::ShowInfo { info: file_info });
            }
        }
    }

    /// Toggles the file info overlay.
    fn toggle_file_info(&mut self) {
        let is_open = self
            .overlays()
            .iter()
            .any(|o| matches!(o, Overlay::ShowInfo { .. }));

        if is_open {
            self.overlays_mut()
                .retain(|o| !matches!(o, Overlay::ShowInfo { .. }));
        } else {
            self.show_file_info();
        }
    }

    /// Pushes a message overlay that lasts for the specified duration.
    pub fn push_overlay_message(&mut self, text: String, duration: Duration) {
        self.notification_time = Some(Instant::now() + duration);

        if matches!(self.overlays.top(), Some(Overlay::Message { .. })) {
            self.overlays_mut().pop();
        }

        self.overlays_mut().push(Overlay::Message { text });
    }
}
