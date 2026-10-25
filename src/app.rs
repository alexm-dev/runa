//! Application module.

pub(crate) mod actions;
mod clipboard;
mod container;
pub(crate) mod keymap;
pub(crate) mod metadata;
pub(crate) mod nav;
pub(crate) mod overlays;
mod parent;
pub(crate) mod preview;
mod state;
pub(crate) mod tab;

pub(crate) use clipboard::Clipboard;
pub(crate) use container::AppContainer;
pub(crate) use nav::NavState;
pub(crate) use parent::ParentState;
pub(crate) use preview::{PreviewData, PreviewState};
pub(crate) use state::{AppState, KeypressResult, LayoutMetrics};
pub(crate) use tab::MAX_TABS;
