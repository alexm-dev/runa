//! Core runtime logic for runa.
//!
//! This module contains the non-UI “engine” pieces used by the application:
//! - [fm]: directory traversal and file metadata (see [browse_dir], [FileEntry], [FileInfo]).
//! - [formatter]: formatting helpers for displaying file attributes, sizes, times, types, and previews.
//! - [worker]: background work and message passing back into the app state.
//! - [terminal]: terminal setup/teardown and the main crossterm/ratatui event loop.
//! - [proc]: process management for running external commands like `bat`, `fd`.
//!
//! Most callers will import [browse_dir], [FileEntry], and [FileInfo] from this module.

pub(crate) mod fm;
pub(crate) mod formatter;
pub(crate) mod proc;
pub(crate) mod terminal;
pub(crate) mod worker;

pub(crate) use fm::{FileEntry, FileInfo, FileType, browse_dir};
pub(crate) use formatter::Formatter;
pub(crate) use proc::{FindResult, find, preview_bat};
