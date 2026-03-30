//! Core runtime logic for runa.
//!
//! This module contains the non-UI “engine” pieces used by the application:
//! - [fm]: directory traversal and file metadata (see [browse_dir], [FileEntry]).
//! - [formatter]: formatting helpers for displaying file attributes, sizes, times, types, and previews.
//! - [worker]: background work and message passing back into the RunaRoot struct.
//! - [terminal]: terminal setup/teardown and the main crossterm/ratatui event loop.
//! - [proc]: process management for running external commands like `bat`, `fd`.
//! - [metadata]: file metadata extraction and caching, including file properties.
//!
//! Most callers will import [browse_dir], [FileEntry], from this module.

pub(crate) mod fm;
pub(crate) mod formatter;
pub(crate) mod metadata;
pub(crate) mod proc;
pub(crate) mod terminal;
pub(crate) mod worker;

pub(crate) use fm::{FileEntry, browse_dir};
pub(crate) use formatter::Formatter;
pub(crate) use proc::{FindResult, find, preview_bat};
