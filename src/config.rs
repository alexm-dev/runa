//! Configuration options for runner
//!
//! This module defines all configuration options and deserializes them
//! from the `runner.toml` using `serde`.
//!
//! Each config struct corresponds to a top-level key in the `runner.toml`.

use ratatui::style::{Color, Style};
use serde::Deserialize;
use std::{fs, io, path::PathBuf};

use crate::utils::parse_color;

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Config {
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

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Display {
    selection_marker: bool,
    dir_marker: bool,
    borders: BorderStyle,
    titles: bool,
    separators: bool,
    origin: bool,
    preview: bool,
    origin_ratio: u16,
    main_ratio: u16,
    preview_ratio: u16,
    scroll_padding: usize,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    None,
    Unified,
    Split,
}

#[derive(Deserialize, Debug, Default, Clone, Copy)]
pub struct ColorPair {
    #[serde(deserialize_with = "deserialize_color_field")]
    fg: Color,
    #[serde(deserialize_with = "deserialize_color_field")]
    bg: Color,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Theme {
    #[serde(deserialize_with = "deserialize_color_field")]
    background: Color,
    selection: ColorPair,
    accent: ColorPair,
    entry: ColorPair,
    separator: ColorPair,
    selection_icon: String,
    origin: ColorPair,
    preview: ColorPair,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Keys {
    open_file: Vec<String>,
    go_up: Vec<String>,
    go_down: Vec<String>,
    go_origin: Vec<String>,
    go_into_dir: Vec<String>,
    quit: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Editor {
    cmd: String,
}

impl Config {
    pub fn load() -> Self {
        let path = Self::default_path();

        if !path.exists() {
            eprintln!("No config file found at {:?}", path);
            eprintln!(
                "Tip: Run 'runner --init' or '--init-minimal' to generate a default configuration."
            );
            eprintln!("Starting with internal defaults...\n");
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Error parsing config: {}", e);
                    Self::default()
                }
            },
            Err(_) => Self::default(),
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

    pub fn always_show(&self) -> &[String] {
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
        if let Ok(path) = std::env::var("RUNNER_CONFIG") {
            return PathBuf::from(path);
        }
        if let Some(home) = dirs::home_dir() {
            return home.join(".config/runner/runner.toml");
        }
        PathBuf::from("runner.toml")
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
        let full_toml = r#"# runner.toml - default configuration for runner

# General behavior
dirs_first = true
show_hidden = false
show_system = false
case_insensitive = true
always_show = ["AppData", ".config"]

[display]
selection_marker = true
dir_marker = true
borders = "unified"
titles = false
separators = false
origin = false
preview = false
origin_ratio = 30
main_ratio = 40
preview_ratio = 30
scroll_padding = 5

[theme]
background = "default"
selection_icon = "> "

[theme.selection]
fg = "default"
bg = "default"

[theme.accent]
fg = "default"
bg = "default"

[theme.entry]
fg = "default"
bg = "default"

[theme.separator]
fg = "default"
bg = "default"

[theme.origin]
fg = "default"
bg = "default"

[theme.preview]
fg = "default"
bg = "default"

[editor]
cmd = "nvim"

[keys]
open_file = ["Enter"]
go_up = ["k", "Up Arrow"]
go_down = ["j", "Down Arrow"]
go_origin = ["h", "Left Arrow", "Backspace"]
go_into_dir = ["l", "Right Arrow"]
quit = ["q", "Esc"]
"#;

        let minimal_toml = r#"# runner.toml - minimal configuration
# Only the essentials. The rest uses internal defaults.

dirs_first = true
show_hidden = false

[display]
origin = true
preview = true

[editor]
cmd = "nvim"
"#;

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
            always_show: vec!["AppData".to_string(), ".config".to_string()],
            display: Display::default(),
            theme: Theme::default(),
            editor: Editor::default(),
            keys: Keys::default(),
        }
    }
}

impl Display {
    pub fn selection_marker(&self) -> bool {
        self.selection_marker
    }

    pub fn dir_marker(&self) -> bool {
        self.dir_marker
    }

    // pub fn borders(&self) -> &BorderStyle {
    //     &self.borders
    // }

    pub fn is_unified(&self) -> bool {
        matches!(self.borders, BorderStyle::Unified)
    }

    pub fn is_split(&self) -> bool {
        matches!(self.borders, BorderStyle::Split)
    }

    pub fn titles(&self) -> bool {
        self.titles
    }

    pub fn separators(&self) -> bool {
        self.separators
    }

    pub fn origin(&self) -> bool {
        self.origin
    }

    pub fn preview(&self) -> bool {
        self.preview
    }

    pub fn origin_ratio(&self) -> u16 {
        self.origin_ratio
    }

    pub fn main_ratio(&self) -> u16 {
        self.main_ratio
    }

    pub fn preview_ratio(&self) -> u16 {
        self.preview_ratio
    }

    pub fn scroll_padding(&self) -> usize {
        self.scroll_padding
    }
}

impl Default for Display {
    fn default() -> Self {
        Display {
            selection_marker: true,
            dir_marker: true,
            borders: BorderStyle::Unified,
            titles: false,
            separators: false,
            origin: false,
            preview: true,
            origin_ratio: 25,
            main_ratio: 50,
            preview_ratio: 50,
            scroll_padding: 5,
        }
    }
}

impl ColorPair {
    pub fn as_style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }
}

impl Theme {
    // pub fn background(&self) -> Color {
    //     self.background
    // }

    pub fn accent(&self) -> Style {
        self.accent.as_style()
    }

    pub fn selection(&self) -> Style {
        self.selection.as_style()
    }

    pub fn entry(&self) -> Style {
        self.entry.as_style()
    }

    pub fn separator(&self) -> Style {
        self.separator.as_style()
    }

    pub fn selection_icon(&self) -> &str {
        &self.selection_icon
    }

    pub fn origin(&self) -> Style {
        self.origin.as_style()
    }

    pub fn preview(&self) -> Style {
        self.preview.as_style()
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            background: Color::Reset,
            accent: ColorPair::default(),
            selection: ColorPair::default(),
            entry: ColorPair::default(),
            separator: ColorPair::default(),
            selection_icon: "> ".into(),
            origin: ColorPair::default(),
            preview: ColorPair::default(),
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
    pub fn go_origin(&self) -> &Vec<String> {
        &self.go_origin
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
            go_origin: vec!["h".into(), "Left Arrow".into(), "Backspace".into()],
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

// Helper function to deserialize Theme colors
fn deserialize_color_field<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(parse_color(&s))
}
