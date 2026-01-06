//! Core runtime logic for runa.
//!
//! This module contains the non-UI “engine” pieces used by the application:
//! - [file_manager]: directory traversal and file metadata (see [browse_dir], [FileEntry], [FileInfo]).
//! - [worker]: background work and message passing back into the app state.
//! - [terminal]: terminal setup/teardown and the main crossterm/ratatui event loop.
//! - [find]: fuzzy find helpers used by runa.
//!
//! Most callers will import [browse_dir], [FileEntry], and [FileInfo] from this module.

pub mod file_manager;
pub mod find;
pub mod terminal;
pub mod worker;

pub use file_manager::{FileEntry, FileInfo, browse_dir};
pub use find::{FindResult, find};
