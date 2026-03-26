use crate::app::Workers;
use crate::app::actions::{ActionMode, InputMode};
use crate::app::keymap::{Action, NavAction, PrefixCommand, SystemAction};
use crate::app::state::{AppState, KeypressResult};
use crate::core::proc::{complete_dirs_with_fd, fd_binary};
use crate::ui::overlays::Overlay;
use crate::utils::expand_home_path;

use crossterm::event::{KeyCode::*, KeyEvent};
use std::path::MAIN_SEPARATOR;
use std::sync::Arc;
use std::time::Duration;

/// AppState input and action handlers
impl<'a> AppState<'a> {
    // AppState core handlers

    /// Handles key events when in an input mode (rename, filter, etc).
    /// Returns a [KeypressResult] indicating how the key event was handled.
    ///
    /// If not in an input mode, returns [KeypressResult::Continue].
    /// Consumes keys related to input editing and mode confirmation/cancellation.
    pub(in crate::app) fn handle_input_mode(
        &mut self,
        workers: &Workers,
        key: KeyEvent,
    ) -> KeypressResult {
        let prev_action_mode = self.actions().mode().clone();
        let ActionMode::Input { mode, .. } = &prev_action_mode else {
            return KeypressResult::Continue;
        };

        if let Some(action) = self.keymap.lookup(key) {
            match action {
                Action::Nav(NavAction::ScrollUp) => {
                    self.actions.scroll().scroll_up();
                    return KeypressResult::Consumed;
                }
                Action::Nav(NavAction::ScrollDown) => {
                    self.actions.scroll().scroll_down();
                    return KeypressResult::Consumed;
                }
                _ => {}
            }
        }

        match key.code {
            Enter => {
                match mode {
                    InputMode::NewFile => self.create_file(workers),
                    InputMode::NewFolder => self.create_folder(workers),
                    InputMode::Rename => self.rename_entry(workers),
                    InputMode::Filter => self.apply_filter(workers),
                    InputMode::ConfirmDelete { .. } => self.confirm_delete(workers),
                    InputMode::ConfirmOverwrite { .. } => self.confirm_overwrite(workers),
                    InputMode::Find => self.handle_find(workers),
                    InputMode::MoveFile => self.handle_move(workers),
                    InputMode::GoToPath => self.handle_go_to_path(workers),
                }
                if self.actions().mode() == &prev_action_mode {
                    self.exit_input_mode();
                }
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
                    self.apply_filter(workers);
                }
                if matches!(mode, InputMode::Find) {
                    self.actions.find_debounce(Duration::from_millis(90));
                }
                KeypressResult::Consumed
            }

            Tab => {
                if matches!(mode, InputMode::MoveFile | InputMode::GoToPath) {
                    if fd_binary().is_ok() {
                        self.tab_autocomplete();
                        KeypressResult::Consumed
                    } else {
                        KeypressResult::Continue
                    }
                } else {
                    KeypressResult::Continue
                }
            }

            Char(c) => match mode {
                InputMode::ConfirmDelete { .. } => {
                    self.process_confirm_delete_char(workers, c);
                    KeypressResult::Consumed
                }
                InputMode::ConfirmOverwrite { .. } => {
                    self.process_confirm_overwrite_char(workers, c);
                    KeypressResult::Consumed
                }
                InputMode::Filter => {
                    self.actions.action_insert_at_cursor(c);
                    self.apply_filter(workers);
                    KeypressResult::Consumed
                }
                InputMode::Rename
                | InputMode::NewFile
                | InputMode::NewFolder
                | InputMode::MoveFile
                | InputMode::GoToPath => {
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

    pub(in crate::app) fn handle_sys_action(&mut self, action: SystemAction) -> KeypressResult {
        match action {
            SystemAction::Quit => KeypressResult::Quit,
            SystemAction::KeyBindHelp => {
                self.toggle_keybind_help();
                KeypressResult::Consumed
            }
        }
    }

    pub(in crate::app) fn handle_prefix_dispatch(
        &mut self,
        workers: &Workers,
        key: &KeyEvent,
    ) -> Option<KeypressResult> {
        let gmap = self.keymap.gmap();

        let (started, exited, result, consumed) = {
            let prefix = self.actions.prefix_recognizer_mut();
            let was_g = prefix.is_g_state();

            let result = prefix.feed(key, gmap);

            let consumed = was_g && key.code == Esc;

            (
                prefix.started_prefix(),
                prefix.exited_prefix(),
                result,
                consumed,
            )
        };

        if started {
            self.show_prefix_help();
        }
        if exited {
            self.hide_prefix_help();
        }

        if consumed {
            return Some(KeypressResult::Consumed);
        }

        if let Some(cmd) = result {
            let _ = self.handle_prefix_action(workers, cmd);
            return Some(KeypressResult::Consumed);
        }

        None
    }

    fn handle_prefix_action(&mut self, workers: &Workers, prefix: PrefixCommand) -> bool {
        match prefix {
            PrefixCommand::Nav(NavAction::GoToTop) => {
                self.handle_go_to_top(workers);
                self.refresh_show_info_if_open();
            }
            PrefixCommand::Nav(NavAction::GoToHome) => {
                self.handle_go_to_home(workers);
                self.refresh_show_info_if_open();
            }
            PrefixCommand::Nav(NavAction::GoToPath) => {
                self.prompt_go_to_path();
                self.refresh_show_info_if_open();
            }
            _ => return false,
        }
        true
    }

    /// Enters an input mode with the given parameters.
    pub(super) fn enter_input_mode(
        &mut self,
        mode: InputMode,
        prompt: String,
        initial: Option<String>,
    ) {
        self.overlays_mut()
            .retain(|o| !matches!(o, Overlay::KeybindHelp));

        let buffer = initial.unwrap_or_default();
        self.actions
            .enter_mode(ActionMode::Input { mode, prompt }, buffer);
        self.actions.scroll().reset();
    }

    /// Exits the current input mode.
    /// Simple wrapper around actions::exit_mode.
    pub(super) fn exit_input_mode(&mut self) {
        self.actions.exit_mode();
    }

    /// Processes a character input for the confirm delete input mode.
    fn process_confirm_delete_char(&mut self, workers: &Workers, c: char) {
        if matches!(c, 'y' | 'Y') {
            self.confirm_delete(workers);
        }
        self.exit_input_mode();
    }

    fn process_confirm_overwrite_char(&mut self, workers: &Workers, c: char) {
        if matches!(c, 'y' | 'Y') {
            self.confirm_overwrite(workers);
        }
        self.exit_input_mode();
    }

    /// Creates a new file with the name in the input buffer.
    /// Calls actions::action_create with `is_folder` set to false.
    fn create_file(&mut self, workers: &Workers) {
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
    fn create_folder(&mut self, workers: &Workers) {
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
    fn rename_entry(&mut self, workers: &Workers) {
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
    fn apply_filter(&mut self, workers: &Workers) {
        self.actions.action_filter(&mut self.nav);
        self.request_preview(workers);
    }

    /// Confirms deletion of the selected items.
    /// Calls actions::action_delete.
    fn confirm_delete(&mut self, workers: &Workers) {
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

    fn confirm_overwrite(&mut self, workers: &Workers) {
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

    // Prompt functions

    /// Prompts the user to confirm deletion of selected items.
    pub(super) fn prompt_delete(&mut self, is_trash: bool) {
        let targets = self.nav.get_action_targets();
        let count = targets.len();
        if targets.is_empty() {
            return;
        }

        let action_word = if is_trash { "Trash" } else { "Delete" };
        let item_label = if count > 1 {
            format!("{} items", count)
        } else {
            "item".to_string()
        };

        let prompt_text = format!("{} {}? [Y/n]", action_word, item_label);
        self.enter_input_mode(InputMode::ConfirmDelete { is_trash }, prompt_text, None);
    }

    /// Prompts the user to rename the selected entry.
    pub(super) fn prompt_rename(&mut self) {
        if let Some(entry) = self.nav.selected_shown_entry() {
            let name = entry.name().to_string_lossy().to_string();
            self.enter_input_mode(InputMode::Rename, "Rename: ".to_string(), Some(name));
        }
    }

    /// Prompts the user to create a new file.
    pub(super) fn prompt_create_file(&mut self) {
        self.enter_input_mode(InputMode::NewFile, "New File: ".to_string(), None);
    }

    /// Prompts the user to create a new folder.
    pub(super) fn prompt_create_folder(&mut self) {
        self.enter_input_mode(InputMode::NewFolder, "New Folder: ".to_string(), None);
    }

    /// Prompts the user to enter a filter string.
    pub(super) fn prompt_filter(&mut self) {
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
    pub(super) fn prompt_find(&mut self) {
        if fd_binary().is_err() {
            self.push_overlay_message(
                "Fuzzy Find requires the `fd` tool.".to_string(),
                Duration::from_secs(5),
            );
            return;
        }
        self.enter_input_mode(InputMode::Find, "".to_string(), None);
    }

    pub(super) fn prompt_move(&mut self) {
        let prompt = "Move to directory: ".to_string();
        self.enter_input_mode(InputMode::MoveFile, prompt, None);
    }

    fn prompt_go_to_path(&mut self) {
        self.enter_input_mode(InputMode::GoToPath, "Go To Path:".to_string(), None);
    }

    /// Handles the autocomplete for move to directory action
    fn tab_autocomplete(&mut self) {
        if fd_binary().is_err() {
            return;
        }

        let input = self.actions.input_buffer().to_string();
        let expanded = expand_home_path(input.trim());

        let (base_dir, prefix) = if let Some(idx) = expanded.rfind(MAIN_SEPARATOR) {
            let (base, frag) = expanded.split_at(idx + 1);
            (std::path::Path::new(base), frag)
        } else {
            (self.nav.current_dir(), expanded.as_str())
        };

        let show_hidden = self.config.general().show_hidden();

        let suggestion_opt = {
            let ac = self.actions.autocomplete_mut();

            let needs_update = ac.last_input() != input || ac.suggestions().is_empty();
            if needs_update {
                let suggestions =
                    complete_dirs_with_fd(base_dir, prefix, show_hidden).unwrap_or_default();
                ac.update(suggestions, &input);
            }

            let suggestion = ac.current().cloned();

            if suggestion.is_some() {
                ac.advance();
            }

            suggestion
        };

        if let Some(suggestion) = suggestion_opt {
            let mut completed_path = base_dir.to_path_buf();
            completed_path.push(&suggestion);
            let mut out = completed_path.to_string_lossy().to_string();
            if !out.ends_with(MAIN_SEPARATOR) {
                out.push(MAIN_SEPARATOR);
            }
            self.actions.set_input_buffer(out);
        }
    }
}
