//! Input action handler methods for runa.
//!
//! This module implements [AppState] methods that process key events, file/nav actions,
//! and input modes (rename, filter, etc).

use crate::app::NavState;
use crate::app::actions::{ActionMode, InputMode};
use crate::app::keymap::{FileAction, NavAction, PrefixCommand, SystemAction};
use crate::app::state::{AppState, KeypressResult};
use crate::core::FileInfo;
use crate::core::proc::{complete_dirs_with_fd, fd_binary};
use crate::ui::overlays::Overlay;
use crate::utils::{
    clean_display_path, expand_home_path, expand_home_path_buf, get_home, normalize_relative_path,
    open_in_editor,
};

use crossterm::event::{KeyCode::*, KeyEvent};
use std::path::MAIN_SEPARATOR;
use std::time::{Duration, Instant};

/// AppState input and action handlers
impl<'a> AppState<'a> {
    // AppState core handlers

    /// Handles key events when in an input mode (rename, filter, etc).
    /// Returns a [KeypressResult] indicating how the key event was handled.
    ///
    /// If not in an input mode, returns [KeypressResult::Continue].
    /// Consumes keys related to input editing and mode confirmation/cancellation.
    pub(super) fn handle_input_mode(&mut self, key: KeyEvent) -> KeypressResult {
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
                    InputMode::ConfirmDelete { .. } => self.confirm_delete(),
                    InputMode::Find => self.handle_find(),
                    InputMode::MoveFile => self.handle_move(),
                    InputMode::GoToPath => self.handle_go_to_path(),
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
                InputMode::MoveFile => {
                    self.actions.action_insert_at_cursor(c);
                    KeypressResult::Consumed
                }
                InputMode::GoToPath => {
                    self.actions.action_insert_at_cursor(c);
                    KeypressResult::Consumed
                }
            },

            _ => KeypressResult::Consumed,
        }
    }

    /// Handles navigation actions (up, down, into dir, etc).
    /// Returns a [KeypressResult] indicating how the action was handled.
    pub(super) fn handle_nav_action(&mut self, action: NavAction) -> KeypressResult {
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
            _ => {}
        }
        KeypressResult::Continue
    }

    /// Handles file actions (open, delete, copy, etc).
    /// Returns a [KeypressResult] indicating how the action was handled.
    pub(super) fn handle_file_action(&mut self, action: FileAction) -> KeypressResult {
        match action {
            FileAction::Open => return self.handle_open_file(),
            FileAction::Delete => {
                let is_trash = self.config.general().move_to_trash();
                self.prompt_delete(is_trash);
            }
            FileAction::AlternateDelete => {
                let is_trash = !self.config.general().move_to_trash();
                self.prompt_delete(is_trash);
            }
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
            FileAction::MoveFile => self.prompt_move(),
        }
        KeypressResult::Continue
    }

    pub(super) fn handle_sys_action(&mut self, action: SystemAction) -> KeypressResult {
        match action {
            SystemAction::Quit => KeypressResult::Quit,
            SystemAction::KeyBindHelp => {
                self.toggle_keybind_help();
                KeypressResult::Consumed
            }
        }
    }

    pub(super) fn handle_prefix_dispatch(&mut self, key: &KeyEvent) -> Option<KeypressResult> {
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
            let _ = self.handle_prefix_action(cmd);
            return Some(KeypressResult::Consumed);
        }

        None
    }

    fn handle_prefix_action(&mut self, prefix: PrefixCommand) -> bool {
        match prefix {
            PrefixCommand::Nav(NavAction::GoToTop) => self.handle_go_to_top(),
            PrefixCommand::Nav(NavAction::GoToHome) => self.handle_go_to_home(),
            PrefixCommand::Nav(NavAction::GoToPath) => self.prompt_go_to_path(),
            _ => return false,
        }
        true
    }

    pub(super) fn handle_esc_close_overlays(&mut self, key: &KeyEvent) -> Option<KeypressResult> {
        if key.code != Esc {
            return None;
        }

        if self
            .overlays()
            .iter()
            .any(|o| matches!(o, Overlay::KeybindHelp))
        {
            self.overlays_mut()
                .retain(|o| !matches!(o, Overlay::KeybindHelp));
            return Some(KeypressResult::Consumed);
        }

        None
    }

    /// Enters an input mode with the given parameters.
    pub(crate) fn enter_input_mode(
        &mut self,
        mode: InputMode,
        prompt: String,
        initial: Option<String>,
    ) {
        let buffer = initial.unwrap_or_default();
        self.actions
            .enter_mode(ActionMode::Input { mode, prompt }, buffer);
    }

    // Handlers

    /// Calls the provided function to move navigation if possible.
    ///
    /// If the movement was successful (f returns true), marks the preview as pending refresh.
    /// Used to encapsulate common logic for nav actions that change selection or directory.
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
    fn handle_go_parent(&mut self) -> KeypressResult {
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
        self.nav.save_position();
        self.nav.set_path(parent_path);

        self.request_dir_load(exited_name);
        self.request_parent_content();

        KeypressResult::Continue
    }

    /// Handles the go into directory action.
    ///
    /// If the selected entry is a directory, navigates into it, saves the current position,
    /// and requests loading of the new directory and its parent content.
    fn handle_go_into_dir(&mut self) -> KeypressResult {
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
                self.nav.save_position();
                self.nav.set_path(entry_path);
                self.request_dir_load(None);
                self.request_parent_content();
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

    /// Handles the open file action.
    ///
    /// If a file is selected, attempts to open it in the configured editor.
    /// If an error occurs, prints it to stderr.
    fn handle_open_file(&mut self) -> KeypressResult {
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
                    self.request_preview();
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

        self.nav.save_position();
        self.nav.set_path(target_dir);
        self.request_dir_load(focus);
        self.request_parent_content();

        self.exit_input_mode();
    }

    /// Handles the move action
    ///
    /// Checks if the directory for files to be moved to exists
    /// Also normalizes relative paths for easier moving of files.
    fn handle_move(&mut self) {
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
                let norm_msg = normalize_relative_path(&resolved_path);
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
            let norm_msg = normalize_relative_path(&absolute_dest);
            self.push_overlay_message(
                format!("Move failed: Permission denied in {}: {}", norm_msg, e),
                Duration::from_secs(3),
            );
            return;
        }

        if !absolute_dest.is_dir() {
            let norm_msg = normalize_relative_path(&absolute_dest);
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
                    let normalized = normalize_relative_path(&absolute_src);
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

        let fileop_tx = self.workers.fileop_tx();
        let move_msg = format!(
            "Files moved to: {}",
            clean_display_path(&absolute_dest.to_string_lossy())
        );

        self.actions
            .actions_move(&mut self.nav, absolute_dest, fileop_tx);

        self.exit_input_mode();
        self.push_overlay_message(move_msg, Duration::from_secs(3));
    }

    fn handle_go_to_home(&mut self) {
        if let Some(home_path) = get_home() {
            self.nav.save_position();
            self.nav.set_path(home_path.clone());
            self.request_dir_load(None);
            self.request_parent_content();
        }
    }

    fn handle_go_to_path(&mut self) {
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
                self.nav.save_position();
                self.nav.set_path(abs_path.clone());
                self.request_dir_load(None);
                self.request_parent_content();
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

    fn handle_go_to_top(&mut self) {
        self.nav.first_selected();
        self.request_preview();
    }

    /// Handles displaying a timed message overlay.
    fn handle_timed_message(&mut self, duration: Duration) {
        self.notification_time = Some(Instant::now() + duration);
    }

    // Input processes

    /// Processes a character input for the confirm delete input mode.
    fn process_confirm_delete_char(&mut self, c: char) {
        if matches!(c, 'y' | 'Y') {
            self.confirm_delete();
        }
        self.exit_input_mode();
    }

    /// Exits the current input mode.
    /// Simple wrapper around actions::exit_mode.
    fn exit_input_mode(&mut self) {
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
        let move_to_trash = if let ActionMode::Input {
            mode: InputMode::ConfirmDelete { is_trash },
            ..
        } = self.actions.mode()
        {
            *is_trash
        } else {
            self.config.general().move_to_trash()
        };

        let fileop_tx = self.workers.fileop_tx();

        self.actions
            .action_delete(&mut self.nav, fileop_tx, move_to_trash);
    }

    // Prompt functions

    /// Prompts the user to confirm deletion of selected items.
    fn prompt_delete(&mut self, is_trash: bool) {
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
        if fd_binary().is_err() {
            self.push_overlay_message(
                "Fuzzy Find requires the `fd` tool.".to_string(),
                Duration::from_secs(5),
            );
            return;
        }
        self.enter_input_mode(InputMode::Find, "".to_string(), None);
    }

    fn prompt_move(&mut self) {
        let prompt = "Move to directory: ".to_string();
        self.enter_input_mode(InputMode::MoveFile, prompt, None);
    }

    fn prompt_go_to_path(&mut self) {
        self.enter_input_mode(InputMode::GoToPath, "Go To Path:".to_string(), None);
    }

    /// Refreshes the file info overlay if it is currently open.
    pub(crate) fn refresh_show_info_if_open(&mut self) {
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

    fn show_prefix_help(&mut self) {
        if !matches!(self.overlays().top(), Some(Overlay::PreifxHelp)) {
            self.overlays_mut().push(Overlay::PreifxHelp);
        }
    }

    pub(crate) fn hide_prefix_help(&mut self) {
        if matches!(self.overlays().top(), Some(Overlay::PreifxHelp)) {
            self.overlays_mut().pop();
        }
    }

    fn toggle_keybind_help(&mut self) {
        let is_open = self
            .overlays()
            .iter()
            .any(|o| matches!(o, Overlay::KeybindHelp));

        if is_open {
            self.overlays_mut()
                .retain(|o| !matches!(o, Overlay::KeybindHelp));
        } else {
            self.overlays_mut().push(Overlay::KeybindHelp);
        }
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

    /// Pushes a message overlay that lasts for the specified duration.
    pub(crate) fn push_overlay_message(&mut self, text: String, duration: Duration) {
        self.notification_time = Some(Instant::now() + duration);

        if matches!(self.overlays.top(), Some(Overlay::Message { .. })) {
            self.overlays_mut().pop();
        }

        self.overlays_mut().push(Overlay::Message { text });
    }
}
