//! Clipboard logic for runa
//!
//! Module contains the [Clipboard] struct, which holds the copied or cut paths that every tab
//! shares.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The shared clipboard used by all tabs and all states.
#[derive(Default)]
pub(crate) struct Clipboard {
    entries: Option<HashSet<PathBuf>>,
    is_cut: bool,
}

impl Clipboard {
    crate::getters! {
        is_cut: bool,
    }

    pub(crate) fn entries(&self) -> Option<&HashSet<PathBuf>> {
        self.entries.as_ref()
    }

    pub(crate) fn set(&mut self, entries: HashSet<PathBuf>, is_cut: bool) {
        self.entries = Some(entries);
        self.is_cut = is_cut;
    }

    pub(crate) fn remove(&mut self, path: &Path) -> bool {
        let Some(entries) = &mut self.entries else {
            return false;
        };

        let removed = entries.remove(path);
        if entries.is_empty() {
            self.entries = None;
        }

        removed
    }

    pub(crate) fn clear(&mut self) {
        self.entries = None;
        self.is_cut = false;
    }
}
