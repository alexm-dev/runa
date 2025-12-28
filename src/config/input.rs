//! Input configuration options for runa
//!
//! This module defines the input configuration options which are read from the runa.toml
//! configuration file.

use serde::Deserialize;
use std::vec;

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Keys {
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
    filter: Vec<String>,
    toggle_marker: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Editor {
    cmd: String,
}

impl Keys {
    pub fn open_file(&self) -> &Vec<String> {
        &self.open_file
    }

    pub fn go_up(&self) -> &Vec<String> {
        &self.go_up
    }

    pub fn go_down(&self) -> &Vec<String> {
        &self.go_down
    }

    pub fn go_parent(&self) -> &Vec<String> {
        &self.go_parent
    }

    pub fn go_into_dir(&self) -> &Vec<String> {
        &self.go_into_dir
    }

    pub fn quit(&self) -> &Vec<String> {
        &self.quit
    }

    pub fn delete(&self) -> &Vec<String> {
        &self.delete
    }

    pub fn copy(&self) -> &Vec<String> {
        &self.copy
    }

    pub fn paste(&self) -> &Vec<String> {
        &self.paste
    }

    pub fn rename(&self) -> &Vec<String> {
        &self.rename
    }

    pub fn create(&self) -> &Vec<String> {
        &self.create
    }

    pub fn create_directory(&self) -> &Vec<String> {
        &self.create_directory
    }

    pub fn filter(&self) -> &Vec<String> {
        &self.filter
    }

    pub fn toggle_marker(&self) -> &Vec<String> {
        &self.toggle_marker
    }
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            open_file: vec!["Enter".into()],
            go_up: vec!["k".into(), "Up".into()],
            go_down: vec!["j".into(), "Down".into()],
            go_parent: vec!["h".into(), "Left".into(), "Backspace".into()],
            go_into_dir: vec!["l".into(), "Right".into()],
            quit: vec!["q".into(), "Esc".into()],

            delete: vec!["d".into()],
            copy: vec!["y".into()],
            paste: vec!["p".into()],
            rename: vec!["r".into()],
            create: vec!["n".into()],
            create_directory: vec!["Shift+n".into()],
            filter: vec!["f".into()],
            toggle_marker: vec![" ".into()],
        }
    }
}

impl Editor {
    pub fn cmd(&self) -> &str {
        &self.cmd
    }
}

impl Default for Editor {
    fn default() -> Self {
        Editor { cmd: "nvim".into() }
    }
}
