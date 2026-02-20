//! Application module.
//!
//! Defines the main application controller and the logic for mutating app state
//! in response to user input. Submodules handle actions, navigation, key mapping,
//! preview pane and parent pane requests.

pub(crate) mod actions;
pub(crate) mod handlers;
pub(crate) mod keymap;
mod nav;
mod parent;
pub(crate) mod preview;
mod state;
pub(crate) mod tab;

pub(crate) use nav::NavState;
pub(crate) use parent::ParentState;
pub(crate) use preview::{PreviewData, PreviewState};
pub(crate) use state::{AppState, KeypressResult, LayoutMetrics};
pub(crate) use tab::handle_tab_action;

use crate::{app::tab::TabManager, core::worker::Workers};
use std::{collections::HashSet, path::PathBuf};

pub(crate) enum AppContainer<'a> {
    Single(Box<AppState<'a>>),
    Tabs(TabManager<'a>),
}

#[derive(Default)]
pub(crate) struct Clipboard {
    pub(crate) entries: Option<HashSet<PathBuf>>,
    pub(crate) is_cut: bool,
}

pub(crate) struct RunaRoot<'a> {
    pub(crate) container: AppContainer<'a>,
    pub(crate) clipboard: Clipboard,
    pub(crate) workers: Workers,
}

impl RunaRoot<'_> {
    pub(crate) fn update(&mut self) -> bool {
        let mut changed = false;

        while let Ok(response) = self.workers.response_rx().try_recv() {
            changed = true;
            match &mut self.container {
                AppContainer::Single(app) => {
                    app.handle_worker_response(response, &self.workers);
                }
                AppContainer::Tabs(tab_manager) => {
                    let target_app = if let Some(id) = response.tab_id() {
                        tab_manager.tabs.iter_mut().find(|t| t.tab_id == Some(id))
                    } else {
                        Some(&mut tab_manager.tabs[tab_manager.current])
                    };

                    if let Some(app) = target_app {
                        app.handle_worker_response(response, &self.workers);
                    }
                }
            }
        }
        changed
    }
}
