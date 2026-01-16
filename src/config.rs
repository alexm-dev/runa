//! Configuration options for runa
//!
//! This module holds the submodules and structs necessary to load and represent
//! configuration options for runa, including display settings, input keybindings,

pub mod display;
pub mod input;
pub mod load;
pub mod presets;
pub mod theme;

pub use display::Display;
pub use input::{Editor, Keys};
pub use load::Config;
pub use theme::Theme;
