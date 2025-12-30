//! Configuration options for runa
//!
//! This module defines all configuration options and deserializes them
//! from the runa.toml using serde.
//!
//! Each config struct corresponds to a top-level key in the runa.toml.

pub mod display;
pub mod input;
pub mod theme;

pub use display::Display;
pub use input::{Editor, Keys};
pub use theme::Theme;

use serde::Deserialize;
use std::collections::HashSet;
use std::ffi::OsString;
use std::sync::Arc;
use std::{fs, io, path::PathBuf};

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct RawConfig {
    dirs_first: bool,
    show_hidden: bool,
    show_system: bool,
    case_insensitive: bool,
    always_show: Vec<String>,
    display: Display,
    theme: Theme,
    editor: Editor,
    keys: Keys,
}

#[derive(Debug)]
pub struct Config {
    dirs_first: bool,
    show_hidden: bool,
    show_system: bool,
    case_insensitive: bool,
    always_show: Arc<HashSet<OsString>>,
    display: Display,
    theme: Theme,
    editor: Editor,
    keys: Keys,
}

impl From<RawConfig> for Config {
    fn from(raw: RawConfig) -> Self {
        Self {
            dirs_first: raw.dirs_first,
            show_hidden: raw.show_hidden,
            show_system: raw.show_system,
            case_insensitive: raw.case_insensitive,
            always_show: Arc::new(
                raw.always_show
                    .into_iter()
                    .map(OsString::from)
                    .collect::<HashSet<_>>(),
            ),
            display: raw.display,
            theme: raw.theme,
            editor: raw.editor,
            keys: raw.keys,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::default_path();

        if !path.exists() {
            eprintln!("No config file found at {:?}", path);
            eprintln!(
                "Tip: Run 'runa --init' or '--init-minimal' to generate a default configuration."
            );
            eprintln!("Starting with internal defaults...\n");
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<RawConfig>(&content) {
                Ok(raw) => raw.into(),
                Err(e) => {
                    eprintln!("Error parsing config: {}", e);
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    // Getters

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

    pub fn always_show(&self) -> &Arc<HashSet<OsString>> {
        &self.always_show
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
        if let Ok(path) = std::env::var("RUNA_CONFIG") {
            return PathBuf::from(path);
        }
        if let Some(home) = dirs::home_dir() {
            return home.join(".config/runa/runa.toml");
        }
        PathBuf::from("runa.toml")
    }

    pub fn generate_default(path: &PathBuf, minimal: bool) -> std::io::Result<()> {
        if path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("Config file already exists at {:?}", path),
            ));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let full_toml = r##"# runa.toml - default configuration for runa

# Note:
# Commented values are the internal defaults of runa
# Use hex codes (eg. "#333333") or terminal colors ("cyan")

# General behavior
dirs_first = true
show_hidden = false
# show_system = false
case_insensitive = true
# always_show = []

[display]
# selection_marker = true
# dir_marker = true
borders = "split"
# border_shape = "square"
# titles = false
separators = true
parent = true
preview = true
preview_underline = true
# preview_underline_color = false
entry_padding = 1
# scroll_padding = 5

# [display.layout]
# parent = 20
# main = 40
# preview = 40

# [display.info]
# name = true
# file_type = true
# size = true
# modified = true
# perms = true

[theme]
selection_icon = ""

[theme.selection]
# fg = "default"
bg = "#333333"

[theme.accent]
fg = "#353536"
# bg = "default"

# [theme.entry]
# fg = "default"
# bg = "default"

[theme.directory]
fg = "blue"
# bg = "default"

# [theme.separator]
# fg = "default"
# bg = "default"

# [theme.parent]
# fg = "default"
# bg = "default"
# selection_fg = "default"
# selection_bg = "default"

# [theme.preview]
# fg = "default"
# bg = "default"
# selection_fg = "default"
# selection_bg = "default"

# [theme.underline]
# fg = "default"
# bg = "default"

[theme.path]
fg = "magenta"
# bg = "default"

# [theme.marker]
# icon = "*"
# fg = "default"
# bg = "default"

# [theme.widget]
# size = "medium"           # "small", "medium", "large" or [w ,h] or { w = 30, y = 30 }.
# position = "center"       # "center", "top_left", "bottomright", or [x, y] (percent) or { x = 42, y = 80 }.
# confirm_size = "large"
# color.fg = "default"
# color.bg = "default"
# border.fg = "default"
# border.bg = "default"

# [theme.status_line]
# fg = "default"
# bg = "default"

# [theme.info]
# color.fg = "default"
# color.bg = "default"
# border.fg = "default"
# border.bg = "default"
# title.fg = "default"
# title.bg = "default"
# position = "bottom_left"

[editor]
# cmd = "nvim"

# [keys]
# open_file = ["Enter"]
# go_up = ["k", "Up Arrow"]
# go_down = ["j", "Down Arrow"]
# go_parent = ["h", "Left Arrow", "Backspace"]
# go_into_dir = ["l", "Right Arrow"]
# quit = ["q", "Esc"]
# delete = ["d"]
# copy = ["y"]
# paste = ["p"]
# rename = ["r"]
# create = ["n"]
# create_directory = ["Shift+n"]
# filter = ["f"]
# toggle_marker = [" "]     # " " - indicates space bar
# info = ["i"]
"##;

        let minimal_toml = r##"# runa.toml - minimal configuration
# Only the essentials. The rest uses internal defaults.

dirs_first = true
show_hidden = false

[display]
borders = "split"
entry_padding = 1

[theme]
selection_icon = ""

[theme.selection]
bg = "#333333"

[theme.accent]
fg = "#353536"

[theme.directory]
fg = "blue"

[theme.path]
fg = "magenta"

[editor]
# cmd = "nvim"
"##;

        let content = if minimal { minimal_toml } else { full_toml };

        fs::write(path, content)?;
        println!(
            "{} Default config generated at {:?}",
            if minimal { "Minimal" } else { "Full" },
            path
        );
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            dirs_first: true,
            show_hidden: false,
            show_system: false,
            case_insensitive: true,
            always_show: Arc::new(HashSet::new()),
            display: Display::default(),
            theme: Theme::default(),
            editor: Editor::default(),
            keys: Keys::default(),
        }
    }
}
