//! Ovelay module to seamless stack widgets, dialogs with each other.
//! Currently handles ShowInfo as a overlay.
//!
//! Can be expanded to hanlde more widget types for more functions.
//!
//! Is used throughout the ui modules and in handlers.rs.

use crate::core::metadata::FileMetadata;
use std::slice;
use std::sync::Arc;

pub(crate) enum Overlay {
    ShowInfo { info: Arc<FileMetadata> },
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

    pub(crate) fn is_open(&self, kind: OverlayKind) -> bool {
        self.iter().any(|o| o.kind() == kind)
    }

    pub(crate) fn remove_kind(&mut self, kind: OverlayKind) {
        self.retain(|o| o.kind() != kind);
    }

    pub(crate) fn is_top(&self, kind: OverlayKind) -> bool {
        self.top().map(|o| o.kind() == kind).unwrap_or(false)
    }

    pub(crate) fn find_show_info_mut(&mut self) -> Option<&mut Arc<FileMetadata>> {
        self.overlays.iter_mut().find_map(|o| match o {
            Overlay::ShowInfo { info } => Some(info),
            _ => None,
        })
    }
}

impl Default for OverlayStack {
    fn default() -> Self {
        Self::new()
    }
}
