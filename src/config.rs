//! Configuration options for runner
//!
//! This modules defines all configuration options and deserializes them
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
    pub dirs_first: bool,
    pub show_hidden: bool,
    pub show_system: bool,
    pub case_insensitive: bool,
    pub display: Display,
    pub theme: Theme,
    pub editor: Editor,
    pub keys: Keys,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Display {
    pub show_selection_marker: bool,
    pub show_dir_marker: bool,
    pub borders: bool,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Theme {
    pub background: String,
    pub accent_color: String,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Keys {
    pub open_file: Vec<String>,
    pub go_up: Vec<String>,
    pub go_down: Vec<String>,
    pub go_parent: Vec<String>,
    pub go_into_dir: Vec<String>,
    pub quit: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Editor {
    pub cmd: String,
}

impl Config {
    /// Loads config from
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
        println!("Default config generatet at {:?}", path);
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

impl Default for Display {
    fn default() -> Self {
        Display {
            show_selection_marker: true,
            show_dir_marker: true,
            borders: true,
        }
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

impl Default for Editor {
    fn default() -> Self {
        Editor { cmd: "nvim".into() }
    }
}
