//! Terminal User Interface rendering logic.
//!
//! This module handles all layout, pane arrangement, and rendering for the runa UI.
//! It coordinates drawing of the parent, main, and preview panes, as well as input dialogs and the status bar.
//!
//! UI configuration is driven by the applications display/theme settings and adapts to different layouts: unified, split, or normal views.
//!
//! Functions:
//! - render: Main entry point for rendering the entire UI to a frame.
//! - layout_chunks: Utility for calculating pane positions and widths based on config.
//!
//! See submodules [panes] and [widgets] for detailed drawing functions.

pub mod icons;
pub mod overlays;
pub mod panes;
pub mod render;
pub mod widgets;

pub use render::render;
