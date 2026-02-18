use crate::app::KeypressResult;
use crate::app::keymap::TabAction;
use crate::app::{AppContainer, AppState};

use std::fmt::Write;
use std::sync::Arc;

pub(crate) struct TabManager<'a> {
    pub(crate) tabs: Vec<AppState<'a>>,
    pub(crate) current: usize,
}

impl<'a> TabManager<'a> {
    const MAX_TABS: usize = 9;

    pub(crate) fn new(existing: AppState<'a>, new_tab: AppState<'a>) -> Self {
        let mut manager = Self {
            tabs: vec![existing, new_tab],
            current: 0,
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

    pub(crate) fn add_tab(&mut self, tab: AppState<'a>) -> usize {
        if self.tabs.len() >= Self::MAX_TABS {
            return self.current;
        }

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
        let line = if self.tabs.len() <= 1 {
            String::new()
        } else {
            let mut line = String::with_capacity(64);
            for (i, _) in self.tabs.iter().enumerate() {
                let marker = if i == self.current { "*" } else { "" };
                let _ = write!(line, "{}:{} ", i + 1, marker);
            }
            line
        };

        if let Some(active_tab) = self.tabs.get_mut(self.current) {
            active_tab.tab_line = Arc::new(line);
        }
    }
}

pub(crate) fn handle_tab_action<'a>(
    root: &mut AppContainer<'a>,
    action: TabAction,
) -> KeypressResult {
    match action {
        TabAction::New => {
            let config = match root {
                AppContainer::Single(app_state) => app_state.config,
                AppContainer::Tabs(tab) => tab.current_tab().config,
            };

            match root {
                AppContainer::Single(app_state) => {
                    let current = std::mem::replace(
                        app_state,
                        Box::new(AppState::new(config).expect("Failed to create new blank tab")),
                    );
                    let new_tab = AppState::new(config).expect("Failed to create new blank tab");
                    *root = AppContainer::Tabs(TabManager::new(*current, new_tab));
                }
                AppContainer::Tabs(tab) => {
                    let new_tab = AppState::new(config).expect("Failed to create new blank tab");
                    tab.add_tab(new_tab);
                }
            }
            KeypressResult::Consumed
        }
        TabAction::Close => {
            match root {
                AppContainer::Single(_) => return KeypressResult::Quit,
                AppContainer::Tabs(tab) => {
                    tab.close_tab(tab.current);
                    if tab.len() == 1 {
                        let last = tab.tabs.remove(0);
                        *root = AppContainer::Single(Box::new(last));
                    } else if tab.is_empty() {
                        return KeypressResult::Quit;
                    }
                }
            }
            KeypressResult::Consumed
        }
        TabAction::Next => {
            if let AppContainer::Tabs(tab) = root {
                tab.switch(1);
            }
            KeypressResult::Consumed
        }
        TabAction::Prev => {
            if let AppContainer::Tabs(tab) = root {
                tab.switch(-1);
            }
            KeypressResult::Consumed
        }
        TabAction::Cycle => {
            if let AppContainer::Tabs(tab) = root {
                tab.switch(1);
            }
            KeypressResult::Consumed
        }
        TabAction::Switch(n) => {
            if let AppContainer::Tabs(tab) = root {
                let idx = n as usize - 1;
                if idx < tab.len() {
                    tab.set_active(idx);
                }
            }
            KeypressResult::Consumed
        }
    }
}
