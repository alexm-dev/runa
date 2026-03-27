//! Ovelay module to seamless stack widgets, dialogs with each other.
//! Currently handles ShowInfo as a overlay.
//!
//! Can be expanded to hanlde more widget types for more functions.
//!
//! Is used throughout the ui modules and in handlers.rs.

use crate::core::file_info::CachedFileInfo;
use std::slice;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) enum Overlay {
    ShowInfo { info: Arc<CachedFileInfo> },
    Message { text: String },
    PrefixHelp,
    KeybindHelp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayKind {
    ShowInfo,
    Message,
    PrefixHelp,
    KeybindHelp,
}

pub(crate) struct OverlayStack {
    overlays: Vec<Overlay>,
}

impl Overlay {
    pub(crate) fn kind(&self) -> OverlayKind {
        match self {
            Overlay::ShowInfo { .. } => OverlayKind::ShowInfo,
            Overlay::Message { .. } => OverlayKind::Message,
            Overlay::PrefixHelp => OverlayKind::PrefixHelp,
            Overlay::KeybindHelp => OverlayKind::KeybindHelp,
        }
    }
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

    pub(crate) fn is_open(&self, kind: OverlayKind) -> bool {
        self.iter().any(|o| o.kind() == kind)
    }
}

impl Default for OverlayStack {
    fn default() -> Self {
        Self::new()
    }
}
