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

const FULL_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/config/runa_full.toml"
));

const MINIMAL_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/config/runa_minimal.toml"
));

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
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        toml::from_str::<RawConfig>(&content)
            .map(|mut raw| {
                raw.theme = raw.theme.with_overrides();
                raw.into()
            })
            .unwrap_or_else(|e| {
                eprintln!("Error parsing config: {}", e);
                Self::default()
            })
    }

    crate::getters! {
        general: &InternalGeneral,
        display: &Display,
        theme: &Theme,
        editor: &Editor,
        keys: &Keys,
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
        std::env::var_os("RUNA_CONFIG")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("XDG_CONFIG_HOME").map(|s| PathBuf::from(s).join("runa/runa.toml"))
            })
            .or_else(|| get_home().map(|h| h.join(".config/runa/runa.toml")))
            .unwrap_or_else(|| PathBuf::from("runa.toml"))
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

        let content = if minimal { MINIMAL_TOML } else { FULL_TOML };

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
