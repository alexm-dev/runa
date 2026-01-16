//! Display configuration options for runa
//!
//! This module defines the display configuration options which are read from the runa.toml
//! configuration file.

use crate::ui::widgets::DialogPosition;
use ratatui::widgets::BorderType;
use serde::Deserialize;

/// Display configuration options
///
/// This struct holds various options related to the display of the file manager,
/// including border styles, preview settings, layout ratios, and more.
/// These options can be customized by the user in the configuration file.
/// The struct derives `Deserialize` to allow easy loading from a TOML file,
/// and `Debug` for convenient debugging output.
///
/// Default values are provided for all options to ensure a consistent user experience
/// even if the user does not specify certain settings.
#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Display {
    selection_marker: bool,
    dir_marker: bool,
    borders: BorderStyle,
    border_shape: BorderShape,
    titles: bool,
    icons: bool,
    separators: bool,
    parent: bool,
    preview: bool,
    preview_underline: bool,
    preview_underline_color: bool,
    entry_padding: u8,
    scroll_padding: usize,
    toggle_marker_jump: bool,
    instant_preview: bool,
    preview_options: PreviewOptions,
    layout: LayoutConfig,
    info: ShowInfoOptions,
}

/// Public methods for accessing display configuration options
impl Display {
    pub fn selection_marker(&self) -> bool {
        self.selection_marker
    }

    pub fn dir_marker(&self) -> bool {
        self.dir_marker
    }

    pub fn is_unified(&self) -> bool {
        matches!(self.borders, BorderStyle::Unified)
    }

    pub fn is_split(&self) -> bool {
        matches!(self.borders, BorderStyle::Split)
    }

    pub fn border_shape(&self) -> &BorderShape {
        &self.border_shape
    }

    pub fn titles(&self) -> bool {
        self.titles
    }

    pub fn icons(&self) -> bool {
        self.icons
    }

    pub fn separators(&self) -> bool {
        self.separators
    }

    pub fn parent(&self) -> bool {
        self.parent
    }

    pub fn preview(&self) -> bool {
        self.preview
    }

    pub fn parent_ratio(&self) -> u16 {
        self.layout.parent_ratio()
    }

    pub fn main_ratio(&self) -> u16 {
        self.layout.main_ratio()
    }

    pub fn preview_ratio(&self) -> u16 {
        self.layout.preview_ratio()
    }

    pub fn preview_underline(&self) -> bool {
        self.preview_underline
    }

    pub fn preview_underline_color(&self) -> bool {
        self.preview_underline_color
    }

    pub fn entry_padding(&self) -> u8 {
        self.entry_padding
    }

    pub fn scroll_padding(&self) -> usize {
        self.scroll_padding
    }

    pub fn toggle_marker_jump(&self) -> bool {
        self.toggle_marker_jump
    }

    pub fn instant_preview(&self) -> bool {
        self.instant_preview
    }

    pub fn preview_options(&self) -> &PreviewOptions {
        &self.preview_options
    }

    pub fn info(&self) -> &ShowInfoOptions {
        &self.info
    }

    /// Get padding string based on entry_padding
    pub fn padding_str(&self) -> &'static str {
        // ASCII whitespaces
        match self.entry_padding {
            0 => "",
            1 => " ",
            2 => "  ",
            3 => "   ",
            _ => "    ",
        }
    }
}

/// Default display configuration options
impl Default for Display {
    fn default() -> Self {
        Display {
            selection_marker: true,
            dir_marker: true,
            borders: BorderStyle::Unified,
            border_shape: BorderShape::Square,
            titles: true,
            icons: false,
            separators: true,
            parent: true,
            preview: true,
            layout: LayoutConfig {
                parent: 20,
                main: 40,
                preview: 40,
            },
            preview_underline: true,
            preview_underline_color: false,
            entry_padding: 1,
            scroll_padding: 5,
            toggle_marker_jump: false,
            instant_preview: false,
            preview_options: PreviewOptions::default(),
            info: ShowInfoOptions::default(),
        }
    }
}

/// Layout configuration for the display panes
/// This struct holds the ratio settings for the parent, main, and preview panes
#[derive(Deserialize, Debug)]
pub struct LayoutConfig {
    parent: u16,
    main: u16,
    preview: u16,
}

/// Public methods for accessing layout configuration options
impl LayoutConfig {
    fn parent_ratio(&self) -> u16 {
        self.parent
    }

    fn main_ratio(&self) -> u16 {
        self.main
    }

    fn preview_ratio(&self) -> u16 {
        self.preview
    }
}

/// Options for showing file information in the info dialog
/// This struct holds boolean flags for various file attributes
/// that can be displayed, as well as an optional position for the dialog
///
/// Positions can be specified using the DialogPosition enum
#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct ShowInfoOptions {
    name: bool,
    file_type: bool,
    size: bool,
    modified: bool,
    perms: bool,
    position: Option<DialogPosition>,
}

/// Public methods for accessing show info configuration options
impl ShowInfoOptions {
    pub fn name(&self) -> bool {
        self.name
    }

    pub fn file_type(&self) -> bool {
        self.file_type
    }

    pub fn size(&self) -> bool {
        self.size
    }

    pub fn modified(&self) -> bool {
        self.modified
    }

    pub fn perms(&self) -> bool {
        self.perms
    }

    pub fn position(&self) -> &Option<DialogPosition> {
        &self.position
    }
}

/// Default show info configuration options
impl Default for ShowInfoOptions {
    fn default() -> Self {
        ShowInfoOptions {
            name: true,
            file_type: false,
            size: true,
            modified: true,
            perms: false,
            position: None,
        }
    }
}

/// Preview method options
/// This enum defines the available methods for previewing file contents
/// - Internal: Use the built-in preview functionality
/// - Bat: Use the external 'bat' command for previewing
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PreviewMethod {
    Internal,
    Bat,
}

/// Bat style options for file previewing
/// This enum defines the different styles that can be used with the 'bat' command
/// - Plain: No line numbers or decorations
/// - Numbers: Line numbers only
/// - Full: Full decorations including line numbers and and grid
#[derive(Deserialize, Debug, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum BatStyle {
    #[default]
    Plain,
    Numbers,
    Full,
}

/// Preview configuration options
/// This struct holds various options related to file previewing,
/// including the preview method, bat style, and text wrapping.
/// These options can be customized by the user in the configuration file.
#[derive(Deserialize, Debug, Clone)]
pub struct PreviewOptions {
    #[serde(default = "PreviewOptions::default_method")]
    method: PreviewMethod,
    #[serde(default)]
    style: BatStyle,
    #[serde(default)]
    theme: Option<String>,
    #[serde(default = "PreviewOptions::default_wrap")]
    wrap: bool,
}

/// Public methods for accessing preview configuration options
impl PreviewOptions {
    fn default() -> Self {
        PreviewOptions {
            method: PreviewMethod::Internal,
            style: BatStyle::Plain,
            theme: None,
            wrap: true,
        }
    }

    fn default_method() -> PreviewMethod {
        PreviewMethod::Internal
    }

    fn default_wrap() -> bool {
        true
    }

    pub fn method(&self) -> &PreviewMethod {
        &self.method
    }

    pub fn style(&self) -> BatStyle {
        self.style
    }

    pub fn wrap(&self) -> bool {
        self.wrap
    }

    /// Generate command-line arguments for the 'bat' command based on the preview options
    /// and the given theme name and pane width.
    ///
    /// # Arguments
    /// * `theme_name` - The name of the syntax highlighting theme to use
    /// * `pane_width` - The width of the preview pane in characters
    ///
    /// # Returns
    /// A vector of strings representing the command-line arguments for 'bat'
    pub fn bat_args(&self, default_theme: &str, pane_width: usize) -> Vec<String> {
        let mut args = vec!["--color=always".to_owned(), "--paging=never".to_owned()];
        args.push(format!("--terminal-width={}", pane_width));
        match self.style {
            BatStyle::Plain => args.push("--style=plain".to_owned()),
            BatStyle::Numbers => args.push("--style=numbers".to_owned()),
            BatStyle::Full => args.push("--style=full".to_owned()),
        }

        let theme = self.theme.as_deref().unwrap_or(default_theme);
        args.push("--theme".to_owned());
        args.push(theme.to_owned());

        if self.wrap {
            args.push("--wrap=character".to_owned());
        } else {
            args.push("--wrap=never".to_owned());
        }

        args
    }
}

/// Border style options
/// This enum defines the different border styles that can be used in the UI
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    None,
    Unified,
    Split,
}

/// Border shape options
/// This enum defines the different border shapes that can be used in the UI
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BorderShape {
    Square,
    Rounded,
    Double,
    Thick,
}

/// Public methods for accessing border shape options
impl BorderShape {
    pub fn as_border_type(&self) -> BorderType {
        match self {
            BorderShape::Square => BorderType::Plain,
            BorderShape::Rounded => BorderType::Rounded,
            BorderShape::Double => BorderType::Double,
            BorderShape::Thick => BorderType::Thick,
        }
    }
}
