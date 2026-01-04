//! runa TUI widget module
//!
//! Provides reusable UI components for widgets, panes, separator lines, and status lines,
//! as well as helpers to correctly render and position the input fields of these widgets.
//!
//! Module contains:
//! - Rendering of input dialogs/widgets and confirm dialogs.
//! - General pane blocks, separators and the status line.
//! - Configurable dialog/widget style, position and style

pub mod dialog;
pub mod draw;

pub use dialog::{
    DialogLayout, DialogPosition, DialogSize, DialogStyle, dialog_area, draw_dialog, get_pane_block,
};
pub use draw::*;
