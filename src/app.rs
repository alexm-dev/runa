//! Application module.
//!
//! Contains the main application controller and the logic that mutates it in response
//! to user input.

pub mod actions;
pub mod find_state;
mod handlers;
mod keymap;
mod nav;
mod parent;
mod preview;
mod state;

pub use nav::NavState;
pub use parent::ParentState;
pub use preview::{PreviewData, PreviewState};
pub use state::{AppState, KeypressResult, LayoutMetrics};
