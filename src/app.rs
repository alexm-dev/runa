//! Application State and main controller module for runa.
//!
//! This module defines the overall [AppState] struct, which holds all major application
//! information and passes it to relevant UI/Terminal functions
//! - Configuration (loaded from config files)
//! - Pane view models for navigation, preview and parent states.
//! - Action context for relevant inputs
//! - Current layout metrics
//! - Communication with worker threads via crossbeam_channel
//! - Notification and message handling
//!
//! This module coordinates user input processing, keybindings, state mutation,
//! pane switching and communication with worder tasks
//!
//! This is the primary context/state object passed to most UI/Terminal event logic.

pub mod actions;
mod handlers;
mod nav;
mod parent;
mod preview;

pub use nav::NavState;
pub use parent::ParentState;
pub use preview::{PreviewData, PreviewState};

use crate::app::actions::ActionContext;
use crate::config::Config;
use crate::keymap::{Action, Keymap, SystemAction};
use crate::worker::{WorkerResponse, WorkerTask, start_worker};
use crossbeam_channel::{Receiver, Sender, unbounded};
use crossterm::event::KeyEvent;
use std::sync::Arc;
use std::time::Instant;

/// Enumeration for each individual keypress result processed.
///
/// Is used to process action logic correctly.
pub enum KeypressResult {
    Continue,
    Consumed,
    Quit,
    OpenedEditor,
}

/// Enumeration which holds the metrics of the layout of the TUI
#[derive(Debug, Clone, Copy)]
pub struct LayoutMetrics {
    pub parent_width: usize,
    pub main_width: usize,
    pub preview_width: usize,
    pub preview_height: usize,
}

impl Default for LayoutMetrics {
    fn default() -> Self {
        Self {
            parent_width: 20,
            main_width: 40,
            preview_width: 40,
            preview_height: 50,
        }
    }
}

/// Main struct which holds the central Application state of runa
///
/// AppState holds all the persisten state for the application while it is running
///
/// Includes:
/// - References to configuration settings and the keymaps.
/// - Models for navigation, actions, file previews, and parent directory pane
/// - Live layout information
/// - crossbeam channels for communication with background worker threads
/// - Notification timing and loading indicators
///
/// Functions are provided for the core event loop, input handling, file navigationm
/// worker requests and Notification management.
pub struct AppState<'a> {
    config: &'a Config,
    keymap: Keymap,

    metrics: LayoutMetrics,

    nav: NavState,
    actions: ActionContext,
    preview: PreviewState,
    parent: ParentState,

    worker_tx: Sender<WorkerTask>,
    response_rx: Receiver<WorkerResponse>,
    is_loading: bool,

    notification_time: Option<Instant>,
}

impl<'a> AppState<'a> {
    pub fn new(config: &'a Config) -> std::io::Result<Self> {
        let (worker_tx, task_rx) = unbounded::<WorkerTask>();
        let (res_tx, response_rx) = unbounded::<WorkerResponse>();
        let current_dir = std::env::current_dir()?;

        start_worker(task_rx, res_tx);

        let mut app = Self {
            config,
            keymap: Keymap::from_config(config),
            metrics: LayoutMetrics::default(),
            nav: NavState::new(current_dir),
            actions: ActionContext::default(),
            preview: PreviewState::default(),
            parent: ParentState::default(),
            worker_tx,
            response_rx,
            is_loading: false,
            notification_time: None,
        };

        app.request_dir_load(None);
        app.request_parent_content();
        Ok(app)
    }

    // Getters/ accessors

    pub fn config(&self) -> &Config {
        self.config
    }

    pub fn metrics_mut(&mut self) -> &mut LayoutMetrics {
        &mut self.metrics
    }

    pub fn nav(&self) -> &NavState {
        &self.nav
    }

    pub fn actions(&self) -> &ActionContext {
        &self.actions
    }

    pub fn preview(&self) -> &PreviewState {
        &self.preview
    }

    pub fn parent(&self) -> &ParentState {
        &self.parent
    }

    pub fn notification_time(&self) -> &Option<Instant> {
        &self.notification_time
    }

    /// Entry functions

    pub fn visible_selected(&self) -> Option<usize> {
        if self.nav.entries().is_empty() {
            None
        } else {
            Some(self.nav.selected_idx())
        }
    }
    pub fn has_visible_entries(&self) -> bool {
        !self.nav.entries().is_empty()
    }

    /// The heart of the app: updates state and handles worker messages
    ///
    /// Is used by the main event loop to update the application state.
    /// Returns a bool to determine if the AppState needs reloading
    /// and sets it to true if a WorkerResponse was made or if a preview should be triggered.
    pub fn tick(&mut self) -> bool {
        let mut changed = false;

        // Handle preview debounc
        if self.preview.should_trigger() {
            self.request_preview();
            changed = true;
        }

        // Process worker response
        while let Ok(response) = self.response_rx.try_recv() {
            changed = true;
            match response {
                WorkerResponse::DirectoryLoaded {
                    path,
                    entries,
                    focus,
                    request_id,
                } => {
                    // only update nav if BOTH the ID and path match.
                    if request_id == self.nav.request_id() && path == self.nav.current_dir() {
                        self.nav.update_from_worker(path, entries, focus);
                        self.is_loading = false;
                        self.request_preview();
                        self.request_parent_content();
                    }
                    // PREVIEW CHECK: Must match the current preview request
                    else if request_id == self.preview.request_id() {
                        self.preview.update_from_entries(entries, request_id);
                        if let Some(entry) = self.nav.selected_entry() {
                            let path = self.nav.current_dir().join(entry.name());
                            if let Some(&cached_pos) = self.nav.get_position().get(&path) {
                                self.preview.set_selected_idx(cached_pos);
                            } else {
                                self.preview.set_selected_idx(0);
                            }
                        }
                    }
                    // PARENT CHECK: Must match the current parent request
                    else if request_id == self.parent.request_id() {
                        let current_name = self
                            .nav
                            .current_dir()
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();

                        self.parent
                            .update_from_entries(entries, &current_name, request_id);
                    }
                }
                WorkerResponse::PreviewLoaded { lines, request_id } => {
                    self.preview.update_content(lines, request_id);
                }

                WorkerResponse::OperationComplete {
                    message: _,
                    request_id: _,
                    need_reload,
                    focus,
                } => {
                    if need_reload {
                        self.request_dir_load(focus);
                        self.request_parent_content();
                    }
                }

                WorkerResponse::Error(e) => {
                    self.preview.set_error(e);
                }
            }
        }
        changed
    }

    /// Central key handlers
    ///
    /// Coordinates the action and handler module functions.
    pub fn handle_keypress(&mut self, key: KeyEvent) -> KeypressResult {
        if self.actions.is_input_mode() {
            return self.handle_input_mode(key);
        }

        if let Some(action) = self.keymap.lookup(key) {
            match action {
                Action::System(SystemAction::Quit) => return KeypressResult::Quit,
                Action::Nav(nav_act) => return self.handle_nav_action(nav_act),
                Action::File(file_act) => return self.handle_file_action(file_act),
            }
        }

        KeypressResult::Continue
    }

    /// Worker requests functions for directory loading, preview and parent pane content

    pub fn request_dir_load(&mut self, focus: Option<std::ffi::OsString>) {
        self.is_loading = true;
        let request_id = self.nav.prepare_new_request();
        let _ = self.worker_tx.send(WorkerTask::LoadDirectory {
            path: self.nav.current_dir().to_path_buf(),
            focus,
            dirs_first: self.config.dirs_first(),
            show_hidden: self.config.show_hidden(),
            show_system: self.config.show_system(),
            case_insensitive: self.config.case_insensitive(),
            always_show: Arc::clone(self.config.always_show()),
            pane_width: self.metrics.main_width,
            request_id,
        });
    }

    pub fn request_preview(&mut self) {
        if let Some(entry) = self.nav.selected_shown_entry() {
            let path = self.nav.current_dir().join(entry.name());
            let req_id = self.preview.prepare_new_request(path.clone());

            if entry.is_dir() {
                let _ = self.worker_tx.send(WorkerTask::LoadDirectory {
                    path,
                    focus: None,
                    dirs_first: self.config.dirs_first(),
                    show_hidden: self.config.show_hidden(),
                    show_system: self.config.show_system(),
                    case_insensitive: self.config.case_insensitive(),
                    always_show: Arc::clone(self.config.always_show()),
                    pane_width: self.metrics.preview_width,
                    request_id: req_id,
                });
            } else {
                let _ = self.worker_tx.send(WorkerTask::LoadPreview {
                    path,
                    max_lines: self.metrics.preview_height,
                    pane_width: self.metrics.preview_width,
                    request_id: req_id,
                });
            }
        }
    }

    pub fn request_parent_content(&mut self) {
        if let Some(parent_path) = self.nav.current_dir().parent() {
            let parent_path_buf = parent_path.to_path_buf();

            if self.parent.should_request(&parent_path_buf) {
                let req_id = self.parent.prepare_new_request(parent_path_buf.clone());

                let _ = self.worker_tx.send(WorkerTask::LoadDirectory {
                    path: parent_path_buf,
                    focus: None,
                    dirs_first: self.config.dirs_first(),
                    show_hidden: self.config.show_hidden(),
                    show_system: self.config.show_system(),
                    case_insensitive: self.config.case_insensitive(),
                    always_show: Arc::clone(self.config.always_show()),
                    pane_width: self.metrics.parent_width,
                    request_id: req_id,
                });
            }
        } else {
            // at root.
            self.parent.clear();
        }
    }
}
