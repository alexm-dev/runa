//! Configuration options for runa
//!
//! This module holds the submodules and structs necessary to load and represent
//! configuration options for runa, including display settings, input keybindings,

pub(crate) mod display;
pub(crate) mod input;
pub(crate) mod load;
pub(crate) mod presets;
pub(crate) mod theme;

pub(crate) use display::Display;
pub(crate) use input::{Editor, Keys};
pub(crate) use load::Config;
pub(crate) use theme::Theme;
