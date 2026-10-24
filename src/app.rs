//! Application module.

pub(crate) mod actions;
mod clipboard;
pub(crate) mod keymap;
pub(crate) mod metadata;
pub(crate) mod nav;
mod parent;
pub(crate) mod preview;
mod root;
mod state;
pub(crate) mod tab;

pub(crate) use clipboard::Clipboard;
pub(crate) use nav::NavState;
pub(crate) use parent::ParentState;
pub(crate) use preview::{PreviewData, PreviewState};
pub(crate) use root::{AppContainer, RunaRoot};
pub(crate) use state::{AppState, KeypressResult, LayoutMetrics};
pub(crate) use tab::MAX_TABS;
