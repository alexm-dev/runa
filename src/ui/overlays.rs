//! Ovelay module to seamless stack widgets, dialogs with each other.
//! Currently handles ShowInfo as a overlay.
//!
//! Can be expanded to hanlde more widget types for more functions.
//!
//! Is used throughout the ui modules and in handlers.rs.

use crate::file_manager::FileInfo;

#[derive(Clone)]
pub enum Overlay {
    ShowInfo { info: FileInfo },
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
}

impl Default for OverlayStack {
    fn default() -> Self {
        Self::new()
    }
}
