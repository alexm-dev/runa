//! Miscellaneous utility functions for runa.
//!
//! This modules hodl the [helpers] submodule, which provides commonly used utilities such as:
//! - Color parsing
//! - Opening a file/path in the chosen editor
//! - Computing an unused path for core/workers
//! - Shortening the home directory path to "~"
//!
//! All of these utilities are used throughout runa for convenience and code clarity.

pub mod cli;
pub mod helpers;

pub use helpers::{
    DEFAULT_FIND_RESULTS, as_path_op, clean_display_path, copy_recursive, expand_home_path,
    get_home, get_unused_path, open_in_editor, parse_color, readable_path, shorten_home_path,
};
