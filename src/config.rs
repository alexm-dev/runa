//! Configuration options for runner
//!
//! This module defines all configuration options and deserializes them
//! from the `runner.toml` using `serde`.
//!
//! Each config struct corresponds to a top-level key in the `runner.toml`.

use serde::Deserialize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    dirs_first: bool,
    show_hidden: bool,
    show_system: bool,
    case_insensitive: bool,
    display: Display,
    theme: Theme,
    editor: Editor,
    keys: Keys,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Display {
    show_selection_marker: bool,
    show_dir_marker: bool,
    borders: bool,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Theme {
    background: String,
    accent_color: String,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Keys {
    open_file: Vec<String>,
    go_up: Vec<String>,
    go_down: Vec<String>,
    go_parent: Vec<String>,
    go_into_dir: Vec<String>,
    quit: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Editor {
    cmd: String,
}

impl Config {
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path)
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
        {
            Some(cfg) => cfg,
            None => {
                println!("Config file missing or invalid, using defaults.");
                Config::default()
            }
        }
    }

    pub fn dirs_first(&self) -> bool {
        self.dirs_first
    }
    pub fn show_hidden(&self) -> bool {
        self.show_hidden
    }
    pub fn show_system(&self) -> bool {
        self.show_system
    }
    pub fn case_insensitive(&self) -> bool {
        self.case_insensitive
    }
    pub fn display(&self) -> &Display {
        &self.display
    }
    pub fn theme(&self) -> &Theme {
        &self.theme
    }
    pub fn editor(&self) -> &Editor {
        &self.editor
    }
    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    pub fn default_path() -> PathBuf {
        if let Ok(path) = std::env::var("RUNNER_CONFIG") {
            return PathBuf::from(path);
        }
        if let Some(home) = dirs::home_dir() {
            return home.join(".config/runner/runner.toml");
        }
        PathBuf::from("runner.toml")
    }

    pub fn generate_default(path: &PathBuf) -> std::io::Result<()> {
        if path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("Config file already exists at {:?}", path),
            ));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let default_toml = r#"
dirs_first = true
show_hidden = false
"#;
        fs::write(path, default_toml)?;
        println!("Default config generated at {:?}", path);
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            dirs_first: true,
            show_hidden: false,
            show_system: false,
            case_insensitive: false,
            display: Display::default(),
            theme: Theme::default(),
            editor: Editor::default(),
            keys: Keys::default(),
        }
    }
}

impl Display {
    // pub fn show_selection_marker(&self) -> bool {
    //     self.show_selection_marker
    // }
    pub fn show_dir_marker(&self) -> bool {
        self.show_dir_marker
    }
    pub fn borders(&self) -> bool {
        self.borders
    }
}

impl Default for Display {
    fn default() -> Self {
        Display {
            show_selection_marker: true,
            show_dir_marker: true,
            borders: true,
        }
    }
}

impl Theme {
    // pub fn background(&self) -> &str {
    //     &self.background
    // }
    pub fn accent_color(&self) -> &str {
        &self.accent_color
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            background: "default".into(),
            accent_color: "default".into(),
        }
    }
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
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            open_file: vec!["Enter".into()],
            go_up: vec!["k".into(), "Up Arrow".into()],
            go_down: vec!["j".into(), "Down Arrow".into()],
            go_parent: vec!["h".into(), "Left Arrow".into(), "Backspace".into()],
            go_into_dir: vec!["l".into(), "Right Arrow".into()],
            quit: vec!["q".into(), "Esc".into()],
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
