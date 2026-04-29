//! Application State and main controller module for runa.
//!
//! This module defines the overall [AppState] struct, which holds all major application
//! information and passes it to relevant UI/Terminal functions

use std::ffi::OsString;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossterm::event::KeyEvent;
use ratatui::text::Span;

use crate::app::{
    Clipboard, NavState, ParentState, PreviewState,
    actions::{ActionContext, ActionMode, InputMode},
    keymap::{Action, Keymap, TabAction},
    metadata::MetadataState,
};
use crate::config::Config;
use crate::config::display::PreviewMethod;
use crate::core::{
    cache::DirListOptions,
    metadata::{FileMetadataCache, MetadataNeeds},
    sort::SortConfig,
    workers::{WorkerResponse, WorkerTask, Workers},
};

use crate::ui::overlays::{OverlayKind, OverlayStack};
use crate::utils::timings::{Throttler, Timings};

/// Enumeration for each individual keypress result processed.
///
/// Is used to process action logic correctly.
pub(crate) enum KeypressResult {
    Continue,
    Consumed,
    Quit,
    OpenedEditor,
    Recovered,
    Tab(TabAction),
    Sort(SortConfig),
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

    pub(super) metadata: MetadataState,

    pub(super) is_loading: bool,

    pub(super) notification_time: Option<Instant>,
    pub(super) worker_time: Option<Instant>,

    pub(super) nav_time: Throttler,
    pub(super) preview_request_time: Throttler,

    pub(super) overlays: OverlayStack,

    pub(super) tab_id: Option<usize>,
    pub(super) tab_line: Arc<Vec<Span<'a>>>,
}

impl<'a> AppState<'a> {
    pub(crate) fn new(config: &'a Config) -> std::io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        Self::from_dir(config, &current_dir)
    }

    pub(crate) fn new_current_dir(&self) -> std::io::Result<Self> {
        let mut app = Self::from_dir(self.config, self.nav.current_dir())?;
        app.nav.set_sort_config(self.nav.sort_config());
        Ok(app)
    }

    pub(crate) fn from_dir(config: &'a Config, initial_path: &Path) -> std::io::Result<Self> {
        let current_dir = if initial_path.exists() && initial_path.is_dir() {
            initial_path.to_path_buf()
        } else {
            std::env::current_dir()?
        };

        let app = Self {
            config,
            keymap: Keymap::from_config(config),
            metrics: LayoutMetrics::default(),
            nav: NavState::new(current_dir),
            actions: ActionContext::default(),
            preview: PreviewState::default(),
            parent: ParentState::default(),
            metadata: MetadataState::new(),
            is_loading: false,
            notification_time: None,
            nav_time: Throttler::new(),
            preview_request_time: Throttler::new(),
            worker_time: None,
            overlays: OverlayStack::new(),
            tab_line: Arc::new(Vec::new()),
            tab_id: None,
        };

        Ok(app)
    }

    /// Initializes the AppState by requesting the initial directory load and parent content.
    pub(crate) fn initialize(&mut self, workers: &Workers, focus: Option<OsString>) {
        self.request_dir_load(workers, focus);
        self.request_parent_content(workers);
    }

    crate::getters! {
        config: &Config,
        nav: &NavState,
        actions: &ActionContext,
        preview: &PreviewState,
        parent: &ParentState,
        is_loading: bool,
        worker_time: &Option<Instant>,
        overlays: &OverlayStack,
    }

    #[inline]
    pub(crate) fn overlays_mut(&mut self) -> &mut OverlayStack {
        &mut self.overlays
    }

    #[inline]
    pub(crate) fn tab_line(&self) -> &Arc<Vec<Span<'_>>> {
        &self.tab_line
    }

    #[inline]
    fn tab_id(&self) -> Option<usize> {
        self.tab_id
    }

    #[inline]
    pub(crate) fn selected_metadata(&self) -> Option<&FileMetadataCache> {
        self.metadata.selected()
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
    pub(crate) fn update_layout_metrics(&mut self, workers: &Workers, metrics: LayoutMetrics) {
        let old_width = self.metrics.preview_width;
        let old_height = self.metrics.preview_height;

        self.metrics = metrics;

        if old_width != self.metrics.preview_width || old_height != self.metrics.preview_height {
            if self.preview.data().is_empty() {
                self.request_preview(workers);
            } else {
                self.preview.mark_pending();
            }
        }
    }

    pub(crate) fn update_file_info_cache(&mut self, workers: &Workers) {
        let status_info = self.config.display().info().status_bar();
        let info_overlay = self.overlays().is_open(OverlayKind::ShowInfo);

        if !status_info && !info_overlay {
            return;
        }
        if !self.metadata.can_request(Timings::FILE_INFO_DEBOUNCE_MS) {
            return;
        }

        let Some(entry) = self.nav.selected_entry() else {
            self.metadata.clear();
            return;
        };

        if let Some(selected_cache) = self.metadata.selected_arc()
            && entry.name() == selected_cache.name()
        {
            return;
        }

        let path = self.nav.current_dir().join(entry.name());
        if self.metadata.is_pending_path(&path) {
            return;
        }

        let req_id = self.metadata.prepare_new_request();
        let date_format = self.config.display().info().date_format();

        let needs = self.metadata_needs();

        if workers
            .metadata_tx()
            .try_send(WorkerTask::GetFileMetadata {
                path: path.clone(),
                date_format: date_format.to_string(),
                request_id: req_id,
                needs,
            })
            .is_ok()
        {
            self.metadata.touch();
            self.metadata.set_pending(req_id, path);
        }
    }

    pub(crate) fn apply_sort_config(&mut self, config: SortConfig) {
        self.nav.set_sort_config(config);
    }

    /// The heart of the app: updates state and handles worker messages
    ///
    /// Is used by the main event loop to update the application state.
    /// Returns a bool to determine if the AppState needs reloading
    /// and sets it to true if a WorkerResponse was made or if a preview should be triggered.
    pub(crate) fn tick(&mut self, workers: &Workers) -> bool {
        let mut changed = false;

        if let Some(expiry) = self.notification_time
            && Instant::now() >= expiry
        {
            self.notification_time = None;

            self.overlays_mut().remove_kind(OverlayKind::Message);

            changed = true;
        }

        if workers.active().load(Ordering::Relaxed) > 0 {
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
            self.request_preview(workers);
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
                self.actions.find_mut().clear_results();
            } else {
                self.request_find(workers, query);
            }
            changed = true;
        }

        changed
    }

    pub(crate) fn handle_worker_response(&mut self, response: WorkerResponse, workers: &Workers) {
        match response {
            WorkerResponse::DirectoryLoaded {
                path,
                entries,
                focus,
                sort_column,
                request_id,
                tab_id: _tab_id,
            } => {
                // only update nav if BOTH the ID and path match.
                if request_id == self.nav.request_id() && path == self.nav.current_dir() {
                    self.nav
                        .update_from_worker(path, entries, sort_column, focus);
                    self.is_loading = false;

                    self.request_parent_content(workers);
                    self.request_preview_force(workers);
                    self.update_file_info_cache(workers);
                    self.refresh_show_info_if_open();
                    return;
                }
                // PREVIEW CHECK: Must match the current preview request
                if request_id == self.preview.request_id()
                    && let Some(entry) = self.nav.selected_entry()
                    && path.parent() == Some(self.nav.current_dir())
                    && path.file_name() == Some(entry.name())
                {
                    self.preview
                        .update_from_entries(entries.clone(), sort_column, request_id);

                    let pos = self
                        .nav
                        .get_position()
                        .get(&path)
                        .and_then(|saved| entries.iter().position(|e| e.name() == saved))
                        .unwrap_or(0);

                    self.preview.set_selected_idx(pos);
                    return;
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
                        let sort_config = self.nav.sort_config();
                        self.parent.update_from_entries(
                            entries,
                            &current_name,
                            request_id,
                            &path,
                            sort_config,
                            sort_column,
                        );
                    }
                }
            }

            WorkerResponse::PreviewLoaded {
                lines,
                request_id,
                is_eof,
                tab_id: _tab_id,
            } => {
                if request_id == self.preview.request_id() {
                    self.preview.update_content(
                        lines,
                        self.metrics.preview_height,
                        is_eof,
                        request_id,
                    );
                }
            }

            WorkerResponse::OperationComplete { need_reload, focus } => {
                if need_reload {
                    workers.cache().invalidate_path(self.nav.current_dir());
                    self.request_dir_load(workers, focus);
                    self.request_parent_content(workers);
                }
            }

            WorkerResponse::FileMetadataLoaded {
                metadata,
                path,
                request_id,
            } => {
                if self.metadata.matches_pending(request_id, &path) {
                    if path.parent() == Some(self.nav.current_dir())
                        && let Some(sel) = self.nav.selected_entry()
                        && path.file_name() == Some(sel.name())
                    {
                        self.metadata.set_selected(Some(metadata));
                        self.refresh_show_info_if_open();
                    }
                    self.metadata.clear_pending();
                }
            }

            WorkerResponse::FindResults {
                base_dir,
                results,
                request_id,
                tab_id: _tab_id,
            } => {
                if base_dir == self.nav.current_dir()
                    && request_id == self.actions.find().request_id()
                {
                    self.actions.find_mut().set_results(results);
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

    /// Central key handlers
    ///
    /// Coordinates the action and handler module functions.
    pub(crate) fn handle_keypress(
        &mut self,
        key: KeyEvent,
        workers: &Workers,
        clipboard: &mut Clipboard,
    ) -> KeypressResult {
        if self.actions.is_input_mode() {
            return self.handle_input_mode(workers, key);
        }

        if let Some(res) = self.handle_esc_close_overlays(&key) {
            return res;
        }

        if let Some(res) = self.handle_prefix_dispatch(workers, &key) {
            return res;
        }

        if let Some(action) = self.keymap.lookup(key) {
            match action {
                Action::System(sys_act) => return self.handle_sys_action(sys_act),
                Action::Nav(nav_act) => return self.handle_nav_action(workers, nav_act, clipboard),
                Action::File(file_act) => {
                    return self.handle_file_action(workers, file_act, clipboard);
                }
                Action::Tab(tab_act) => return KeypressResult::Tab(tab_act),
            }
        }

        KeypressResult::Continue
    }

    // Worker requests functions for directory loading, preview and parent pane content

    /// Requests a directory load for the current navigation directory
    pub(crate) fn request_dir_load(&mut self, workers: &Workers, focus: Option<OsString>) {
        self.is_loading = true;
        let request_id = self.nav.prepare_new_request();
        let sort_config = self.nav.sort_config();
        let sort_date_format: Arc<str> = Arc::from(self.config.display().sort_date_format());
        let _ = workers.nav_io_tx().try_send(WorkerTask::LoadDirectory {
            path: self.nav.current_dir().to_path_buf(),
            focus,
            list: self.dir_list_options(),
            sort_config,
            sort_date_format,
            always_show: Arc::clone(self.config.general().always_show()),
            request_id,
            tab_id: self.tab_id(),
        });
    }

    #[inline]
    pub(crate) fn request_preview(&mut self, workers: &Workers) {
        self.do_request_preview(workers, false);
    }

    #[inline]
    pub(crate) fn request_preview_force(&mut self, workers: &Workers) {
        self.do_request_preview(workers, true);
    }

    /// Requests loading of the parent directory content for the parent pane
    pub(crate) fn request_parent_content(&mut self, workers: &Workers) {
        let Some(parent_path) = self.nav.current_dir().parent() else {
            self.parent.clear();
            return;
        };

        let sort_config = self.nav.sort_config();
        let list_opts = self.dir_list_options();

        if let Some(val) = workers.cache().get(parent_path, sort_config, &list_opts) {
            let (entries, sort_col, _rid, _ts) = &*val;
            let entries_vec = entries.clone();
            let sort_vec = sort_col.clone();
            let current_name = self
                .nav
                .current_dir()
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let req_id = self.parent.prepare_new_request(parent_path, sort_config);
            self.parent.update_from_entries(
                entries_vec,
                &current_name,
                req_id,
                parent_path,
                sort_config,
                sort_vec,
            );
            return;
        }

        if self.parent.is_cached(parent_path, sort_config) {
            return;
        }

        let parent_path_buf = parent_path.to_path_buf();
        let sort_date_format: Arc<str> = Arc::from(self.config.display().sort_date_format());
        let req_id = self
            .parent
            .prepare_new_request(&parent_path_buf, sort_config);
        let _ = workers.parent_io_tx().try_send(WorkerTask::LoadDirectory {
            path: parent_path_buf,
            focus: None,
            list: self.dir_list_options(),
            sort_config,
            sort_date_format,
            always_show: Arc::clone(self.config.general().always_show()),
            request_id: req_id,
            tab_id: self.tab_id(),
        });
    }

    /// Requests a recursive find operation for the current navigation directory
    pub(crate) fn request_find(&mut self, workers: &Workers, query: String) {
        self.actions.find_mut().cancel_current();

        let request_id = self.actions.find_mut().prepare_new_request();
        let cancel_token = Arc::new(AtomicBool::new(false));

        let show_hidden = self.config.general().show_hidden();

        self.actions
            .set_cancel_find_token(Arc::clone(&cancel_token));

        let _ = workers.find_tx().try_send(WorkerTask::FindRecursive {
            base_dir: self.nav.current_dir().to_path_buf(),
            query,
            max_results: self.config().general().max_find_results(),
            request_id,
            show_hidden,
            cancel: cancel_token,
            tab_id: self.tab_id(),
        });
    }

    pub(crate) fn request_dir_sort(&mut self, workers: &Workers, focus: Option<OsString>) {
        self.is_loading = true;
        let request_id = self.nav.request_id();
        let sort_config = self.nav.sort_config();
        let sort_date_format: Arc<str> = Arc::from(self.config.display().sort_date_format());
        let entries = self.nav.entries_arc();

        let _ = workers.sort_io_tx().try_send(WorkerTask::SortDirectory {
            path: self.nav.current_dir().to_path_buf(),
            entries,
            focus,
            list: self.dir_list_options(),
            sort_config,
            sort_date_format,
            always_show: Arc::clone(self.config.general().always_show()),
            request_id,
            tab_id: self.tab_id(),
        });
    }

    pub(crate) fn dir_list_options(&self) -> DirListOptions {
        DirListOptions {
            dirs_first: self.config.general().dirs_first(),
            show_hidden: self.config.general().show_hidden(),
            show_symlink: self.config.general().show_symlink(),
            show_system: self.config.general().show_system(),
            case_insensitive: self.config.general().case_insensitive(),
        }
    }

    /// Requests a preview load for the currently selected entry in the navigation pane
    fn do_request_preview(&mut self, workers: &Workers, force: bool) {
        if let Some(entry) = self.nav.selected_entry() {
            let path = self.nav.current_dir().join(entry.name());
            if !force
                && let Some(current) = self.preview.current_path()
                && current == path
                && !self.preview.data().is_empty()
                && self.preview.scroll().offset() == self.preview.loaded_scroll()
            {
                return;
            }

            if !force
                && !self
                    .preview_request_time
                    .can_trigger(Timings::PREVIEW_REQUEST_MS)
            {
                return;
            }

            let req_id = self.preview.prepare_new_request(path.clone());
            let sort_config = self.nav.sort_config();
            let list_opts = self.dir_list_options();
            let sort_date_format: Arc<str> = Arc::from(self.config.display().sort_date_format());

            if entry.is_dir() || entry.is_symlink() {
                if let Some(val) = workers.cache().get(&path, sort_config, &list_opts) {
                    let (entries, sort_col, _rid, _ts) = &*val;
                    self.preview
                        .update_from_entries(entries.clone(), sort_col.clone(), req_id);
                    let pos = self
                        .nav
                        .get_position()
                        .get(&path)
                        .and_then(|saved| entries.iter().position(|e| e.name() == saved))
                        .unwrap_or(0);
                    self.preview.set_selected_idx(pos);
                    self.preview_request_time.touch();
                    return;
                }

                if workers
                    .preview_io_tx()
                    .try_send(WorkerTask::LoadDirectory {
                        path,
                        focus: None,
                        list: list_opts.clone(),
                        sort_config,
                        sort_date_format,
                        always_show: Arc::clone(self.config.general().always_show()),
                        request_id: req_id,
                        tab_id: self.tab_id(),
                    })
                    .is_ok()
                {
                    self.preview_request_time.touch();
                } else {
                    self.preview.mark_pending();
                }
            } else {
                let preview_options = self.config.display().preview_options();
                let preview_method = preview_options.method();
                let scroll = self.preview.scroll().offset() as usize;

                let task = match preview_method {
                    PreviewMethod::Bat => {
                        let args = self
                            .config
                            .bat_args_for_preview(self.metrics.preview_width)
                            .into_iter()
                            .map(OsString::from)
                            .collect();

                        WorkerTask::LoadBatPreview {
                            path,
                            max_lines: self.metrics.preview_height,
                            pane_width: self.metrics.preview_width,
                            scroll,
                            args,
                            request_id: req_id,
                            tab_id: self.tab_id(),
                        }
                    }

                    PreviewMethod::Internal => WorkerTask::LoadInternalPreview {
                        path,
                        max_lines: self.metrics.preview_height,
                        pane_width: self.metrics.preview_width,
                        scroll,
                        request_id: req_id,
                        tab_id: self.tab_id(),
                    },
                };

                if workers.preview_file_tx().try_send(task).is_ok() {
                    self.preview_request_time.touch();
                } else {
                    self.preview.mark_pending();
                }
            }
        } else {
            self.preview.clear();
        }
    }

    fn metadata_needs(&self) -> MetadataNeeds {
        MetadataNeeds {
            name: self.config.display().info().name(),
            file_type: self.config.display().info().file_type(),
            size: self.config.display().info().size(),
            accessed: self.config.display().info().accessed(),
            modified: self.config.display().info().modified(),
            created: self.config.display().info().created(),
            perms: self.config.display().info().perms(),
            #[cfg(unix)]
            owner: self.config.display().info().owner(),
            #[cfg(unix)]
            group: self.config.display().info().group(),
        }
    }
}

// AppState tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FileEntry;
    use crate::ui::overlays::Overlay;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::ffi::OsString;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    fn dummy_config() -> Config {
        Config::default()
    }

    fn dummy_workers() -> Workers {
        Workers::spawn()
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
        let workers = dummy_workers();
        let temp = tempdir()?;
        let mut app = AppState::from_dir(&config, temp.path())?;
        app.notification_time = Some(Instant::now() - Duration::from_secs(2));
        app.overlays_mut().push(Overlay::Message {
            text: "Timed MSG".to_string(),
        });
        let changed = app.tick(&workers);
        assert!(changed);
        assert!(app.notification_time.is_none());
        assert!(!app.overlays().is_open(OverlayKind::Message));
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
        let workers = dummy_workers();
        let temp = tempdir()?;

        let mut clipboard = Clipboard::default();

        let mut app = AppState::from_dir(&config, temp.path())?;
        let key = KeyEvent::new(KeyCode::Null, KeyModifiers::NONE);
        let result = app.handle_keypress(key, &workers, &mut clipboard);
        assert!(matches!(result, KeypressResult::Continue));
        Ok(())
    }

    #[test]
    fn request_dir_load_sets_is_loading() -> Result<(), Box<dyn std::error::Error>> {
        let config = dummy_config();
        let workers = dummy_workers();
        let temp = tempdir()?;
        let mut app = AppState::from_dir(&config, temp.path())?;
        app.is_loading = false;
        app.request_dir_load(&workers, None);
        assert!(app.is_loading);
        Ok(())
    }

    #[test]
    fn request_parent_content_handles_root() -> Result<(), Box<dyn std::error::Error>> {
        let config = dummy_config();
        let workers = dummy_workers();
        let temp = tempdir()?;
        let mut app = AppState::from_dir(&config, temp.path())?;
        #[cfg(unix)]
        {
            use std::path::PathBuf;
            app.nav.set_path(PathBuf::from("/"));
            app.request_parent_content(&workers);
            assert!(app.parent.entries().is_empty());
        }
        #[cfg(windows)]
        {
            use std::path::PathBuf;
            app.nav.set_path(PathBuf::from("C:\\"));
            app.request_parent_content(&workers);
            assert!(app.parent.entries().is_empty());
        }
        Ok(())
    }

    #[test]
    fn request_parent_content_uses_cache() -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::default();
        let workers = dummy_workers();
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
        let sort_config = app.nav.sort_config();

        let file_entry = FileEntry::new(OsString::from("test_file"), 0, None);
        let dir_entry = FileEntry::new(OsString::from("test_dir"), 1, None);

        app.parent.update_from_entries(
            Arc::from(vec![file_entry, dir_entry]),
            "irrelevant",
            prev_request_id,
            &parent_path,
            sort_config,
            None,
        );

        app.request_parent_content(&workers);

        assert_eq!(
            app.parent.request_id(),
            prev_request_id,
            "request_id changed even though parent was cached. Did not use cache!"
        );

        Ok(())
    }
}
