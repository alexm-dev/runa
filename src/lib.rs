//! Internal library crate for runa.
//!
//! The shipped application is the `rn` binary (`src/main.rs`).
//!
//! This library exists to share code between targets (binary, tests) and to keep modules organized.
//! This API is only used to build the `rn` binary and is not considered a library for external use.

pub mod app;
pub mod config;
pub mod core;
pub mod ui;
pub mod utils;
