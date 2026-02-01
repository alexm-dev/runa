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
    go_to_path: Vec<String>,
    go_to_top: Vec<String>,
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
}

/// Editor configuration options
#[derive(Deserialize, Debug)]
#[serde(default)]
pub(crate) struct Editor {
    cmd: String,
}

/// Public methods for accessing input configuration options
impl Keys {
    #[inline]
    pub(crate) fn open_file(&self) -> &[String] {
        &self.open_file
    }

    #[inline]
    pub(crate) fn go_up(&self) -> &[String] {
        &self.go_up
    }

    #[inline]
    pub(crate) fn go_down(&self) -> &[String] {
        &self.go_down
    }

    #[inline]
    pub(crate) fn go_parent(&self) -> &[String] {
        &self.go_parent
    }

    #[inline]
    pub(crate) fn go_into_dir(&self) -> &[String] {
        &self.go_into_dir
    }

    #[inline]
    pub(crate) fn go_to_path(&self) -> &[String] {
        &self.go_to_path
    }

    #[inline]
    pub(crate) fn go_to_top(&self) -> &[String] {
        &self.go_to_top
    }

    #[inline]
    pub(crate) fn quit(&self) -> &[String] {
        &self.quit
    }

    #[inline]
    pub(crate) fn delete(&self) -> &[String] {
        &self.delete
    }

    #[inline]
    pub(crate) fn copy(&self) -> &[String] {
        &self.copy
    }

    #[inline]
    pub(crate) fn paste(&self) -> &[String] {
        &self.paste
    }

    #[inline]
    pub(crate) fn rename(&self) -> &[String] {
        &self.rename
    }

    #[inline]
    pub(crate) fn create(&self) -> &[String] {
        &self.create
    }

    #[inline]
    pub(crate) fn create_directory(&self) -> &[String] {
        &self.create_directory
    }

    #[inline]
    pub(crate) fn move_file(&self) -> &[String] {
        &self.move_file
    }

    #[inline]
    pub(crate) fn filter(&self) -> &[String] {
        &self.filter
    }

    #[inline]
    pub(crate) fn toggle_marker(&self) -> &[String] {
        &self.toggle_marker
    }

    #[inline]
    pub(crate) fn show_info(&self) -> &[String] {
        &self.show_info
    }

    #[inline]
    pub(crate) fn find(&self) -> &[String] {
        &self.find
    }

    #[inline]
    pub(crate) fn clear_markers(&self) -> &[String] {
        &self.clear_markers
    }

    #[inline]
    pub(crate) fn clear_filter(&self) -> &[String] {
        &self.clear_filter
    }

    #[inline]
    pub(crate) fn alternate_delete(&self) -> &[String] {
        &self.alternate_delete
    }
}

/// Default input configuration options
impl Default for Keys {
    fn default() -> Self {
        Keys {
            open_file: vec!["Enter".into()],
            go_up: vec!["k".into(), "Up".into()],
            go_down: vec!["j".into(), "Down".into()],
            go_parent: vec!["h".into(), "Left".into(), "Backspace".into()],
            go_into_dir: vec!["l".into(), "Right".into()],

            go_to_path: vec!["p".into()],
            go_to_top: vec!["g".into()],

            quit: vec!["q".into(), "Esc".into()],

            delete: vec!["d".into()],
            copy: vec!["y".into()],
            paste: vec!["p".into()],
            rename: vec!["r".into()],
            create: vec!["n".into()],
            create_directory: vec!["Shift+n".into()],
            move_file: vec!["m".into()],
            filter: vec!["f".into()],
            toggle_marker: vec![" ".into()],
            show_info: vec!["i".into()],
            find: vec!["s".into()],

            clear_markers: vec!["Ctrl+c".into()],
            clear_filter: vec!["Ctrl+f".into()],

            alternate_delete: vec!["Ctrl+d".into()],
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
