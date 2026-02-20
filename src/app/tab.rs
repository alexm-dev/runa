//! Tab management for runa.
//!
//! This module deines the `TabManager` struct, which manages multiple tabs in the application,
//! allowing to switch between them, add new tabs, and close existing ones.
//!
//! It also includes the [handle_tab_action] function, which processes tab actions related to tab management.

use crate::app::KeypressResult;
use crate::app::keymap::TabAction;
use crate::app::{AppContainer, AppState};
use crate::core::worker::Workers;

use ratatui::text::Span;
use std::ffi::OsString;
use std::sync::Arc;

pub(crate) struct TabManager<'a> {
    pub(crate) tabs: Vec<AppState<'a>>,
    pub(crate) current: usize,
    next_tab_id: usize,
}

impl<'a> TabManager<'a> {
    const MAX_TABS: usize = 9;

    pub(crate) fn new(
        mut existing: AppState<'a>,
        mut new_tab: AppState<'a>,
        workers: &Workers,
        focus: Option<OsString>,
    ) -> Self {
        existing.tab_id = Some(0);
        new_tab.tab_id = Some(1);

        new_tab.initialize(workers, focus);

        let mut manager = Self {
            tabs: vec![existing, new_tab],
            current: 1,
            next_tab_id: 2,
        };
        manager.sync_tab_line();
        manager
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.tabs.len()
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub(crate) fn current_tab(&self) -> &AppState<'a> {
        &self.tabs[self.current]
    }

    pub(crate) fn current_tab_mut(&mut self) -> &mut AppState<'a> {
        &mut self.tabs[self.current]
    }

    pub(crate) fn add_tab(
        &mut self,
        mut tab: AppState<'a>,
        workers: &Workers,
        focus: Option<OsString>,
    ) -> usize {
        if self.tabs.len() >= Self::MAX_TABS {
            return self.current;
        }

        tab.tab_id = Some(self.next_tab_id);
        self.next_tab_id = self.next_tab_id.saturating_add(1);
        tab.initialize(workers, focus);
        self.tabs.push(tab);
        self.current = self.tabs.len() - 1;
        self.sync_tab_line();
        self.current
    }

    pub(crate) fn switch(&mut self, dir: isize) {
        let n = self.tabs.len() as isize;
        if n == 0 {
            return;
        }
        self.current = ((self.current as isize + dir + n) % n) as usize;
        self.sync_tab_line();
    }

    pub(crate) fn set_active(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.current = idx;
            self.sync_tab_line();
        }
    }

    pub(crate) fn close_tab(&mut self, idx: usize) -> bool {
        if idx >= self.tabs.len() {
            return !self.tabs.is_empty();
        }

        self.tabs.remove(idx);
        if self.tabs.is_empty() {
            self.current = 0;
            false
        } else {
            if self.current >= self.tabs.len() {
                self.current = self.tabs.len() - 1;
            }
            self.sync_tab_line();
            true
        }
    }

    pub(crate) fn sync_tab_line(&mut self) {
        let tab_spans = if self.tabs.len() <= 1 {
            Vec::new()
        } else {
            let theme = self.tabs[self.current].config.theme();
            let tab_theme = &theme.tab();

            let mut spans = Vec::with_capacity(self.tabs.len() * 2 - 1);
            for (i, tab) in self.tabs.iter().enumerate() {
                let is_current = i == self.current;
                let style = if is_current {
                    tab_theme.active_style_or_theme()
                } else {
                    tab_theme.inactive_style_or_theme()
                };

                let name = if tab_theme.uses_name() {
                    let cwd = tab.nav.current_dir();
                    cwd.file_name()
                        .map(|os_str| os_str.to_string_lossy().into_owned())
                        .unwrap_or_else(|| cwd.to_string_lossy().into_owned())
                } else {
                    String::new()
                };

                let formatted = tab_theme.format_tab(i, is_current, Some(&name));

                spans.push(Span::styled(formatted, style));
                if i != self.tabs.len() - 1 {
                    spans.push(Span::raw(tab_theme.separator()));
                }
            }
            spans
        };

        if let Some(active_tab) = self.tabs.get_mut(self.current) {
            active_tab.tab_line = Arc::new(tab_spans);
        }
    }
}

pub(crate) fn handle_tab_action<'a>(
    workers: &Workers,
    container: &mut AppContainer<'a>,
    action: TabAction,
) -> KeypressResult {
    match action {
        TabAction::New => {
            match container {
                AppContainer::Single(app_state) => {
                    let focus = app_state
                        .nav()
                        .selected_entry()
                        .map(|entry| entry.name().to_os_string());
                    let original = std::mem::replace(
                        app_state,
                        Box::new(
                            app_state
                                .new_current_dir()
                                .expect("Failed to create temp tab"),
                        ),
                    );
                    let new_tab = original
                        .new_current_dir()
                        .expect("Failed to create new blank tab");
                    *container =
                        AppContainer::Tabs(TabManager::new(*original, new_tab, workers, focus));
                    if let AppContainer::Tabs(tab_manager) = container {
                        tab_manager.current_tab_mut().tick(workers);
                    }
                }
                AppContainer::Tabs(tab) => {
                    let focus = tab
                        .current_tab()
                        .nav()
                        .selected_entry()
                        .map(|entry| entry.name().to_os_string());
                    let new_tab = tab
                        .current_tab()
                        .new_current_dir()
                        .expect("Failed to create new blank tab");
                    tab.add_tab(new_tab, workers, focus);
                }
            }
            KeypressResult::Consumed
        }
        TabAction::Close => {
            match container {
                AppContainer::Single(_) => return KeypressResult::Quit,
                AppContainer::Tabs(tab) => {
                    tab.close_tab(tab.current);
                    if tab.len() == 1 {
                        let last = tab.tabs.remove(0);
                        *container = AppContainer::Single(Box::new(last));
                    } else if tab.is_empty() {
                        return KeypressResult::Quit;
                    }
                }
            }
            KeypressResult::Consumed
        }
        TabAction::Next => {
            if let AppContainer::Tabs(tab) = container {
                tab.switch(1);
            }
            KeypressResult::Consumed
        }
        TabAction::Prev => {
            if let AppContainer::Tabs(tab) = container {
                tab.switch(-1);
            }
            KeypressResult::Consumed
        }
        TabAction::Cycle => {
            if let AppContainer::Tabs(tab) = container {
                tab.switch(1);
            }
            KeypressResult::Consumed
        }
        TabAction::Switch(n) => {
            if let AppContainer::Tabs(tab) = container {
                let idx = n as usize - 1;
                if idx < tab.len() {
                    tab.set_active(idx);
                }
            }
            KeypressResult::Consumed
        }
    }
}
