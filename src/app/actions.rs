use crate::app::nav::NavState;
use crate::keymap::FileAction;
use crate::worker::{FileOperation, WorkerTask};
use crossbeam_channel::Sender;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Clone, PartialEq)]
pub enum ActionMode {
    Normal,
    Input { mode: InputMode, prompt: String },
    Confim { prompt: String, action: FileAction },
}

#[derive(Clone, Copy, PartialEq)]
pub enum InputMode {
    Rename,
    NewFile,
    NewFolder,
    Filter,
    ConfirmDelete,
}

pub struct ActionContext {
    pub mode: ActionMode,
    pub input_buffer: String,
    pub clipboard: Option<HashSet<PathBuf>>,
    pub is_cut: bool,
}

impl ActionContext {
    pub fn new() -> Self {
        Self {
            mode: ActionMode::Normal,
            input_buffer: String::new(),
            clipboard: None,
            is_cut: false,
        }
    }

    pub fn is_input_mode(&self) -> bool {
        matches!(self.mode, ActionMode::Input { .. })
    }

    pub fn enter_mode(&mut self, mode: ActionMode, initial_value: String) {
        self.mode = mode;
        self.input_buffer = initial_value;
    }

    pub fn exit_mode(&mut self) {
        self.mode = ActionMode::Normal;
        self.input_buffer.clear();
    }

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

    pub fn action_copy(&mut self, nav: &NavState, is_cut: bool) {
        let targets = nav.get_action_targets();
        if !targets.is_empty() {
            self.clipboard = Some(targets);
            self.is_cut = is_cut;
        }
    }

    pub fn action_paste(&mut self, nav: &mut NavState, worker_tx: &Sender<WorkerTask>) {
        if let Some(source) = &self.clipboard {
            let first_file_name = source
                .iter()
                .next()
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
}
