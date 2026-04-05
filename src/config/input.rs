//! Input configuration options for runa
//!
//! This module defines the input configuration options which are read from the runa.toml
//! configuration file.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Debug, Hash, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InputKeys {
    OpenFile,
    GoUp,
    GoDown,
    GoParent,
    GoIntoDir,
    Quit,
    Delete,
    Copy,
    Paste,
    Rename,
    Create,
    CreateDirectory,
    MoveFile,
    Filter,
    ToggleMarker,
    ShowInfo,
    Find,
    ClearMarkers,
    ClearClipboard,
    ClearFilter,
    ClearAll,
    AlternateDelete,
    SelectAll,
    PrefixGoTo,
    GoToTop,
    GoToHome,
    GoToPath,
    GoToBottom,
    TabNew,
    TabClose,
    TabNext,
    TabPrev,
    KeybindHelp,
    ScrollUp,
    ScrollDown,
    Sort,
    SortByName,
    SortByNatural,
    SortByExtension,
    SortBySize,
    SortByModified,
    SortByAccessed,
    SortByCreated,
}

/// Input configuration options of all actions
#[derive(Debug)]
pub(crate) struct Keys {
    bindings: HashMap<InputKeys, Vec<String>>,
}

impl<'de> Deserialize<'de> for Keys {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let user_bindings = HashMap::<InputKeys, Vec<String>>::deserialize(deserializer)?;
        let mut keys = Keys::default();
        keys.bindings.extend(user_bindings);
        Ok(keys)
    }
}

crate::key_accessor!(
    open_file => OpenFile,
    go_up => GoUp,
    go_down => GoDown,
    go_parent => GoParent,
    go_into_dir => GoIntoDir,
    quit => Quit,
    delete => Delete,
    copy => Copy,
    paste => Paste,
    rename => Rename,
    create => Create,
    create_directory => CreateDirectory,
    move_file => MoveFile,
    filter => Filter,
    toggle_marker => ToggleMarker,
    show_info => ShowInfo,
    find => Find,
    clear_markers => ClearMarkers,
    clear_clipboard => ClearClipboard,
    clear_filter => ClearFilter,
    clear_all => ClearAll,
    alternate_delete => AlternateDelete,
    select_all => SelectAll,
    prefix_go_to => PrefixGoTo,
    go_to_top => GoToTop,
    go_to_home => GoToHome,
    go_to_path => GoToPath,
    go_to_bottom => GoToBottom,
    tab_new => TabNew,
    tab_close => TabClose,
    tab_next => TabNext,
    tab_prev => TabPrev,
    keybind_help => KeybindHelp,
    scroll_up => ScrollUp,
    scroll_down => ScrollDown,
    sort => Sort,
    sort_by_name => SortByName,
    sort_by_natural => SortByNatural,
    sort_by_extension => SortByExtension,
    sort_by_size => SortBySize,
    sort_by_modified => SortByModified,
    sort_by_accessed => SortByAccessed,
    sort_by_created => SortByCreated,
);

impl Default for Keys {
    fn default() -> Self {
        let defaults = [
            (InputKeys::OpenFile, vec!["enter"]),
            (InputKeys::GoUp, vec!["k", "up"]),
            (InputKeys::GoDown, vec!["j", "down"]),
            (InputKeys::GoParent, vec!["h", "left", "back"]),
            (InputKeys::GoIntoDir, vec!["l", "right"]),
            (InputKeys::Quit, vec!["q", "esc"]),
            (InputKeys::Delete, vec!["d"]),
            (InputKeys::Copy, vec!["y"]),
            (InputKeys::Paste, vec!["p"]),
            (InputKeys::Rename, vec!["r"]),
            (InputKeys::Create, vec!["n"]),
            (InputKeys::CreateDirectory, vec!["N"]),
            (InputKeys::MoveFile, vec!["m"]),
            (InputKeys::Filter, vec!["f"]),
            (InputKeys::ToggleMarker, vec!["<space>"]),
            (InputKeys::ShowInfo, vec!["i"]),
            (InputKeys::Find, vec!["s"]),
            (InputKeys::ClearMarkers, vec!["<c-c>"]),
            (InputKeys::ClearClipboard, vec!["<f2>"]),
            (InputKeys::ClearFilter, vec!["<c-f>"]),
            (InputKeys::ClearAll, vec!["<c-l>"]),
            (InputKeys::SelectAll, vec!["<c-a>"]),
            (InputKeys::AlternateDelete, vec!["<m-d>"]),
            (InputKeys::PrefixGoTo, vec!["g"]),
            (InputKeys::GoToTop, vec!["g"]),
            (InputKeys::GoToHome, vec!["h"]),
            (InputKeys::GoToPath, vec!["p"]),
            (InputKeys::GoToBottom, vec!["G"]),
            (InputKeys::TabNew, vec!["<c-t>"]),
            (InputKeys::TabClose, vec!["<c-w>"]),
            (InputKeys::TabNext, vec!["<c-n>"]),
            (InputKeys::TabPrev, vec!["<c-p>"]),
            (InputKeys::KeybindHelp, vec!["?"]),
            (InputKeys::ScrollUp, vec!["<c-d>"]),
            (InputKeys::ScrollDown, vec!["<c-u>"]),
            (InputKeys::Sort, vec!["o"]),
            (InputKeys::SortByName, vec!["n"]),
            (InputKeys::SortByNatural, vec!["N"]),
            (InputKeys::SortByExtension, vec!["e"]),
            (InputKeys::SortBySize, vec!["s"]),
            (InputKeys::SortByModified, vec!["m"]),
            (InputKeys::SortByAccessed, vec!["a"]),
            (InputKeys::SortByCreated, vec!["c"]),
        ];

        let bindings = defaults
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().map(String::from).collect()))
            .collect();

        Keys { bindings }
    }
}
/// Editor configuration options
#[derive(Deserialize, Debug)]
#[serde(default)]
pub(crate) struct Editor {
    cmd: String,
}

/// Public methods for accessing editor configuration options
impl Editor {
    #[inline]
    pub(crate) fn cmd(&self) -> &str {
        let trimmed = self.cmd.trim();
        if trimmed.is_empty() { "vim" } else { trimmed }
    }

    pub(crate) fn resolved_path(&self) -> Option<PathBuf> {
        which::which(self.cmd()).ok()
    }

    pub(crate) fn exists(&self) -> bool {
        self.resolved_path().is_some()
    }
}

/// Default editor configuration options
impl Default for Editor {
    fn default() -> Self {
        Editor { cmd: "nvim".into() }
    }
}
