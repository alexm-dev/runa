//! The main config loading module for runa.
//!
//! Handles loading and deserializing settings from `runa.toml`.
//!
//! Provides and manages the main [Config] struct, as well as the internal [RawConfig] used for parsing and processing.
//!
//! Also implements default config initialization when `runa.toml` is not present.

use crate::config::Display;
use crate::config::Theme;
use crate::config::{Editor, Keys};
use crate::config::{General, InternalGeneral};
use crate::utils::get_home;

use serde::Deserialize;
use std::{fs, io, path::PathBuf};

/// Raw configuration as read from the toml file
/// This struct is deserialized directly from the toml file.
/// It uses owned types and is then converted into the main [Config] struct.
#[derive(Deserialize, Debug)]
#[serde(default)]
pub(crate) struct RawConfig {
    general: General,
    display: Display,
    theme: Theme,
    editor: Editor,
    keys: Keys,
}

/// Default values for RawConfig
/// These are the same as the internal defaults used by runa.
impl Default for RawConfig {
    fn default() -> Self {
        RawConfig {
            general: General::default(),
            display: Display::default(),
            theme: Theme::default(),
            editor: Editor::default(),
            keys: Keys::default(),
        }
    }
}

/// Main configuration struct for runa
/// This struct holds the processed configuration options used by runa.
#[derive(Debug)]
pub(crate) struct Config {
    general: InternalGeneral,
    display: Display,
    theme: Theme,
    editor: Editor,
    keys: Keys,
}

/// Conversion from RawConfig to Config
/// This handles any necessary processing of the raw values
impl From<RawConfig> for Config {
    fn from(raw: RawConfig) -> Self {
        Self {
            general: InternalGeneral::from(raw.general),
            display: raw.display,
            theme: raw.theme,
            editor: raw.editor,
            keys: raw.keys,
        }
    }
}

/// Public methods for loading and accessing the configuration
impl Config {
    /// Load configuration from the default path
    /// If the file does not exist or fails to parse, returns the default configuration.
    /// Also applies any necessary overrides to the theme after loading.
    ///
    /// Called by entry point to load config at startup.
    pub(crate) fn load() -> Self {
        let path = Self::default_path();

        if !path.exists() {
            eprintln!("No config file found at {:?}", path);
            eprintln!("Tip: Run 'rn --init' or '--init-full' to generate a default configuration.");
            eprintln!("Starting with internal defaults...\n");
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<RawConfig>(&content) {
                Ok(mut raw) => {
                    raw.theme = raw.theme.with_overrides();
                    raw.into()
                }
                Err(e) => {
                    eprintln!("Error parsing config: {}", e);
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    // Getters

    #[inline]
    pub(crate) fn general(&self) -> &InternalGeneral {
        &self.general
    }

    #[inline]
    pub(crate) fn display(&self) -> &Display {
        &self.display
    }

    #[inline]
    pub(crate) fn theme(&self) -> &Theme {
        &self.theme
    }

    #[inline]
    pub(crate) fn editor(&self) -> &Editor {
        &self.editor
    }

    #[inline]
    pub(crate) fn keys(&self) -> &Keys {
        &self.keys
    }

    pub(crate) fn bat_args_for_preview(&self, pane_width: usize) -> Vec<String> {
        self.display
            .preview_options()
            .bat_args(self.theme.bat_theme_name(), pane_width)
    }

    /// Determine the default configuration file path.
    /// Checks the RUNA_CONFIG environment variable first,
    /// Checks for XDG_CONFIG_HOME after,
    /// then defaults to ~/.config/runa/runa.toml,
    pub(crate) fn default_path() -> PathBuf {
        if let Ok(path) = std::env::var("RUNA_CONFIG") {
            return PathBuf::from(path);
        }

        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg_config).join("runa/runa.toml");
        }

        if let Some(home) = get_home() {
            return home.join(".config/runa/runa.toml");
        }
        PathBuf::from("runa.toml")
    }

    /// Generate a default configuration file at the specified path.
    /// If the file already exists, returns an error.
    pub(crate) fn generate_default(path: &PathBuf, minimal: bool) -> std::io::Result<()> {
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
[general]
dirs_first = true
show_hidden = true
# show_symlink = true
# show_system = false
case_insensitive = true
# always_show = []
# max_find_results = 2000
# move_to_trash = true

[display]
# selection_marker = true
# dir_marker = true
borders = "unified"
border_shape = "square"
# titles = true
# icons = false
# separators = true
# parent = true
# preview = true
preview_underline = true
# preview_underline_color = false
# entry_padding = 1
# scroll_padding = 5
# toggle_marker_jump = false
# instant_preview = false
# entry_count = "footer"

[display.preview_options]
method = "internal"
# bat related options if method = "bat"
# theme = "TwoDark"
# style = "plain"
# wrap = true

# [display.layout]
# parent = 20
# main = 40
# preview = 40

# [display.info]
# name = true
# file_type = false
# size = true
# modified = true
# perms = false
# position = "default"

[theme]
name = "default"
symlink = "default"
selection_icon = ""

# [theme.selection]
# fg = "default"
# bg = "default"

# [theme.accent]
# fg = "default"
# bg = "default"

# [theme.entry]
# fg = "default"
# bg = "default"

# [theme.directory]
# fg = "blue"
# bg = "default"

# [theme.separator]
# fg = "default"
# bg = "default"

# [theme.parent]
# fg = "default"
# bg = "default"
# selection_mode = "on"
# selection.fg = "default"
# selection.bg = "default"

# [theme.preview]
# fg = "default"
# bg = "default"
# selection_mode = "on"
# selection.fg = "default"
# selection.bg = "default"

# [theme.underline]
# fg = "default"
# bg = "default"

# [theme.path]
# fg = "magenta"
# bg = "default"

# [theme.marker]
# icon = "*"
# fg = "default"
# bg = "default"
# clipboard.fg = "default"
# clipboard.bg = "default"

# [theme.widget]
# size = "medium"           # "small", "medium", "large" or [w ,h] or { w = 30, y = 30 }.
# position = "center"       # "center", "top_left", "bottomright", or [x, y] (percent) or { x = 42, y = 80 }.
# confirm_size = "large"
# move_size = [70, 14]
# find_size = "medium"
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

# [editor]
# cmd = "nvim"

# [keys]
# open_file = ["Enter"]
# go_up = ["k", "Up"]
# go_down = ["j", "Down"]
# go_parent = ["h", "Left", "Backspace"]
# go_into_dir = ["l", "Right"]
# quit = ["q", "Esc"]
# delete = ["d"]
# copy = ["y"]
# paste = ["p"]
# rename = ["r"]
# create = ["n"]
# create_directory = ["Shift+n"]
# move_file = ["m"]
# filter = ["f"]
# toggle_marker = [" "]     # " " - indicates space bar
# info = ["i"]
# find = ["s"]
# clear_markers = ["Ctrl+c"]
# clear_filter = ["Ctrl+f"]
# alternate_delete = ["Ctrl+d"]
"##;

        let minimal_toml = r##"# runa.toml - minimal configuration
# Only the essentials. The rest uses internal defaults.

[general]
dirs_first = true
show_hidden = true

[display]
borders = "unified"
icons = false
entry_padding = 1

[theme]
name = "default"
selection_icon = ""
accent.fg = "default"

# [theme.path]
# fg = "magenta"

# [editor]
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

/// Default configuration options
impl Default for Config {
    fn default() -> Self {
        Config {
            general: InternalGeneral::from(General::default()),
            display: Display::default(),
            theme: Theme::default(),
            editor: Editor::default(),
            keys: Keys::default(),
        }
    }
}
