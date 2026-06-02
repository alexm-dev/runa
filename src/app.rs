//! Application module.
//!
//! Defines the main application controller and the logic for mutating app state
//! in response to user input. Submodules handle actions, navigation, key mapping,
//! preview pane and parent pane requests.

pub(crate) mod actions;
pub(crate) mod handlers;
pub(crate) mod keymap;
pub(crate) mod metadata;
pub(crate) mod nav;
mod parent;
pub(crate) mod preview;
mod state;
pub(crate) mod tab;

pub(crate) use nav::NavState;
pub(crate) use parent::ParentState;
pub(crate) use preview::{PreviewData, PreviewState};
pub(crate) use state::{AppState, KeypressResult, LayoutMetrics};
pub(crate) use tab::{handle_sort_action, handle_tab_action};

use crossterm::{
    cursor::Hide,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::config::Config;
use crate::utils::timings::{Throttler, Timings};
use crate::{
    app::tab::TabManager,
    core::workers::{WorkerResponse, Workers},
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// The main container enum to hold either the TabManager or a single boxed AppState to then match
/// either a single state or a tabs which then hold multiple AppStates.
pub(crate) enum AppContainer {
    Single(Box<AppState>),
    Tabs(Box<TabManager>),
}

impl AppContainer {
    pub(crate) fn create_tabs(tabs: Vec<AppState>) -> Self {
        Self::Tabs(Box::new(tab::TabManager::from_vec(tabs)))
    }
}

/// The shared clipboard used by all tabs and all states.
#[derive(Default)]
pub(crate) struct Clipboard {
    pub(crate) entries: Option<HashSet<PathBuf>>,
    pub(crate) is_cut: bool,
}

impl Clipboard {
    pub(crate) fn clear(&mut self) {
        self.entries = None;
        self.is_cut = false;
    }
}

/// The main struct of runa
/// Contains the AppContainer, the shared clipboard and the worker pool
pub(crate) struct RunaRoot {
    pub(crate) container: AppContainer,
    pub(crate) clipboard: Clipboard,
    pub(crate) workers: Workers,
    pub(crate) ui_reload_throttler: Throttler,
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

    pub(crate) fn sync_watch(&mut self) {
        {
            let app = match &self.container {
                AppContainer::Single(app) => app.as_ref(),
                AppContainer::Tabs(tabs) => &tabs.tabs[tabs.current],
            };
            if self.last_watch_dir.as_deref() == Some(app.nav().current_dir()) {
                return;
            }
        }

        let current = match &self.container {
            AppContainer::Single(app) => app.as_ref(),
            AppContainer::Tabs(tabs) => &tabs.tabs[tabs.current],
        }
        .nav()
        .current_dir()
        .to_path_buf();

        let mut dirs = Vec::with_capacity(2);
        dirs.push(current.clone());
        if let Some(parent) = current.parent() {
            dirs.push(parent.to_path_buf());
        }

        self.workers.retarget_watch(dirs);
        self.last_watch_dir = Some(current);
    }

    pub(crate) fn update(&mut self) -> bool {
        let mut changed = false;

        while let Ok(response) = self.workers.response_rx().try_recv() {
            changed = true;
            if matches!(response, WorkerResponse::ConfigChanged) {
                self.reload_config();
                continue;
            }

            match &mut self.container {
                AppContainer::Single(app) => {
                    app.handle_worker_response(response, &self.workers);
                }
                AppContainer::Tabs(tabs) => {
                    let target_app = if let Some(id) = response.tab_id() {
                        tabs.tabs.iter_mut().find(|t| t.tab_id == Some(id))
                    } else {
                        Some(&mut tabs.tabs[tabs.current])
                    };

                    if let Some(app) = target_app {
                        app.handle_worker_response(response, &self.workers);
                    }
                }
            }
        }
        changed
    }

    pub(crate) fn reload_ui(&mut self, writer: &mut impl std::io::Write) -> std::io::Result<bool> {
        if !self.ui_reload_throttler.can_trigger(Timings::UI_RELOAD_MS) {
            return Ok(false);
        }

        self.ui_reload_throttler.touch();
        execute!(writer, LeaveAlternateScreen, EnterAlternateScreen, Hide,)?;
        match &mut self.container {
            AppContainer::Single(app) => {
                app.push_overlay_message("UI reloaded".into(), Duration::from_secs(2));
            }
            AppContainer::Tabs(tabs) => {
                for tab in &mut tabs.tabs {
                    tab.push_overlay_message("UI reloaded".into(), Duration::from_secs(2));
                }
            }
        }
        Ok(true)
    }

    pub(crate) fn reload_config(&mut self) {
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
                            Duration::from_secs(2),
                        );
                    }
                    AppContainer::Tabs(tabs) => {
                        for tab in &mut tabs.tabs {
                            tab.apply_new_config(Arc::clone(&new_config));
                        }
                        tabs.sync_tab_line();
                        tabs.tabs[tabs.current].push_overlay_message(
                            "Configuration reloaded!".into(),
                            Duration::from_secs(2),
                        );
                    }
                }
            }
            Err(e) => {
                self.config_reload_throttler.touch();
                let target_app = match &mut self.container {
                    AppContainer::Single(app) => app,
                    AppContainer::Tabs(tabs) => &mut tabs.tabs[tabs.current],
                };
                target_app.push_overlay_message(e, Duration::from_secs(5));
            }
        }
    }
}
