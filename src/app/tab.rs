use crate::app::AppState;

pub(crate) struct TabManager<'a> {
    pub(crate) tabs: Vec<AppState<'a>>,
    pub(crate) current: usize,
}

impl<'a> TabManager<'a> {
    pub(crate) fn new(existing: AppState<'a>, new_tab: AppState<'a>) -> Self {
        Self {
            tabs: vec![existing, new_tab],
            current: 0,
        }
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
        self.sync_tab_line();
        self.current = ((self.current as isize + dir + n) % n) as usize;
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
            true
        }
    }

    pub(crate) fn sync_tab_line(&mut self) {
        use std::fmt::Write;
        let mut line = String::with_capacity(64);

        for (i, tab) in self.tabs.iter().enumerate() {
            let dirname = tab
                .nav
                .current_dir()
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_else(|| "/".into());

            let marker = if i == self.current { "*" } else { "" };
            let _ = write!(line, "{}:{}{} ", i + 1, dirname, marker);
        }

        for tab in self.tabs.iter_mut() {
            tab.tab_line = line.clone();
        }
    }
}

pub(crate) enum RunaRoot<'a> {
    Single(Box<AppState<'a>>),
    Tabs(TabManager<'a>),
}
