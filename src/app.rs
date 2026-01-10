//! Application module.
//!
//! Defines the main application controller and the logic for mutating app state
//! in response to user input. Submodules handle actions, navigation, key mapping,
//! preview pane and parent pane requests.

pub mod actions;
mod handlers;
mod keymap;
mod nav;
mod parent;
pub mod preview;
mod state;

pub use nav::NavState;
pub use parent::ParentState;
pub use preview::{PreviewData, PreviewState};
pub use state::{AppState, KeypressResult, LayoutMetrics};
