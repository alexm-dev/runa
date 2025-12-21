use crate::config::Config;
use crate::file_manager::FileEntry;
use crate::keymap::{Action, Keymap};
use crate::utils::open_in_editor;
use crate::worker::{WorkerResponse, WorkerTask, start_worker};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

pub enum KeypressResult {
    Continue,
    Quit,
    OpenedEditor,
}

/// Application state of the file browser
/// The active app state
/// Stores the directory entires, selections/dir_positions, and workser respoens
pub struct AppState<'a> {
    current_dir: std::path::PathBuf,
    entries: Vec<FileEntry>,
    selected: usize,
    config: &'a Config,
    dir_positions: HashMap<std::path::PathBuf, usize>,
    keymap: Keymap,
    worker_tx: Sender<WorkerTask>,
    response_rx: Receiver<WorkerResponse>,

    last_parent_path: Option<PathBuf>,
    last_input_time: Instant,
    preview_pending: bool,

    pub is_loading: bool,
    pub preview_selected: usize,
    pub preview_content: Vec<String>,

    pub preview_request_id: u64,
    pub dir_request_id: u64,
    pub parent_request_id: u64,

    pub current_preview_path: Option<PathBuf>,
    pub parent_selected: Option<usize>,
    pub parent_content: Vec<String>,
}

impl<'a> AppState<'a> {
    pub fn new(config: &'a Config) -> std::io::Result<Self> {
        let (worker_tx, task_rx) = unbounded::<WorkerTask>();
        let (res_tx, response_rx) = unbounded::<WorkerResponse>();
        let current_dir = std::env::current_dir()?;

        // Start the engine
        start_worker(task_rx, res_tx);

        let mut app = Self {
            current_dir,
            entries: Vec::new(),
            selected: 0,
            config,
            dir_positions: HashMap::new(),
            keymap: Keymap::from_config(config),
            worker_tx,
            response_rx,
            last_parent_path: None,
            last_input_time: Instant::now(),
            preview_pending: false,
            is_loading: false,
            preview_selected: 0,
            preview_content: vec!["Loading...".into()],
            preview_request_id: 0,
            dir_request_id: 0,
            parent_request_id: 0,
            current_preview_path: None,
            parent_selected: None,
            parent_content: vec!["Loading...".into()],
        };

        app.request_dir_load(None);
        Ok(app)
    }

    fn request_dir_load(&mut self, focus: Option<std::ffi::OsString>) {
        self.is_loading = true;
        self.dir_request_id += 1;
        let request_id = self.dir_request_id;
        let always_show = Arc::clone(self.config.always_show());
        let _ = self.worker_tx.send(WorkerTask::LoadDirectory {
            path: self.current_dir.clone(),
            focus,
            dirs_first: self.config.dirs_first(),
            show_hidden: self.config.show_hidden(),
            show_system: self.config.show_system(),
            case_insensitive: self.config.case_insensitive(),
            always_show,
            request_id,
        });
    }

    pub fn tick(&mut self) -> bool {
        let mut changed = false;

        // debounce timing
        if self.preview_pending && self.last_input_time.elapsed().as_millis() > 75 {
            self.request_preview();
            self.preview_pending = false;
            changed = true;
        }

        while let Ok(response) = self.response_rx.try_recv() {
            changed = true;
            match response {
                WorkerResponse::DirectoryLoaded {
                    path,
                    entries,
                    focus,
                    request_id,
                } => {
                    if path == self.current_dir && request_id == self.dir_request_id {
                        self.entries = entries;
                        self.is_loading = false;
                        self.selected = if let Some(target) = focus {
                            self.entries
                                .iter()
                                .position(|e| e.name() == &target)
                                .unwrap_or(0)
                        } else {
                            self.dir_positions
                                .get(&self.current_dir)
                                .cloned()
                                .unwrap_or(0)
                        };
                        self.selected = self.selected.min(self.entries.len().saturating_sub(1));

                        self.request_preview();
                        self.preview_pending = false;
                        self.request_parent_content();
                    } else if Some(path.as_path()) == self.current_dir.parent() {
                        self.parent_content = entries
                            .iter()
                            .map(|e| e.display_name().to_string())
                            .collect();

                        let name = self.current_dir.file_name().map(|n| n.to_string_lossy());
                        self.parent_selected = name.and_then(|n| {
                            self.parent_content.iter().position(|line| {
                                line == n.as_ref() || line.strip_suffix('/') == Some(n.as_ref())
                            })
                        })
                    } else if Some(&path) == self.current_preview_path.as_ref()
                        && request_id == self.preview_request_id
                    {
                        self.preview_content = entries
                            .iter()
                            .map(|e| e.display_name().to_string())
                            .collect();

                        self.preview_selected = self.dir_positions.get(&path).cloned().unwrap_or(0);

                        self.preview_selected =
                            self.preview_selected.min(entries.len().saturating_sub(1))
                    }
                }
                WorkerResponse::PreviewLoaded { lines, request_id } => {
                    if request_id == self.preview_request_id {
                        self.preview_content.clear();
                        self.preview_content.extend(lines);
                    }
                }
                WorkerResponse::Error(e) => {
                    self.preview_content = vec![e];
                }
            }
        }
        changed
    }

    fn request_preview(&mut self) {
        let entry = match self.entries.get(self.selected) {
            Some(e) => e,
            None => return,
        };

        let path = self.current_dir.join(entry.name());

        self.preview_request_id += 1;
        let req_id = self.preview_request_id;
        self.current_preview_path = Some(path.clone());
        self.preview_selected = 0;

        if entry.is_dir() {
            let _ = self.worker_tx.send(WorkerTask::LoadDirectory {
                path,
                focus: None,
                dirs_first: self.config.dirs_first(),
                show_hidden: self.config.show_hidden(),
                show_system: self.config.show_system(),
                case_insensitive: self.config.case_insensitive(),
                always_show: Arc::clone(self.config.always_show()),
                request_id: req_id,
            });
        } else {
            let _ = self.worker_tx.send(WorkerTask::LoadPreview {
                path,
                max_lines: 50,
                request_id: req_id,
            });
        }
    }

    fn request_parent_content(&mut self) {
        let parent_path = match self.current_dir.parent() {
            Some(p) => p.to_path_buf(),
            None => {
                self.parent_content = vec!["/".into()];
                self.last_parent_path = None;
                return;
            }
        };

        if Some(&parent_path) == self.last_parent_path.as_ref() {
            return;
        }

        self.last_parent_path = Some(parent_path.clone());
        self.parent_request_id += 1;
        let request_id = self.parent_request_id;

        let always_show = Arc::clone(self.config.always_show());
        let _ = self.worker_tx.send(WorkerTask::LoadDirectory {
            path: parent_path,
            focus: None,
            dirs_first: self.config.dirs_first(),
            show_hidden: self.config.show_hidden(),
            show_system: self.config.show_system(),
            case_insensitive: self.config.case_insensitive(),
            always_show,
            request_id,
        });
    }

    pub fn current_dir(&self) -> &std::path::Path {
        &self.current_dir
    }

    pub fn visible_entries(&self) -> &[FileEntry] {
        &self.entries
    }

    pub fn has_visible_entries(&self) -> bool {
        !self.entries.is_empty()
    }

    pub fn visible_selected(&self) -> Option<usize> {
        if self.entries.is_empty() {
            None
        } else {
            Some(self.selected)
        }
    }

    pub fn config(&self) -> &Config {
        self.config
    }

    fn save_current_pos(&mut self) {
        self.dir_positions
            .insert(self.current_dir.clone(), self.selected);
    }

    pub fn handle_keypress(&mut self, key: &str) -> KeypressResult {
        match self.keymap.lookup(key) {
            Some(Action::GoParent) => self.handle_go_parent(),
            Some(Action::GoIntoDir) => self.handle_go_into_dir(),
            Some(Action::GoUp) => self.handle_go_up(),
            Some(Action::GoDown) => self.handle_go_down(),
            Some(Action::Open) => self.handle_open_file(),
            Some(Action::Quit) => self.handle_quit(),
            _ => KeypressResult::Continue,
        }
    }

    fn handle_go_parent(&mut self) -> KeypressResult {
        if let Some(parent) = self.current_dir.parent() {
            let parent_path = parent.to_path_buf();
            let exited_dir_name = self.current_dir.file_name().map(|n| n.to_os_string());
            self.save_current_pos();
            self.current_dir = parent_path;
            self.request_dir_load(exited_dir_name);
        }
        KeypressResult::Continue
    }

    fn handle_go_into_dir(&mut self) -> KeypressResult {
        if let Some(entry) = self.entries.get(self.selected)
            && entry.is_dir()
        {
            let dir_name = entry.name().clone();
            self.save_current_pos();
            self.current_dir = self.current_dir.join(&dir_name);
            self.entries.clear();
            self.selected = 0;
            self.request_dir_load(None);
        }
        KeypressResult::Continue
    }

    fn handle_go_up(&mut self) -> KeypressResult {
        if self.selected > 0 {
            self.selected -= 1;
            self.preview_pending = true;
            self.last_input_time = Instant::now();
        }
        KeypressResult::Continue
    }

    fn handle_go_down(&mut self) -> KeypressResult {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
            self.preview_pending = true;
            self.last_input_time = Instant::now();
        }
        KeypressResult::Continue
    }

    fn handle_open_file(&mut self) -> KeypressResult {
        if let Some(entry) = self.entries.get(self.selected) {
            let path = self.current_dir.join(entry.name());
            if let Err(e) = open_in_editor(self.config.editor(), &path) {
                eprintln!("Error opening editor: {}", e);
            }
            return KeypressResult::OpenedEditor;
        }
        KeypressResult::Continue
    }

    fn handle_quit(&self) -> KeypressResult {
        KeypressResult::Quit
    }
}
