use crate::app::actions::{ActionMode, InputMode};
use crate::app::{AppState, KeypressResult, NavState};
use crate::keymap::{FileAction, NavAction};
use crossterm::event::{KeyCode::*, KeyEvent};

impl<'a> AppState<'a> {
    // Handlers for app.rs

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
                }
                self.exit_input_mode();
                KeypressResult::Consumed
            }

            Esc => {
                self.exit_input_mode();
                KeypressResult::Consumed
            }

            Backspace => {
                self.actions.input_buffer_mut().pop();
                if matches!(mode, InputMode::Filter) {
                    self.apply_filter();
                }
                KeypressResult::Consumed
            }

            Char(c) => match mode {
                InputMode::ConfirmDelete => {
                    self.process_confirm_delete_char(c);
                    KeypressResult::Consumed
                }
                InputMode::Filter => {
                    self.actions.input_buffer_mut().push(c);
                    self.apply_filter();
                    KeypressResult::Consumed
                }
                InputMode::Rename | InputMode::NewFile | InputMode::NewFolder => {
                    self.actions.input_buffer_mut().push(c);
                    if matches!(mode, InputMode::Filter) {
                        self.apply_filter();
                    }
                    KeypressResult::Consumed
                }
            },

            _ => KeypressResult::Consumed,
        }
    }

    pub fn process_enter_input(&mut self, kind: &InputMode) {
        match kind {
            InputMode::NewFile => self.create_file(),
            InputMode::NewFolder => self.create_folder(),
            InputMode::Rename => self.rename_entry(),
            InputMode::Filter => self.apply_filter(),
            InputMode::ConfirmDelete => self.confirm_delete(),
        }
        self.exit_input_mode();
    }

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
            self.actions
                .action_create(&mut self.nav, false, &self.worker_tx);
        }
    }

    fn create_folder(&mut self) {
        if !self.actions.input_buffer().is_empty() {
            self.actions
                .action_create(&mut self.nav, true, &self.worker_tx);
        }
    }

    fn rename_entry(&mut self) {
        self.actions.action_rename(&mut self.nav, &self.worker_tx);
    }

    fn apply_filter(&mut self) {
        self.actions.action_filter(&mut self.nav);
    }

    fn confirm_delete(&mut self) {
        self.actions.action_delete(&mut self.nav, &self.worker_tx);
    }

    pub fn handle_nav_action(&mut self, action: NavAction) -> KeypressResult {
        match action {
            NavAction::GoUp => self.move_nav_if_possible(|nav| nav.move_up()),
            NavAction::GoDown => self.move_nav_if_possible(|nav| nav.move_down()),
            NavAction::GoParent => return self.handle_go_parent(),
            NavAction::GoIntoDir => return self.handle_go_into_dir(),
            NavAction::ToggleMarker => self.nav.toggle_marker(),
        }
        KeypressResult::Continue
    }

    fn move_nav_if_possible<F>(&mut self, f: F)
    where
        F: FnOnce(&mut NavState) -> bool,
    {
        if f(&mut self.nav) {
            self.preview.mark_pending();
        }
    }

    pub fn handle_go_parent(&mut self) -> KeypressResult {
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

    pub fn handle_go_into_dir(&mut self) -> KeypressResult {
        if let Some(entry) = self.nav.selected_entry() {
            if entry.is_dir() {
                let new_path = self.nav.current_dir().join(entry.name());
                self.nav.save_position();
                self.nav.set_path(new_path);

                self.request_dir_load(None);
                self.request_parent_content();
            }
        }
        KeypressResult::Continue
    }

    pub fn handle_file_action(&mut self, action: FileAction) -> KeypressResult {
        match action {
            FileAction::Open => return self.handle_open_file(),
            FileAction::Delete => self.prompt_delete(),
            FileAction::Copy => self.actions.action_copy(&self.nav, false),
            FileAction::Cut => self.actions.action_copy(&self.nav, true),
            FileAction::Paste => self.actions.action_paste(&mut self.nav, &self.worker_tx),
            FileAction::Rename => self.prompt_rename(),
            FileAction::Create => self.prompt_create_file(),
            FileAction::CreateDirectory => self.prompt_create_folder(),
            FileAction::Filter => self.prompt_filter(),
        }
        KeypressResult::Continue
    }

    fn handle_open_file(&mut self) -> KeypressResult {
        if let Some(entry) = self.nav.selected_entry() {
            let path = self.nav.current_dir().join(entry.name());
            if let Err(e) = crate::utils::open_in_editor(self.config.editor(), &path) {
                eprintln!("Error: {}", e);
            }
            KeypressResult::OpenedEditor
        } else {
            KeypressResult::Continue
        }
    }

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
        if let Some(entry) = self.nav.selected_entry() {
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

    pub fn enter_input_mode(&mut self, mode: InputMode, prompt: String, initial: Option<String>) {
        let buffer = initial.unwrap_or_default();
        self.actions
            .enter_mode(ActionMode::Input { mode, prompt }, buffer);
    }
}
