//! The general configuration settings for runa.
//!
//! This module defines the [General] struct for deserializing
//! general settings from the runa.toml configuration file
//! and the [InternalGeneral] struct for internal use within runa.
//!
//! It includes settings such as display options, case sensitivity,
//! and file handling preferences.

use crate::utils::{DEFAULT_FIND_RESULTS, clamp_find_results};

use serde::Deserialize;

use std::collections::HashSet;
use std::ffi::OsString;
use std::sync::Arc;

#[derive(Deserialize, Debug)]
#[serde(default)]
pub(crate) struct General {
    dirs_first: bool,
    show_hidden: bool,
    show_symlink: bool,
    show_system: bool,
    case_insensitive: bool,
    always_show: Vec<String>,
    #[serde(default = "default_find_results")]
    max_find_results: usize,
    move_to_trash: bool,
}

impl Default for General {
    fn default() -> Self {
        General {
            dirs_first: true,
            show_hidden: true,
            show_symlink: true,
            show_system: false,
            case_insensitive: true,
            always_show: Vec::new(),
            max_find_results: DEFAULT_FIND_RESULTS,
            move_to_trash: true,
        }
    }
}

#[derive(Debug)]
pub(crate) struct InternalGeneral {
    dirs_first: bool,
    show_hidden: bool,
    show_symlink: bool,
    show_system: bool,
    case_insensitive: bool,
    always_show: Arc<HashSet<OsString>>,
    max_find_results: usize,
    move_to_trash: bool,
}

impl From<General> for InternalGeneral {
    fn from(g: General) -> Self {
        let set = g
            .always_show
            .into_iter()
            .map(OsString::from)
            .collect::<HashSet<_>>();
        Self {
            dirs_first: g.dirs_first,
            show_hidden: g.show_hidden,
            show_symlink: g.show_symlink,
            show_system: g.show_system,
            case_insensitive: g.case_insensitive,
            always_show: Arc::new(set),
            max_find_results: clamp_find_results(g.max_find_results),
            move_to_trash: g.move_to_trash,
        }
    }
}

impl InternalGeneral {
    #[inline]
    pub(crate) fn dirs_first(&self) -> bool {
        self.dirs_first
    }

    #[inline]
    pub(crate) fn show_hidden(&self) -> bool {
        self.show_hidden
    }

    #[inline]
    pub(crate) fn show_symlink(&self) -> bool {
        self.show_symlink
    }

    #[inline]
    pub(crate) fn show_system(&self) -> bool {
        self.show_system
    }

    #[inline]
    pub(crate) fn case_insensitive(&self) -> bool {
        self.case_insensitive
    }

    #[inline]
    pub(crate) fn always_show(&self) -> &Arc<HashSet<OsString>> {
        &self.always_show
    }

    #[inline]
    pub(crate) fn max_find_results(&self) -> usize {
        self.max_find_results
    }

    #[inline]
    pub(crate) fn move_to_trash(&self) -> bool {
        self.move_to_trash
    }
}

/// Helper function for default max_find_results
fn default_find_results() -> usize {
    DEFAULT_FIND_RESULTS
}
