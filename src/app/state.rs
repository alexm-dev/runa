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

use crate::app::actions::{ActionContext, ActionMode, InputMode};
use crate::app::keymap::{Action, Keymap, SystemAction};
use crate::app::{NavState, ParentState, PreviewState};
use crate::config::Config;
use crate::core::worker::{WorkerResponse, WorkerTask, Workers};
use crate::ui::overlays::{Overlay, OverlayStack};

use crossterm::event::KeyEvent;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
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
/// - UI overlay for a seamless widet rendering
///
/// Functions are provided for the core event loop, input handling, file navigationm
/// worker requests and Notification management.
pub struct AppState<'a> {
    pub(super) config: &'a Config,
    pub(super) keymap: Keymap,

    pub(super) metrics: LayoutMetrics,

    pub(super) nav: NavState,
    pub(super) actions: ActionContext,
    pub(super) preview: PreviewState,
    pub(super) parent: ParentState,

    pub(super) workers: Workers,
    pub(super) is_loading: bool,

    pub(super) notification_time: Option<Instant>,
    pub(super) overlays: OverlayStack,
}

impl<'a> AppState<'a> {
    pub fn new(config: &'a Config) -> std::io::Result<Self> {
        let workers = Workers::spawn();
        let current_dir = std::env::current_dir()?;

        let mut app = Self {
            config,
            keymap: Keymap::from_config(config),
            metrics: LayoutMetrics::default(),
            nav: NavState::new(current_dir),
            actions: ActionContext::default(),
            preview: PreviewState::default(),
            parent: ParentState::default(),
            workers,
            is_loading: false,
            notification_time: None,
            overlays: OverlayStack::new(),
        };

        app.request_dir_load(None);
        app.request_parent_content();
        Ok(app)
    }

    // Getters/ accessors

    pub fn config(&self) -> &Config {
        self.config
    }

    pub fn metrics(&self) -> &LayoutMetrics {
        &self.metrics
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

    pub fn overlays(&self) -> &OverlayStack {
        &self.overlays
    }

    pub fn overlays_mut(&mut self) -> &mut OverlayStack {
        &mut self.overlays
    }

    pub fn set_notification_time(&mut self, t: Option<Instant>) {
        self.notification_time = t;
    }

    // Entry functions

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

        if let Some(expiry) = self.notification_time
            && Instant::now() >= expiry
        {
            self.notification_time = None;

            self.overlays_mut()
                .retain(|o| !matches!(o, Overlay::Message { .. }));

            changed = true;
        }

        // Handle preview debounc
        if self.preview.should_trigger() {
            self.request_preview();
            changed = true;
        }

        let current_selection_path = self
            .nav
            .selected_entry()
            .map(|entry| self.nav.current_dir().join(entry.name()));

        // Find handling with debounce
        if let ActionMode::Input {
            mode: InputMode::Find,
            ..
        } = self.actions.mode()
            && let Some(query) = self.actions.take_query()
        {
            if query.is_empty() {
                self.actions.clear_find_results();
            } else {
                self.request_find(query);
            }
            changed = true;
        }

        // Process worker response
        while let Ok(response) = self.workers.response_rx().try_recv() {
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
                        self.refresh_show_info_if_open();
                    }
                    // PREVIEW CHECK: Must match the current preview request
                    else if request_id == self.preview.request_id() {
                        if current_selection_path.as_ref() == Some(&path) {
                            self.preview.update_from_entries(entries, request_id);

                            let pos = current_selection_path
                                .as_ref()
                                .and_then(|p| self.nav.get_position().get(p))
                                .copied()
                                .unwrap_or(0);

                            self.preview.set_selected_idx(pos);
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
                    if request_id == self.preview.request_id() {
                        self.preview.update_content(lines, request_id);
                    }
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

                WorkerResponse::FindResults {
                    base_dir,
                    results,
                    request_id,
                } => {
                    if base_dir == self.nav.current_dir()
                        && request_id == self.actions.find_request_id()
                    {
                        self.actions.set_find_results(results);
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

    // Worker requests functions for directory loading, preview and parent pane content

    pub fn request_dir_load(&mut self, focus: Option<std::ffi::OsString>) {
        self.is_loading = true;
        let request_id = self.nav.prepare_new_request();
        let _ = self.workers.io_tx().send(WorkerTask::LoadDirectory {
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
            // Set the directory generation for the preview to the request_id for WorkerResponse

            if entry.is_dir() {
                let _ = self.workers.io_tx().send(WorkerTask::LoadDirectory {
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
                let preview_options = self.config.display().preview_options();
                let preview_method = preview_options.method().clone();
                let bat_args = self.config.bat_args_for_preview(self.metrics.preview_width);
                let _ = self.workers.preview_tx().send(WorkerTask::LoadPreview {
                    path,
                    max_lines: self.metrics.preview_height,
                    pane_width: self.metrics.preview_width,
                    preview_method,
                    args: bat_args,
                    request_id: req_id,
                });
            }
        } else {
            self.preview.clear();
        }
    }

    pub fn request_parent_content(&mut self) {
        if let Some(parent_path) = self.nav.current_dir().parent() {
            let parent_path_buf = parent_path.to_path_buf();

            self.parent.prepare_update();

            if self.parent.should_request(&parent_path_buf) {
                let req_id = self.parent.prepare_new_request(parent_path_buf.clone());

                let _ = self.workers.io_tx().send(WorkerTask::LoadDirectory {
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

    pub fn request_find(&mut self, query: String) {
        self.actions.cancel_find();

        let request_id = self.actions.prepare_new_find_request();
        let cancel_token = Arc::new(AtomicBool::new(false));

        self.actions
            .set_cancel_find_token(Arc::clone(&cancel_token));

        let _ = self.workers.find_tx().send(WorkerTask::FindRecursive {
            base_dir: self.nav.current_dir().to_path_buf(),
            query,
            max_results: self.config().max_find_results(),
            request_id,
            cancel: cancel_token,
        });
    }
}
