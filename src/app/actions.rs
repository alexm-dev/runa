//! Action context and input mode logic for runa.
//!
//! Contains the [ActionContext] struct, tracking user input state, clipboard, and action modes.
//! Defines available modes/actions for file operations (copy, paste, rename, create, delete, filter).

use crate::app::nav::NavState;
use crate::core::FileInfo;
use crate::core::find::FindResult;
use crate::core::worker::{FileOperation, WorkerTask};
use crossbeam_channel::Sender;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::time::Instant;

/// Describes the current mode for action handling/input.
///
/// Used to determine which UI overlays, prompts, or context actions should be active.
#[derive(Clone, PartialEq)]
pub enum ActionMode {
    Normal,
    Input { mode: InputMode, prompt: String },
    ShowInfo { info: FileInfo },
}

/// Enumerates all the available input field modes
///
/// Used to select the prompts, behavior and the style of the input dialog.
#[derive(Clone, Copy, PartialEq)]
pub enum InputMode {
    Rename,
    NewFile,
    NewFolder,
    Filter,
    ConfirmDelete,
    Find,
}

/// Tracks current user action and input buffer state for file operations and commands.
///
/// Stores the current mode/prompt, input buffer, cursor, and clipboard (for copy/yank) status.
/// Handles mutation for input, clipboard & command responses.
pub struct ActionContext {
    mode: ActionMode,
    input_buffer: String,
    input_cursor_pos: usize,
    clipboard: Option<HashSet<PathBuf>>,
    is_cut: bool,
    find: FindState,
}

impl ActionContext {
    // Getters / accessors

    pub fn mode(&self) -> &ActionMode {
        &self.mode
    }

    pub fn input_buffer(&self) -> &str {
        &self.input_buffer
    }

    pub fn input_cursor_pos(&self) -> usize {
        self.input_cursor_pos
    }

    pub fn clipboard(&self) -> &Option<HashSet<PathBuf>> {
        &self.clipboard
    }

    // Find functions

    pub fn find_state_mut(&mut self) -> &mut FindState {
        &mut self.find
    }

    pub fn find_results(&self) -> &[FindResult] {
        self.find.results()
    }

    pub fn find_selected(&self) -> usize {
        self.find.selected()
    }

    pub fn set_find_results(&mut self, results: Vec<FindResult>) {
        self.find.set_results(results)
    }

    pub fn clear_find_results(&mut self) {
        self.find.clear_results()
    }

    pub fn find_request_id(&self) -> u64 {
        self.find.request_id()
    }

    pub fn prepare_new_find_request(&mut self) -> u64 {
        self.find.prepare_new_request()
    }

    pub fn take_query(&mut self) -> Option<String> {
        self.find.take_query(&self.input_buffer)
    }

    pub fn find_debounce(&mut self, delay: Duration) {
        self.find.set_debounce(delay);
    }

    pub fn cancel_find(&mut self) {
        self.find.cancel_current();
    }

    pub fn set_cancel_find_token(&mut self, token: Arc<AtomicBool>) {
        self.find.set_cancel(token);
    }

    // Mode functions

    pub fn is_input_mode(&self) -> bool {
        matches!(self.mode, ActionMode::Input { .. })
    }

    pub fn enter_mode(&mut self, mode: ActionMode, initial_value: String) {
        self.mode = mode;
        self.input_buffer = initial_value;
        self.input_cursor_pos = self.input_buffer.len();
    }

    pub fn exit_mode(&mut self) {
        self.mode = ActionMode::Normal;
        self.input_buffer.clear();
        self.find.reset();
    }

    // Actions functions

    pub fn action_delete(&mut self, nav: &mut NavState, worker_tx: &Sender<WorkerTask>) {
        let targets = nav.get_action_targets();
        if targets.is_empty() {
            return;
        }

        let _ = worker_tx.send(WorkerTask::FileOp {
            op: FileOperation::Delete(targets.into_iter().collect()),
            request_id: nav.prepare_new_request(),
        });

        nav.clear_markers();
    }

    // Currently, cut/move is not implemented yet. Only copy/yank is used.
    // This allows for easy addition of a cut/move feature in the future.
    pub fn action_copy(&mut self, nav: &NavState, is_cut: bool) {
        let mut set = HashSet::new();
        if !nav.markers().is_empty() {
            for path in nav.markers() {
                set.insert(path.clone());
            }
        } else if let Some(entry) = nav.selected_entry() {
            set.insert(nav.current_dir().join(entry.name()));
        }
        if !set.is_empty() {
            self.clipboard = Some(set);
            self.is_cut = is_cut;
        }
    }

    pub fn action_paste(&mut self, nav: &mut NavState, worker_tx: &Sender<WorkerTask>) {
        if let Some(source) = &self.clipboard {
            let first_file_name = source
                .iter()
                .min()
                .and_then(|p| p.file_name())
                .map(|n| n.to_os_string());

            let _ = worker_tx.send(WorkerTask::FileOp {
                op: FileOperation::Copy {
                    src: source.iter().cloned().collect(),
                    dest: nav.current_dir().to_path_buf(),
                    cut: self.is_cut,
                    focus: first_file_name,
                },
                request_id: nav.prepare_new_request(),
            });
            if self.is_cut {
                self.clipboard = None;
            }
            nav.clear_markers();
        }
    }

    pub fn action_filter(&mut self, nav: &mut NavState) {
        nav.set_filter(self.input_buffer.clone());
    }

    pub fn action_rename(&mut self, nav: &mut NavState, worker_tx: &Sender<WorkerTask>) {
        if self.input_buffer.is_empty() {
            return;
        }
        if let Some(entry) = nav.selected_entry() {
            let old_path = nav.current_dir().join(entry.name());
            let new_path = old_path.with_file_name(&self.input_buffer);

            let _ = worker_tx.send(WorkerTask::FileOp {
                op: FileOperation::Rename {
                    old: old_path,
                    new: new_path,
                },
                request_id: nav.prepare_new_request(),
            });
        }
        self.exit_mode();
    }

    pub fn action_create(
        &mut self,
        nav: &mut NavState,
        is_dir: bool,
        worker_tx: &Sender<WorkerTask>,
    ) {
        if self.input_buffer.is_empty() {
            return;
        }

        let path = nav.current_dir().join(&self.input_buffer);
        let _ = worker_tx.send(WorkerTask::FileOp {
            op: FileOperation::Create { path, is_dir },
            request_id: nav.prepare_new_request(),
        });
        self.exit_mode();
    }

    // Cursor actions

    pub fn action_move_cursor_left(&mut self) {
        if self.input_cursor_pos > 0 {
            self.input_cursor_pos -= 1;
        }
    }

    pub fn action_move_cursor_right(&mut self) {
        if self.input_cursor_pos < self.input_buffer.len() {
            self.input_cursor_pos += 1;
        }
    }

    pub fn action_insert_at_cursor(&mut self, ch: char) {
        self.input_buffer.insert(self.input_cursor_pos, ch);
        self.input_cursor_pos += ch.len_utf8();
    }

    pub fn action_backspace_at_cursor(&mut self) {
        if self.input_cursor_pos > 0
            && let Some((previous, _)) = self.input_buffer[..self.input_cursor_pos]
                .char_indices()
                .next_back()
        {
            self.input_buffer.remove(previous);
            self.input_cursor_pos = previous;
        }
    }

    pub fn action_delete_at_cursor(&mut self) {
        if self.input_cursor_pos < self.input_buffer.len() {
            let off = self.input_cursor_pos;
            let mut ch_iter = self.input_buffer[off..].char_indices();
            if let Some((_, _)) = ch_iter.next() {
                self.input_buffer.remove(off);
            }
        }
    }

    pub fn action_cursor_home(&mut self) {
        self.input_cursor_pos = 0;
    }

    pub fn action_cursor_end(&mut self) {
        self.input_cursor_pos = self.input_buffer.len();
    }
}

impl Default for ActionContext {
    fn default() -> Self {
        Self {
            mode: ActionMode::Normal,
            input_buffer: String::new(),
            input_cursor_pos: 0,
            clipboard: None,
            is_cut: false,
            find: FindState::default(),
        }
    }
}

/// Tracks the state of an ongoing fuzzy find operation.
///
/// It includes the cached results, request ID, debounce timer, last query,
/// selected result index, and cancellation token.
#[derive(Default)]
pub struct FindState {
    cache: Vec<FindResult>,
    request_id: u64,
    debounce: Option<Instant>,
    last_query: String,
    selected: usize,
    cancel: Option<Arc<AtomicBool>>,
}

impl FindState {
    // Getters / Accessors

    fn results(&self) -> &[FindResult] {
        &self.cache
    }

    fn request_id(&self) -> u64 {
        self.request_id
    }

    fn selected(&self) -> usize {
        self.selected
    }

    // Find functions

    fn cancel_current(&mut self) {
        if let Some(token) = self.cancel.take() {
            token.store(true, Ordering::Relaxed);
        }
    }

    fn set_results(&mut self, results: Vec<FindResult>) {
        self.cache = results;
        self.selected = 0;
    }

    fn set_cancel(&mut self, token: Arc<AtomicBool>) {
        self.cancel = Some(token);
    }

    fn clear_results(&mut self) {
        self.cache.clear();
    }

    fn prepare_new_request(&mut self) -> u64 {
        self.request_id = self.request_id.wrapping_add(1);
        self.request_id
    }

    fn set_debounce(&mut self, delay: Duration) {
        self.debounce = Some(Instant::now() + delay);
    }

    fn take_query(&mut self, current_query: &str) -> Option<String> {
        let until = self.debounce?;
        if Instant::now() < until {
            return None;
        }

        self.debounce = None;
        if current_query == self.last_query {
            self.last_query.clear();
            return None;
        }

        self.last_query.clear();
        self.last_query.push_str(current_query);
        Some(current_query.to_string())
    }

    fn reset(&mut self) {
        self.cancel_current();
        self.cache.clear();
        self.debounce = None;
        self.last_query.clear();
    }

    pub fn select_next(&mut self) {
        if self.selected + 1 < self.cache.len() {
            self.selected += 1;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn reset_selected(&mut self) {
        self.selected = 0;
    }
}
