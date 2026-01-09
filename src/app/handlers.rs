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

impl<'a> AppState<'a> {
    // AppState core handlers

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
        }
        KeypressResult::Continue
    }

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

    fn handle_go_into_dir(&mut self) -> KeypressResult {
        if let Some(entry) = self.nav.selected_shown_entry()
            && entry.is_dir()
        {
            let new_path = self.nav.current_dir().join(entry.name());
            self.nav.save_position();
            self.nav.set_path(new_path);

            self.request_dir_load(None);
            self.request_parent_content();
        }
        KeypressResult::Continue
    }

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

    pub fn handle_timed_message(&mut self, duration: Duration) {
        self.notification_time = Some(Instant::now() + duration);
    }

    // Input processes

    pub fn process_confirm_delete_char(&mut self, c: char) {
        if matches!(c, 'y' | 'Y') {
            self.confirm_delete();
        }
        self.exit_input_mode();
    }

    pub fn exit_input_mode(&mut self) {
        self.actions.exit_mode();
    }

    fn create_file(&mut self) {
        if !self.actions.input_buffer().is_empty() {
            let fileop_tx = self.workers.fileop_tx();
            self.actions.action_create(&mut self.nav, false, fileop_tx);
        }
    }

    fn create_folder(&mut self) {
        if !self.actions.input_buffer().is_empty() {
            let fileop_tx = self.workers.fileop_tx();
            self.actions.action_create(&mut self.nav, true, fileop_tx);
        }
    }

    fn rename_entry(&mut self) {
        let fileop_tx = self.workers.fileop_tx();
        self.actions.action_rename(&mut self.nav, fileop_tx);
    }

    fn apply_filter(&mut self) {
        self.actions.action_filter(&mut self.nav);
        self.request_preview();
    }

    fn confirm_delete(&mut self) {
        let fileop_tx = self.workers.fileop_tx();
        self.actions.action_delete(&mut self.nav, fileop_tx);
    }

    // Prompt functions

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

    fn prompt_rename(&mut self) {
        if let Some(entry) = self.nav.selected_shown_entry() {
            let name = entry.name().to_string_lossy().to_string();
            self.enter_input_mode(InputMode::Rename, "Rename: ".to_string(), Some(name));
        }
    }

    fn prompt_create_file(&mut self) {
        self.enter_input_mode(InputMode::NewFile, "New File: ".to_string(), None);
    }

    fn prompt_create_folder(&mut self) {
        self.enter_input_mode(InputMode::NewFolder, "New Folder: ".to_string(), None);
    }

    fn prompt_filter(&mut self) {
        let current_filter = self.nav.filter().to_string();
        self.enter_input_mode(
            InputMode::Filter,
            "Filter: ".to_string(),
            Some(current_filter),
        );
    }

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

    fn show_file_info(&mut self) {
        if let Some(entry) = self.nav.selected_shown_entry() {
            let path = self.nav.current_dir().join(entry.name());
            if let Ok(file_info) = FileInfo::get_file_info(&path) {
                self.overlays_mut()
                    .push(Overlay::ShowInfo { info: file_info });
            }
        }
    }

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

    pub fn push_overlay_message(&mut self, text: String, duration: Duration) {
        self.notification_time = Some(Instant::now() + duration);

        if matches!(self.overlays.top(), Some(Overlay::Message { .. })) {
            self.overlays_mut().pop();
        }

        self.overlays_mut().push(Overlay::Message { text });
    }
}
