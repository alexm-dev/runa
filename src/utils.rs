//! Misc utilits functions for color parsing and external editor for opening files with.

pub mod cli;
pub mod formatter;
pub mod helpers;

pub use formatter::{
    Formatter, format_attributes, format_file_size, format_file_time, format_file_type,
};
pub use helpers::{
    DEFAULT_FIND_RESULTS, get_unused_path, open_in_editor, parse_color, shorten_home_path,
};
