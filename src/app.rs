//! Application module.
//!
//! Defines the main application controller and the logic for mutating app state
//! in response to user input. Submodules handle actions, navigation, key mapping,
//! preview pane and parent pane requests.

pub(crate) mod actions;
pub(crate) mod handlers;
pub(crate) mod keymap;
mod nav;
mod parent;
pub(crate) mod preview;
mod state;
pub(crate) mod tab;

pub(crate) use handlers::handle_tab_action;
pub(crate) use nav::NavState;
pub(crate) use parent::ParentState;
pub(crate) use preview::{PreviewData, PreviewState};
pub(crate) use state::{AppState, KeypressResult, LayoutMetrics};
