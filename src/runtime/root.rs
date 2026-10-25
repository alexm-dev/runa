//! Runtime root for runa.
//!
//! Holds the app states, the shared clipboard and the workers and drives them for the event
//! loop.
//! This Module is the main orchestration of runa, that ties everything together.
//!
//! [RunaRoot] is the main struct, which holds the [AppContainer], the [Clipboard] and the [Workers].
//! [AppContainer] is the main container enum, that holds either a single [AppState] or a [TabManager]
//! which holds tabs of [AppState]s.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use crossterm::{
    cursor::Hide,
    event::KeyEvent,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Frame;

use crate::app::tab;
use crate::app::{AppContainer, Clipboard, KeypressResult};
use crate::config::Config;
use crate::core::workers::{WorkerResponse, Workers};
use crate::ui;
use crate::utils::timings::{Throttler, Timings};

/// The main struct of runa
/// Contains the AppContainer, the shared clipboard and the worker pool
pub(crate) struct RunaRoot {
    container: AppContainer,
    clipboard: Clipboard,
    workers: Workers,
    ui_reload_throttler: Throttler,
    config_reload_throttler: Throttler,
    last_watch_dir: Option<PathBuf>,
}

impl RunaRoot {
    #[inline]
    pub(crate) fn new(container: AppContainer, workers: Workers) -> Self {
        Self {
            container,
            clipboard: Clipboard::default(),
            workers,
            config_reload_throttler: Throttler::default(),
            ui_reload_throttler: Throttler::default(),
            last_watch_dir: None,
        }
    }

    pub(crate) fn tick(&mut self) -> bool {
        let mut changed = self.update();
        changed |= self.container.current_mut().process_timers(&self.workers);

        if changed {
            self.sync_watch();

            if let AppContainer::Tabs(tabs) = &mut self.container {
                tabs.sync_tab_line();
            }
        }

        changed
    }

    pub(crate) fn handle_keypress(&mut self, key: KeyEvent) -> KeypressResult {
        let result =
            self.container
                .current_mut()
                .handle_keypress(key, &self.workers, &mut self.clipboard);

        match result {
            KeypressResult::Tab(tab_act) => {
                tab::handle_tab_action(&self.workers, &mut self.container, tab_act)
            }
            KeypressResult::Sort(config) => {
                tab::handle_sort_action(&mut self.container, config);
                KeypressResult::Consumed
            }
            other => other,
        }
    }

    pub(crate) fn render(&mut self, frame: &mut Frame) {
        let app = self.container.current_mut();
        let layout = ui::frame_layout(frame.area(), app);
        app.update_layout_metrics(&self.workers, layout.metrics());
        ui::render(
            frame,
            app,
            &layout,
            &self.clipboard,
            self.workers.fileop_counts(),
        );
    }

    pub(crate) fn reload_ui(&mut self, writer: &mut impl io::Write) -> io::Result<bool> {
        if !self.ui_reload_throttler.can_trigger(Timings::UI_RELOAD_MS) {
            return Ok(false);
        }

        self.ui_reload_throttler.touch();
        execute!(writer, LeaveAlternateScreen, EnterAlternateScreen, Hide,)?;
        match &mut self.container {
            AppContainer::Single(app) => {
                app.push_overlay_message("UI reloaded!".into(), Timings::MESSAGE_SHORT, None);
            }
            AppContainer::Tabs(tabs) => {
                for tab in tabs.iter_mut() {
                    tab.push_overlay_message("UI reloaded!".into(), Timings::MESSAGE_SHORT, None);
                }
            }
        }
        Ok(true)
    }

    fn update(&mut self) -> bool {
        let mut changed = false;

        while let Ok(response) = self.workers.response_rx().try_recv() {
            changed = true;
            if matches!(response, WorkerResponse::ConfigChanged) {
                self.reload_config();
                continue;
            }

            self.container
                .route_worker_response(response, &self.workers);
        }
        changed
    }

    fn sync_watch(&mut self) {
        let current = self.container.current().nav().current_dir();
        if self.last_watch_dir.as_deref() == Some(current) {
            return;
        }

        let current = current.to_path_buf();

        let mut dirs = Vec::with_capacity(2);
        dirs.push(current.clone());
        if let Some(parent) = current.parent() {
            dirs.push(parent.to_path_buf());
        }

        self.workers.retarget_watch(dirs);
        self.last_watch_dir = Some(current);
    }

    fn reload_config(&mut self) {
        if !self
            .config_reload_throttler
            .can_trigger(Timings::CONFIG_RELOAD_MS)
        {
            return;
        }

        match Config::load() {
            Ok(config) => {
                self.config_reload_throttler.touch();
                let new_config = Arc::new(config);

                match &mut self.container {
                    AppContainer::Single(app) => {
                        app.apply_new_config(Arc::clone(&new_config));
                        app.push_overlay_message(
                            "Configuration reloaded!".into(),
                            Timings::MESSAGE_SHORT,
                            None,
                        );
                    }
                    AppContainer::Tabs(tabs) => {
                        for tab in tabs.iter_mut() {
                            tab.apply_new_config(Arc::clone(&new_config));
                        }
                        tabs.sync_tab_line();
                        tabs.current_tab_mut().push_overlay_message(
                            "Configuration reloaded!".into(),
                            Timings::MESSAGE_SHORT,
                            None,
                        );
                    }
                }
            }
            Err(e) => {
                self.config_reload_throttler.touch();
                self.container
                    .current_mut()
                    .push_overlay_message(e, Timings::MESSAGE_LONG, None);
            }
        }
    }
}
