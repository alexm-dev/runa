//! Display configuration options for runa
//!
//! This module defines the display configuration options which are read from the runa.toml
//! configuration file.

use crate::ui::widgets::DialogPosition;
use chrono::format::{Item, StrftimeItems};
use ratatui::widgets::BorderType;
use serde::{Deserialize, Deserializer};

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
    #[serde(
        default = "Display::default_sort_date_format",
        deserialize_with = "deserialize_sort_date_format"
    )]
    sort_date_format: String,
    preview_options: PreviewOptions,
    layout: LayoutConfig,
    info: ShowInfoOptions,
    status: StatusElements,
}

/// Public methods for accessing display configuration options
impl Display {
    crate::getters! {
        selection_marker: bool,
        dir_marker: bool,
        border_shape: &BorderShape,
        titles: bool,
        icons: bool,
        separators: bool,
        parent: bool,
        preview: bool,
        preview_underline: bool,
        preview_underline_color: bool,
        scroll_padding: usize,
        toggle_marker_jump: bool,
        instant_preview: bool,
        sort_date_format: &str,
        preview_options: &PreviewOptions,
        info: &ShowInfoOptions,
        status: &StatusElements,
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
    pub(crate) fn is_no_borders(&self) -> bool {
        matches!(self.borders, BorderStyle::None)
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

    fn default_sort_date_format() -> String {
        "%b %e %H:%M".to_string()
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
            selection_marker: false,
            dir_marker: true,
            borders: BorderStyle::Unified,
            border_shape: BorderShape::Square,
            titles: true,
            icons: false,
            separators: true,
            parent: true,
            preview: true,
            preview_underline: true,
            preview_underline_color: false,
            entry_padding: 1,
            scroll_padding: 5,
            toggle_marker_jump: false,
            instant_preview: true,
            sort_date_format: Display::default_sort_date_format(),
            layout: LayoutConfig::default(),
            preview_options: PreviewOptions::default(),
            info: ShowInfoOptions::default(),
            status: StatusElements::default(),
        }
    }
}

/// Layout configuration for the display panes
/// This struct holds the ratio settings for the parent, main, and preview panes
#[derive(Deserialize, Debug)]
#[serde(default)]
pub(crate) struct LayoutConfig {
    parent: u16,
    main: u16,
    preview: u16,
}

/// Public methods for accessing layout configuration options
impl LayoutConfig {
    crate::getters! {
        parent_ratio => parent: u16,
        main_ratio => main: u16,
        preview_ratio => preview: u16,
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            parent: 20,
            main: 40,
            preview: 40,
        }
    }
}

/// Options for showing file information in the info dialog
/// This struct holds boolean flags for various file attributes
/// that can be displayed, as well as an optional position for the dialog
///
/// Positions can be specified using the DialogPosition enum
#[derive(Debug)]
pub(crate) struct ShowInfoOptions {
    name: bool,
    file_type: bool,
    size: bool,
    modified: bool,
    created: bool,
    accessed: bool,
    perms: bool,
    #[cfg(unix)]
    owner: bool,
    #[cfg(unix)]
    group: bool,
    position: Option<DialogPosition>,
    status_bar: bool,
    format: Option<String>,
    date_format: String,
    segments: Vec<StatusSegment>,
}

impl ShowInfoOptions {
    pub(crate) fn init_status_format(&mut self) {
        if let Some(fmt) = &self.format {
            self.segments = parse_status_format(fmt);
        }
    }

    crate::getters! {
        name: bool,
        file_type: bool,
        size: bool,
        modified: bool,
        created: bool,
        accessed: bool,
        perms: bool,

        #[cfg(unix)]
        owner: bool,
        #[cfg(unix)]
        group: bool,

        position: &Option<DialogPosition>,
        status_bar: bool,
        date_format: &str,
        segments: &[StatusSegment],

    }

    fn validate_date_format(fmt: Option<String>) -> String {
        validate_strftime_format(fmt, "%Y-%m-%d %H:%M", 64)
    }
}

impl<'de> Deserialize<'de> for ShowInfoOptions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(default)]
        struct Helper {
            name: bool,
            file_type: bool,
            size: bool,
            modified: bool,
            created: bool,
            accessed: bool,
            perms: bool,
            #[cfg(unix)]
            owner: bool,
            #[cfg(unix)]
            group: bool,
            position: Option<DialogPosition>,
            status_bar: bool,
            format: Option<String>,
            date_format: Option<String>,
        }

        impl Default for Helper {
            fn default() -> Self {
                let def = ShowInfoOptions::default();
                Self {
                    name: def.name,
                    file_type: def.file_type,
                    size: def.size,
                    modified: def.modified,
                    created: def.created,
                    accessed: def.accessed,
                    perms: def.perms,
                    #[cfg(unix)]
                    owner: def.owner,
                    #[cfg(unix)]
                    group: def.group,
                    position: def.position,
                    status_bar: def.status_bar,
                    format: def.format,
                    date_format: Some(def.date_format),
                }
            }
        }

        let h = Helper::deserialize(deserializer)?;

        let mut info = ShowInfoOptions {
            name: h.name,
            file_type: h.file_type,
            size: h.size,
            modified: h.modified,
            created: h.created,
            accessed: h.accessed,
            perms: h.perms,
            #[cfg(unix)]
            owner: h.owner,
            #[cfg(unix)]
            group: h.group,
            position: h.position,
            status_bar: h.status_bar,
            format: h.format,
            date_format: ShowInfoOptions::validate_date_format(h.date_format),
            segments: Vec::new(),
        };

        info.init_status_format();
        Ok(info)
    }
}

// Default show info configuration options
impl Default for ShowInfoOptions {
    fn default() -> Self {
        let mut options = ShowInfoOptions {
            name: true,
            file_type: false,
            size: true,
            modified: true,
            created: true,
            accessed: false,
            perms: true,
            #[cfg(unix)]
            owner: true,
            #[cfg(unix)]
            group: true,
            position: None,
            status_bar: true,
            format: Some("{perms} | {size}".to_string()),
            date_format: "%Y-%m-%d %H:%M".to_string(),
            segments: Vec::new(),
        };

        options.init_status_format();
        options
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StatusTag {
    Perms,
    Size,
    Mtime,
    Btime,
    Atime,
    Type,
    #[cfg(unix)]
    Owner,
    #[cfg(unix)]
    Group,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StatusSegment {
    Literal(String),
    Tag(StatusTag),
}

/// Entry count position options
/// This enum defines the possible positions for displaying the entry count
#[derive(Deserialize, Debug, Default, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum StatusPosition {
    #[default]
    Footer,
    Header,
    None,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub(crate) struct StatusElements {
    entry_count: StatusPosition,
    filter: StatusPosition,
    markers: StatusPosition,
    clipboard: StatusPosition,
    tasks: StatusPosition,
    tabs: StatusPosition,
}

impl Default for StatusElements {
    fn default() -> Self {
        Self {
            entry_count: StatusPosition::Footer,
            filter: StatusPosition::Header,
            markers: StatusPosition::Footer,
            clipboard: StatusPosition::Footer,
            tasks: StatusPosition::Footer,
            tabs: StatusPosition::Header,
        }
    }
}

impl StatusElements {
    crate::getters! {
        entry_count: StatusPosition,
        filter: StatusPosition,
        markers: StatusPosition,
        clipboard: StatusPosition,
        tasks: StatusPosition,
        tabs: StatusPosition,
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
    #[serde(default = "PreviewOptions::default_tab_width")]
    tab_width: usize,
}

/// Public methods for accessing preview configuration options
impl PreviewOptions {
    fn default() -> Self {
        PreviewOptions {
            method: PreviewMethod::Internal,
            style: BatStyle::Plain,
            theme: None,
            wrap: false,
            tab_width: 4,
        }
    }

    fn default_method() -> PreviewMethod {
        PreviewMethod::Internal
    }

    #[inline]
    fn default_wrap() -> bool {
        false
    }

    #[inline]
    fn default_tab_width() -> usize {
        4
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
        args.push(format!("--tabs={}", self.tab_width));
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

        if self.wrap {
            args.push(format!("--terminal-width={}", pane_width));
            args.push("--wrap=character".to_string());
        } else {
            args.push("--wrap=never".to_string());
        }

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

fn parse_status_format(fmt: &str) -> Vec<StatusSegment> {
    let mut segments = Vec::new();
    let mut cursor = 0;

    while let Some(start) = fmt[cursor..].find('{') {
        let start = cursor + start;
        if let Some(end) = fmt[start..].find('}') {
            let end = start + end;

            if start > cursor {
                segments.push(StatusSegment::Literal(fmt[cursor..start].to_string()));
            }

            let tag_str = &fmt[start + 1..end];
            let tag = match tag_str {
                "perms" => Some(StatusTag::Perms),
                "size" => Some(StatusTag::Size),
                "mtime" | "modified" => Some(StatusTag::Mtime),
                "btime" | "created" => Some(StatusTag::Btime),
                "atime" | "accessed" => Some(StatusTag::Atime),
                "type" => Some(StatusTag::Type),
                #[cfg(unix)]
                "owner" => Some(StatusTag::Owner),
                #[cfg(unix)]
                "group" => Some(StatusTag::Group),
                _ => None,
            };

            if let Some(t) = tag {
                segments.push(StatusSegment::Tag(t));
            }
            cursor = end + 1;
        } else {
            break;
        }
    }

    if cursor < fmt.len() {
        segments.push(StatusSegment::Literal(fmt[cursor..].to_string()));
    }
    segments
}

fn validate_strftime_format(fmt: Option<String>, default_fmt: &str, max_len: usize) -> String {
    let Some(user_str) = fmt else {
        return default_fmt.to_string();
    };

    if user_str.is_empty() || user_str.len() > max_len {
        return default_fmt.to_string();
    }

    let mut has_date_specifier = false;

    for item in StrftimeItems::new(&user_str) {
        match item {
            Item::Error => return default_fmt.to_string(),
            Item::Space(_) | Item::Literal(_) => continue,
            _ => has_date_specifier = true,
        }
    }

    if has_date_specifier {
        user_str
    } else {
        default_fmt.to_string()
    }
}

fn deserialize_sort_date_format<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Option<String> = Option::<String>::deserialize(deserializer)?;
    Ok(validate_strftime_format(raw, "%b %e %H:%M", 32))
}
