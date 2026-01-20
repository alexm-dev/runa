//! Miscellaneous utility functions for runa.
//!
//! This modules hodl the [helpers] submodule, which provides commonly used utilities such as:
//! - Color parsing
//! - Opening a file/path in the chosen editor
//! - Computing an unused path for core/workers
//! - Shortening the home directory path to "~"
//!
//! All of these utilities are used throughout runa for convenience and code clarity.

pub(crate) mod cli;
pub(crate) mod helpers;

pub(crate) use helpers::*;
