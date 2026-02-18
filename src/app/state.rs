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
use crate::app::keymap::{Action, Keymap};
use crate::app::{NavState, ParentState, PreviewState};
use crate::config::Config;
use crate::core::worker::{WorkerResponse, WorkerTask, Workers};
use crate::ui::overlays::{Overlay, OverlayStack};

use crossterm::event::KeyEvent;

use std::ffi::OsString;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Enumeration for each individual keypress result processed.
///
/// Is used to process action logic correctly.
pub(crate) enum KeypressResult {
    Continue,
    Consumed,
    Quit,
    OpenedEditor,
    Recovered,
}

/// Enumeration which holds the metrics of the layout of the TUI
#[derive(Debug, Clone, Copy)]
pub(crate) struct LayoutMetrics {
    pub(crate) parent_width: usize,
    pub(crate) main_width: usize,
    pub(crate) preview_width: usize,
    pub(crate) preview_height: usize,
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
pub(crate) struct AppState<'a> {
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
    pub(super) worker_time: Option<Instant>,
    pub(super) overlays: OverlayStack,
}

impl<'a> AppState<'a> {
    pub(crate) fn new(config: &'a Config) -> std::io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        Self::from_dir(config, &current_dir)
    }

    pub(crate) fn from_dir(config: &'a Config, initial_path: &Path) -> std::io::Result<Self> {
        let workers = Workers::spawn();
        let current_dir = if initial_path.exists() && initial_path.is_dir() {
            initial_path.to_path_buf()
        } else {
            std::env::current_dir()?
        };

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
            worker_time: None,
            overlays: OverlayStack::new(),
        };

        app.request_dir_load(None);
        app.request_parent_content();
        Ok(app)
    }

    // Getters/ accessors

    #[inline]
    pub(crate) fn config(&self) -> &Config {
        self.config
    }

    #[inline]
    pub(crate) fn nav(&self) -> &NavState {
        &self.nav
    }

    #[inline]
    pub(crate) fn actions(&self) -> &ActionContext {
        &self.actions
    }

    #[inline]
    pub(crate) fn preview(&self) -> &PreviewState {
        &self.preview
    }

    #[inline]
    pub(crate) fn parent(&self) -> &ParentState {
        &self.parent
    }

    #[inline]
    pub(crate) fn workers(&self) -> &Workers {
        &self.workers
    }

    #[inline]
    pub(crate) fn is_loading(&self) -> bool {
        self.is_loading
    }

    #[inline]
    pub(crate) fn notification_time(&self) -> &Option<Instant> {
        &self.notification_time
    }

    #[inline]
    pub(crate) fn worker_time(&self) -> &Option<Instant> {
        &self.worker_time
    }

    #[inline]
    pub(crate) fn overlays(&self) -> &OverlayStack {
        &self.overlays
    }

    #[inline]
    pub(crate) fn overlays_mut(&mut self) -> &mut OverlayStack {
        &mut self.overlays
    }

    // Entry functions

    pub(crate) fn visible_selected(&self) -> Option<usize> {
        if self.nav.entries().is_empty() {
            None
        } else {
            Some(self.nav.selected_idx())
        }
    }
    pub(crate) fn has_visible_entries(&self) -> bool {
        !self.nav.entries().is_empty()
    }

    /// Metrics updater for LayoutMetrics to request_preview new preview after old metrics
    pub(crate) fn update_layout_metrics(&mut self, metrics: LayoutMetrics) {
        let old_width = self.metrics.preview_width;
        let old_height = self.metrics.preview_height;

        self.metrics = metrics;

        if old_width != self.metrics.preview_width || old_height != self.metrics.preview_height {
            if self.preview.data().is_empty() {
                self.request_preview();
            } else {
                self.preview.mark_pending();
            }
        }
    }

    /// The heart of the app: updates state and handles worker messages
    ///
    /// Is used by the main event loop to update the application state.
    /// Returns a bool to determine if the AppState needs reloading
    /// and sets it to true if a WorkerResponse was made or if a preview should be triggered.
    pub(crate) fn tick(&mut self) -> bool {
        let mut changed = false;

        if let Some(expiry) = self.notification_time
            && Instant::now() >= expiry
        {
            self.notification_time = None;

            self.overlays_mut()
                .retain(|o| !matches!(o, Overlay::Message { .. }));

            changed = true;
        }

        let active = self.workers.active().load(Ordering::Relaxed);
        if active > 0 {
            let start = *self.worker_time.get_or_insert_with(Instant::now);

            if start.elapsed() >= Duration::from_millis(200) {
                changed = true;
            }
        } else if self.worker_time.is_some() {
            self.worker_time = None;
            changed = true;
        }

        let prefix_recognizer = self.actions.prefix_recognizer_mut();
        if prefix_recognizer.is_g_state() && prefix_recognizer.expired() {
            prefix_recognizer.cancel();
            self.hide_prefix_help();
            changed = true;
        }

        // Handle preview debounc
        if self.preview.should_trigger() {
            self.request_preview();
            changed = true;
        }

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
                        self.request_parent_content();
                        self.request_preview();
                        self.refresh_show_info_if_open();
                        continue;
                    }
                    // PREVIEW CHECK: Must match the current preview request
                    if request_id == self.preview.request_id()
                        && let Some(entry) = self.nav.selected_entry()
                        && path.parent() == Some(self.nav.current_dir())
                        && path.file_name() == Some(entry.name())
                    {
                        self.preview.update_from_entries(entries, request_id);

                        let sel_path = self.nav.current_dir().join(entry.name());
                        let pos = self.nav.get_position().get(&sel_path).copied().unwrap_or(0);

                        self.preview.set_selected_idx(pos);
                        continue;
                    }
                    // PARENT CHECK: Must match the current parent request
                    if request_id == self.parent.request_id() {
                        let expected_parent = self.nav.current_dir().parent();
                        if expected_parent == Some(path.as_path()) {
                            let current_name = self
                                .nav
                                .current_dir()
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();

                            self.parent.update_from_entries(
                                entries,
                                &current_name,
                                request_id,
                                &path,
                            );
                            continue;
                        }
                    }
                }
                WorkerResponse::PreviewLoaded { lines, request_id } => {
                    if request_id == self.preview.request_id() {
                        self.preview.update_content(lines, request_id);
                    }
                }

                WorkerResponse::OperationComplete { need_reload, focus } => {
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

                WorkerResponse::Error(e, request_id) => {
                    self.is_loading = false;
                    match request_id {
                        Some(id) if id == self.preview.request_id() => {
                            self.preview.set_error(e);
                        }
                        _ => {
                            self.push_overlay_message(e.to_string(), Duration::from_secs(7));
                        }
                    }
                }
            }
        }
        changed
    }

    /// Central key handlers
    ///
    /// Coordinates the action and handler module functions.
    pub(crate) fn handle_keypress(&mut self, key: KeyEvent) -> KeypressResult {
        if self.actions.is_input_mode() {
            return self.handle_input_mode(key);
        }

        if let Some(res) = self.handle_esc_close_overlays(&key) {
            return res;
        }

        if let Some(res) = self.handle_prefix_dispatch(&key) {
            return res;
        }

        if let Some(action) = self.keymap.lookup(key) {
            match action {
                Action::System(sys_act) => return self.handle_sys_action(sys_act),
                Action::Nav(nav_act) => return self.handle_nav_action(nav_act),
                Action::File(file_act) => return self.handle_file_action(file_act),
            }
        }

        KeypressResult::Continue
    }

    // Worker requests functions for directory loading, preview and parent pane content

    /// Requests a directory load for the current navigation directory
    pub(crate) fn request_dir_load(&mut self, focus: Option<std::ffi::OsString>) {
        self.is_loading = true;
        let request_id = self.nav.prepare_new_request();
        let _ = self
            .workers
            .nav_io_tx()
            .try_send(WorkerTask::LoadDirectory {
                path: self.nav.current_dir().to_path_buf(),
                focus,
                dirs_first: self.config.general().dirs_first(),
                show_hidden: self.config.general().show_hidden(),
                show_symlink: self.config.general().show_symlink(),
                show_system: self.config.general().show_system(),
                case_insensitive: self.config.general().case_insensitive(),
                always_show: Arc::clone(self.config.general().always_show()),
                request_id,
            });
    }

    /// Requests a preview load for the currently selected entry in the navigation pane
    pub(crate) fn request_preview(&mut self) {
        if let Some(entry) = self.nav.selected_shown_entry() {
            let path = self.nav.current_dir().join(entry.name());
            let req_id = self.preview.prepare_new_request(path.clone());

            if entry.is_dir() || entry.is_symlink() {
                let _ = self
                    .workers
                    .preview_io_tx()
                    .try_send(WorkerTask::LoadDirectory {
                        path,
                        focus: None,
                        dirs_first: self.config.general().dirs_first(),
                        show_hidden: self.config.general().show_hidden(),
                        show_symlink: self.config.general().show_symlink(),
                        show_system: self.config.general().show_system(),
                        case_insensitive: self.config.general().case_insensitive(),
                        always_show: Arc::clone(self.config.general().always_show()),
                        request_id: req_id,
                    });
            } else {
                let preview_options = self.config.display().preview_options();
                let preview_method = preview_options.method().clone();
                let bat_args = self
                    .config
                    .bat_args_for_preview(self.metrics.preview_width)
                    .into_iter()
                    .map(OsString::from)
                    .collect();
                let _ = self
                    .workers
                    .preview_file_tx()
                    .try_send(WorkerTask::LoadPreview {
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

    /// Requests loading of the parent directory content for the parent pane
    pub(crate) fn request_parent_content(&mut self) {
        let Some(parent_path) = self.nav.current_dir().parent() else {
            self.parent.clear();
            return;
        };

        if self.parent.is_cached(parent_path) {
            return;
        }

        let parent_path_buf = parent_path.to_path_buf();
        let req_id = self.parent.prepare_new_request(&parent_path_buf);
        let _ = self
            .workers
            .parent_io_tx()
            .try_send(WorkerTask::LoadDirectory {
                path: parent_path_buf,
                focus: None,
                dirs_first: self.config.general().dirs_first(),
                show_hidden: self.config.general().show_hidden(),
                show_symlink: self.config.general().show_symlink(),
                show_system: self.config.general().show_system(),
                case_insensitive: self.config.general().case_insensitive(),
                always_show: Arc::clone(self.config.general().always_show()),
                request_id: req_id,
            });
    }

    /// Requests a recursive find operation for the current navigation directory
    pub(crate) fn request_find(&mut self, query: String) {
        self.actions.cancel_find();

        let request_id = self.actions.prepare_new_find_request();
        let cancel_token = Arc::new(AtomicBool::new(false));

        let show_hidden = self.config.general().show_hidden();

        self.actions
            .set_cancel_find_token(Arc::clone(&cancel_token));

        let _ = self.workers.find_tx().try_send(WorkerTask::FindRecursive {
            base_dir: self.nav.current_dir().to_path_buf(),
            query,
            max_results: self.config().general().max_find_results(),
            request_id,
            show_hidden,
            cancel: cancel_token,
        });
    }
}

// AppState tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FileEntry;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::ffi::OsString;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    fn dummy_config() -> Config {
        Config::default()
    }

    #[test]
    fn appstate_new_from_dir_sets_initial_state() -> Result<(), Box<dyn std::error::Error>> {
        let config = dummy_config();
        let temp = tempdir()?;
        let app = AppState::from_dir(&config, temp.path())?;
        assert_eq!(app.nav().current_dir(), temp.path());
        Ok(())
    }

    #[test]
    fn tick_clears_expired_notification_message() -> Result<(), Box<dyn std::error::Error>> {
        let config = dummy_config();
        let temp = tempdir()?;
        let mut app = AppState::from_dir(&config, temp.path())?;
        app.notification_time = Some(Instant::now() - Duration::from_secs(2));
        app.overlays_mut().push(Overlay::Message {
            text: "Timed MSG".to_string(),
        });
        let changed = app.tick();
        assert!(changed);
        assert!(app.notification_time.is_none());
        assert!(
            !app.overlays()
                .iter()
                .any(|o| matches!(o, Overlay::Message { .. }))
        );
        Ok(())
    }

    #[test]
    fn visible_selected_and_has_visible_entries() -> Result<(), Box<dyn std::error::Error>> {
        let config = dummy_config();
        let temp = tempdir()?;
        let app = AppState::from_dir(&config, temp.path())?;
        let app_nav = app.nav();
        if app_nav.entries().is_empty() {
            assert_eq!(app.visible_selected(), None);
            assert!(!app.has_visible_entries());
        } else {
            assert!(app.visible_selected().is_some());
            assert!(app.has_visible_entries());
        }
        Ok(())
    }

    #[test]
    fn handle_keypress_continue_if_no_action() -> Result<(), Box<dyn std::error::Error>> {
        let config = dummy_config();
        let temp = tempdir()?;
        let mut app = AppState::from_dir(&config, temp.path())?;
        let key = KeyEvent::new(KeyCode::Null, KeyModifiers::NONE);
        let result = app.handle_keypress(key);
        assert!(matches!(result, KeypressResult::Continue));
        Ok(())
    }

    #[test]
    fn request_dir_load_sets_is_loading() -> Result<(), Box<dyn std::error::Error>> {
        let config = dummy_config();
        let temp = tempdir()?;
        let mut app = AppState::from_dir(&config, temp.path())?;
        app.is_loading = false;
        app.request_dir_load(None);
        assert!(app.is_loading);
        Ok(())
    }

    #[test]
    fn request_parent_content_handles_root() -> Result<(), Box<dyn std::error::Error>> {
        let config = dummy_config();
        let temp = tempdir()?;
        let mut app = AppState::from_dir(&config, temp.path())?;
        #[cfg(unix)]
        {
            use std::path::PathBuf;
            app.nav.set_path(PathBuf::from("/"));
            app.request_parent_content();
            assert!(app.parent.entries().is_empty());
        }
        #[cfg(windows)]
        {
            use std::path::PathBuf;
            app.nav.set_path(PathBuf::from("C:\\"));
            app.request_parent_content();
            assert!(app.parent.entries().is_empty());
        }
        Ok(())
    }

    #[test]
    fn request_parent_content_uses_cache() -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::default();
        let temp = tempdir()?;
        let subdir = temp.path().join("subdir");
        std::fs::create_dir(&subdir)?;
        let mut app = AppState::from_dir(&config, &subdir)?;

        let parent_path = app
            .nav()
            .current_dir()
            .parent()
            .expect("Should have parent")
            .to_path_buf();

        let prev_request_id = app.parent.request_id();

        let file_entry = FileEntry::new(OsString::from("test_file"), 0, None);
        let dir_entry = FileEntry::new(OsString::from("test_dir"), 1, None);

        app.parent.update_from_entries(
            vec![file_entry, dir_entry],
            "irrelevant",
            prev_request_id,
            &parent_path,
        );

        app.request_parent_content();

        assert_eq!(
            app.parent.request_id(),
            prev_request_id,
            "request_id changed even though parent was cached. Did not use cache!"
        );

        Ok(())
    }
}
