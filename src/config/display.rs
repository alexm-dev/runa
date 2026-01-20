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
pub(crate) struct Display {
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
    #[inline]
    pub(crate) fn selection_marker(&self) -> bool {
        self.selection_marker
    }

    #[inline]
    pub(crate) fn dir_marker(&self) -> bool {
        self.dir_marker
    }

    #[inline]
    pub(crate) fn is_unified(&self) -> bool {
        matches!(self.borders, BorderStyle::Unified)
    }

    #[inline]
    pub(crate) fn is_split(&self) -> bool {
        matches!(self.borders, BorderStyle::Split)
    }

    #[inline]
    pub(crate) fn border_shape(&self) -> &BorderShape {
        &self.border_shape
    }

    #[inline]
    pub(crate) fn titles(&self) -> bool {
        self.titles
    }

    #[inline]
    pub(crate) fn icons(&self) -> bool {
        self.icons
    }

    #[inline]
    pub(crate) fn separators(&self) -> bool {
        self.separators
    }

    #[inline]
    pub(crate) fn parent(&self) -> bool {
        self.parent
    }

    #[inline]
    pub(crate) fn preview(&self) -> bool {
        self.preview
    }

    #[inline]
    pub(crate) fn parent_ratio(&self) -> u16 {
        self.layout.parent_ratio()
    }

    #[inline]
    pub(crate) fn main_ratio(&self) -> u16 {
        self.layout.main_ratio()
    }

    #[inline]
    pub(crate) fn preview_ratio(&self) -> u16 {
        self.layout.preview_ratio()
    }

    #[inline]
    pub(crate) fn preview_underline(&self) -> bool {
        self.preview_underline
    }

    #[inline]
    pub(crate) fn preview_underline_color(&self) -> bool {
        self.preview_underline_color
    }

    #[inline]
    pub(crate) fn entry_padding(&self) -> u8 {
        self.entry_padding
    }

    #[inline]
    pub(crate) fn scroll_padding(&self) -> usize {
        self.scroll_padding
    }

    #[inline]
    pub(crate) fn toggle_marker_jump(&self) -> bool {
        self.toggle_marker_jump
    }

    #[inline]
    pub(crate) fn instant_preview(&self) -> bool {
        self.instant_preview
    }

    #[inline]
    pub(crate) fn preview_options(&self) -> &PreviewOptions {
        &self.preview_options
    }

    #[inline]
    pub(crate) fn info(&self) -> &ShowInfoOptions {
        &self.info
    }

    /// Get padding string based on entry_padding
    pub(crate) fn padding_str(&self) -> &'static str {
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
pub(crate) struct LayoutConfig {
    parent: u16,
    main: u16,
    preview: u16,
}

/// Public methods for accessing layout configuration options
impl LayoutConfig {
    #[inline]
    fn parent_ratio(&self) -> u16 {
        self.parent
    }

    #[inline]
    fn main_ratio(&self) -> u16 {
        self.main
    }

    #[inline]
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
pub(crate) struct ShowInfoOptions {
    name: bool,
    file_type: bool,
    size: bool,
    modified: bool,
    perms: bool,
    position: Option<DialogPosition>,
}

/// Public methods for accessing show info configuration options
impl ShowInfoOptions {
    #[inline]
    pub(crate) fn name(&self) -> bool {
        self.name
    }

    #[inline]
    pub(crate) fn file_type(&self) -> bool {
        self.file_type
    }

    #[inline]
    pub(crate) fn size(&self) -> bool {
        self.size
    }

    #[inline]
    pub(crate) fn modified(&self) -> bool {
        self.modified
    }

    #[inline]
    pub(crate) fn perms(&self) -> bool {
        self.perms
    }

    #[inline]
    pub(crate) fn position(&self) -> &Option<DialogPosition> {
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
pub(crate) enum PreviewMethod {
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
pub(crate) enum BatStyle {
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
pub(crate) struct PreviewOptions {
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

    #[inline]
    fn default_wrap() -> bool {
        true
    }

    #[inline]
    pub(crate) fn method(&self) -> &PreviewMethod {
        &self.method
    }

    /// Generate command-line arguments for the 'bat' command based on the preview options
    /// and the given theme name and pane width.
    /// # Returns
    /// A vector of strings representing the command-line arguments for 'bat'
    pub(crate) fn bat_args(&self, default_theme: &str, pane_width: usize) -> Vec<String> {
        let mut args = Vec::with_capacity(10);
        args.push("--color=always".to_string());
        args.push("--paging=never".to_string());
        args.push(format!("--terminal-width={}", pane_width));
        args.push(
            match self.style {
                BatStyle::Plain => "--style=plain",
                BatStyle::Numbers => "--style=numbers",
                BatStyle::Full => "--style=full",
            }
            .to_string(),
        );

        args.push("--theme".to_owned());
        args.push(self.theme.as_deref().unwrap_or(default_theme).to_string());

        args.push(
            if self.wrap {
                "--wrap=character"
            } else {
                "--wrap=never"
            }
            .to_string(),
        );

        args
    }
}

/// Border style options
/// This enum defines the different border styles that can be used in the UI
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BorderStyle {
    None,
    Unified,
    Split,
}

/// Border shape options
/// This enum defines the different border shapes that can be used in the UI
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BorderShape {
    Square,
    Rounded,
    Double,
    Thick,
}

/// Public methods for accessing border shape options
impl BorderShape {
    pub(crate) fn as_border_type(&self) -> BorderType {
        match self {
            BorderShape::Square => BorderType::Plain,
            BorderShape::Rounded => BorderType::Rounded,
            BorderShape::Double => BorderType::Double,
            BorderShape::Thick => BorderType::Thick,
        }
    }
}
