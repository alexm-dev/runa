//! Terminal User Interface rendering logic.
//!
//! This module handles all layout, pane arrangement, and rendering for the runa UI.
//! It coordinates drawing of the parent, main, and preview panes, as well as input dialogs and the status bar.
//!
//! UI configuration is driven by the applications display/theme settings and adapts to different layouts: unified, split, or normal views.
//!
//! Functions:
//! - frame_layout: Computes the layout of a frame without drawing it.
//! - render: Draws the entire UI to a frame from that layout.
//!
//! See submodules [panes] and [widgets] for detailed drawing functions.

pub(crate) mod icons;
pub(crate) mod panes;
pub(crate) mod render;
pub(crate) mod widgets;

pub(crate) use render::{frame_layout, render};
