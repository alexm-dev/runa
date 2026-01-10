//! Core runtime logic for runa.
//!
//! This module contains the non-UI “engine” pieces used by the application:
//! - [file_manager]: directory traversal and file metadata (see [browse_dir], [FileEntry], [FileInfo]).
//! - [worker]: background work and message passing back into the app state.
//! - [terminal]: terminal setup/teardown and the main crossterm/ratatui event loop.
//! - [find]: fuzzy find helpers used by runa.
//!
//! Most callers will import [browse_dir], [FileEntry], and [FileInfo] from this module.

pub mod fm;
pub mod formatter;
pub mod proc;
pub mod terminal;
pub mod worker;

pub use fm::{FileEntry, FileInfo, FileType, browse_dir};
pub use formatter::{
    Formatter, format_attributes, format_file_size, format_file_time, format_file_type,
    preview_directory, safe_read_preview, sanitize_to_exact_width,
};
pub use proc::{FindResult, find, preview_bat};
