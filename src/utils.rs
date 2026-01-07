//! Miscellaneous utility functions for runa.
//!
//! This module contains the [Formatter] and various formatting functions for displaying file info.
//! It also includes the [helpers] submodule, which provides commonly used utilities such as:
//! - Color parsing
//! - Opening a file/path in the chosen editor
//! - Computing an unused path for core/workers
//! - Shortening the home directory path to "~"
//!
//! All of these utilities are used throughout runa for convenience and code clarity.

pub mod cli;
pub mod formatter;
pub mod helpers;

pub use formatter::{
    Formatter, format_attributes, format_file_size, format_file_time, format_file_type,
};
pub use helpers::{
    DEFAULT_FIND_RESULTS, as_path_op, copy_recursive, get_unused_path, open_in_editor, parse_color,
    shorten_home_path,
};
