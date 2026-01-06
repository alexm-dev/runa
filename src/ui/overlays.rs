//! Ovelay module to seamless stack widgets, dialogs with each other.
//! Currently handles ShowInfo as a overlay.
//!
//! Can be expanded to hanlde more widget types for more functions.
//!
//! Is used throughout the ui modules and in handlers.rs.

use crate::core::FileInfo;
use std::slice;

#[derive(Clone)]
pub enum Overlay {
    ShowInfo { info: FileInfo },
    Message { text: String },
}

pub struct OverlayStack {
    overlays: Vec<Overlay>,
}

impl OverlayStack {
    pub fn new() -> Self {
        Self {
            overlays: Vec::new(),
        }
    }

    pub fn push(&mut self, overlay: Overlay) {
        self.overlays.push(overlay);
    }

    pub fn pop(&mut self) -> Option<Overlay> {
        self.overlays.pop()
    }

    pub fn top(&self) -> Option<&Overlay> {
        self.overlays.last()
    }

    pub fn top_mut(&mut self) -> Option<&mut Overlay> {
        self.overlays.last_mut()
    }

    pub fn iter(&self) -> slice::Iter<'_, Overlay> {
        self.overlays.iter()
    }

    pub fn len(&self) -> usize {
        self.overlays.len()
    }

    pub fn is_empty(&self) -> bool {
        self.overlays.is_empty()
    }

    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Overlay) -> bool,
    {
        self.overlays.retain(f);
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Overlay> {
        self.overlays.get_mut(index)
    }

    pub fn find_index<F>(&self, f: F) -> Option<usize>
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
