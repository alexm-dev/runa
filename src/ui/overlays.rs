//! Ovelay module to seamless stack widgets, dialogs with each other.
//! Currently handles ShowInfo as a overlay.
//!
//! Can be expanded to hanlde more widget types for more functions.
//!
//! Is used throughout the ui modules and in handlers.rs.

use crate::core::FileInfo;
use std::slice;

#[derive(Clone)]
pub(crate) enum Overlay {
    ShowInfo { info: FileInfo },
    Message { text: String },
}

pub(crate) struct OverlayStack {
    overlays: Vec<Overlay>,
}

impl OverlayStack {
    pub(crate) fn new() -> Self {
        Self {
            overlays: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, overlay: Overlay) {
        self.overlays.push(overlay);
    }

    pub(crate) fn pop(&mut self) -> Option<Overlay> {
        self.overlays.pop()
    }

    pub(crate) fn top(&self) -> Option<&Overlay> {
        self.overlays.last()
    }

    pub(crate) fn iter(&self) -> slice::Iter<'_, Overlay> {
        self.overlays.iter()
    }

    pub(crate) fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Overlay) -> bool,
    {
        self.overlays.retain(f);
    }

    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut Overlay> {
        self.overlays.get_mut(index)
    }

    pub(crate) fn find_index<F>(&self, f: F) -> Option<usize>
    where
        F: FnMut(&Overlay) -> bool,
    {
        self.overlays.iter().position(f)
    }
}

impl Default for OverlayStack {
    fn default() -> Self {
        Self::new()
    }
}
