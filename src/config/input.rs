//! Input configuration options for runa
//!
//! This module defines the input configuration options which are read from the runa.toml
//! configuration file.

use serde::Deserialize;

/// Input configuration options of all actions
#[derive(Deserialize, Debug)]
#[serde(default)]
pub(crate) struct Keys {
    open_file: Vec<String>,
    go_up: Vec<String>,
    go_down: Vec<String>,
    go_parent: Vec<String>,
    go_into_dir: Vec<String>,
    quit: Vec<String>,
    delete: Vec<String>,
    copy: Vec<String>,
    paste: Vec<String>,
    rename: Vec<String>,
    create: Vec<String>,
    create_directory: Vec<String>,
    move_file: Vec<String>,
    filter: Vec<String>,
    toggle_marker: Vec<String>,
    show_info: Vec<String>,
    find: Vec<String>,
    clear_markers: Vec<String>,
    clear_filter: Vec<String>,
    alternate_delete: Vec<String>,
    go_to_top: Vec<String>,
    go_to_home: Vec<String>,
    go_to_path: Vec<String>,
    go_to_bottom: Vec<String>,
    keybind_help: Vec<String>,
}

/// Editor configuration options
#[derive(Deserialize, Debug)]
#[serde(default)]
pub(crate) struct Editor {
    cmd: String,
}

macro_rules! accessor {
    ($($name:ident),+ $(,)?) => {
        impl Keys {
            $(
                #[inline]
                pub(crate) fn $name(&self) -> &[String] {
                    &self.$name
                }
            )+
        }
    };
}

accessor!(
    open_file,
    go_up,
    go_down,
    go_parent,
    go_into_dir,
    quit,
    delete,
    copy,
    paste,
    rename,
    create,
    create_directory,
    move_file,
    filter,
    toggle_marker,
    show_info,
    find,
    clear_markers,
    clear_filter,
    alternate_delete,
    go_to_top,
    go_to_home,
    go_to_path,
    go_to_bottom,
    keybind_help,
);

/// Default input configuration options
impl Default for Keys {
    fn default() -> Self {
        Keys {
            open_file: vec!["enter".into()],
            go_up: vec!["k".into(), "up".into()],
            go_down: vec!["j".into(), "down".into()],
            go_parent: vec!["h".into(), "left".into(), "back".into()],
            go_into_dir: vec!["l".into(), "right".into()],

            quit: vec!["q".into(), "esc".into()],

            delete: vec!["d".into()],
            copy: vec!["y".into()],
            paste: vec!["p".into()],
            rename: vec!["r".into()],
            create: vec!["n".into()],
            create_directory: vec!["N".into()],
            move_file: vec!["m".into()],
            filter: vec!["f".into()],
            toggle_marker: vec!["<space>".into()],
            show_info: vec!["i".into()],
            find: vec!["s".into()],

            clear_markers: vec!["<c-c>".into()],
            clear_filter: vec!["<c-f>".into()],

            alternate_delete: vec!["<c-d>".into()],

            go_to_top: vec!["g".into()],
            go_to_home: vec!["h".into()],
            go_to_path: vec!["p".into()],

            go_to_bottom: vec!["G".into()],

            keybind_help: vec!["?".into()],
        }
    }
}

/// Public methods for accessing editor configuration options
impl Editor {
    #[inline]
    pub(crate) fn cmd(&self) -> &str {
        let trimmed = self.cmd.trim();
        if trimmed.is_empty() { "vim" } else { trimmed }
    }

    pub(crate) fn exists(&self) -> bool {
        which::which(self.cmd()).is_ok()
    }
}

/// Default editor configuration options
impl Default for Editor {
    fn default() -> Self {
        Editor { cmd: "nvim".into() }
    }
}
