//! theme configuration components

use ratatui::style::{Color, Style};
use serde::Deserialize;

use crate::config::{
    Theme,
    theme::{ColorFallback, ColorPair, deserialize_color_field},
};
use crate::ui::widgets::{DialogPosition, DialogSize};

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
enum SelectionMode {
    #[default]
    On,
    Off,
}

/// PaneTheme struct to hold color and selection styles for panes.
/// Used for parent and preview panes.
#[derive(Deserialize, Debug, PartialEq, Clone, Copy, Default)]
#[serde(default)]
pub(crate) struct PaneTheme {
    #[serde(flatten)]
    color: ColorPair,
    selection: Option<ColorPair>,
    selection_mode: SelectionMode,
}

/// Similar to ColorPair implementation
/// Provides methods to convert to Style and get effective styles.
impl PaneTheme {
    /// Returns the selection style, falling back to the provided fallback if not set.
    /// If selection is None, falls back to the provided fallback ColorPair.
    /// If selection is Some, uses its style_or method with the fallback.
    pub(crate) fn selection_style(&self) -> Style {
        if self.selection_mode == SelectionMode::Off {
            return Style::default();
        }

        let default = &Theme::builtin().selection;
        match self.selection {
            Some(pane_sel) => pane_sel.style_or(default),
            None => default.style(),
        }
    }

    pub(crate) fn entry_style_or_theme(&self) -> Style {
        self.color.style_or(&Theme::builtin().entry)
    }
}

/// MarkerTheme struct to hold marker icon and colors.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub(crate) struct MarkerTheme {
    icon: String,
    #[serde(flatten)]
    color: ColorPair,
    /// Optional clipboard color pair
    /// sets the color of the copy/paste marker
    clipboard: Option<ColorPair>,
}

impl MarkerTheme {
    crate::getters! {
        icon: &str,
    }

    pub(super) fn new(icon: String, color: ColorPair) -> Self {
        Self {
            icon,
            color,
            clipboard: None,
        }
    }

    /// Returns the marker style, falling back to the internal default theme if colors are Reset.
    pub(crate) fn style_or_theme(&self) -> Style {
        self.color.style_or(&MarkerTheme::default().color)
    }

    /// Returns the clipboard marker style, falling back to the marker style if clipboard is None.
    pub(crate) fn clipboard_style_or_theme(&self) -> Style {
        match &self.clipboard {
            Some(c) => c.style_or(&MarkerTheme::default().clipboard.unwrap()),
            None => self.style_or_theme(),
        }
    }

    pub(super) fn with_clipboard(mut self, pair: ColorPair) -> Self {
        self.clipboard = Some(pair);
        self
    }
}

impl Default for MarkerTheme {
    fn default() -> Self {
        MarkerTheme {
            icon: "*".to_string(),
            color: ColorPair::new(Color::Yellow, Color::Reset),
            clipboard: Some(ColorPair::new(Color::Green, Color::Reset)),
        }
    }
}

/// WidgetTheme struct to hold colors and styles for widgets/dialogs.
/// Used by various dialog widgets and overlay widgets.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub(crate) struct WidgetTheme {
    color: ColorPair,
    border: ColorPair,
    title: ColorPair,
    label: ColorPair,
    value: ColorPair,
    position: Option<DialogPosition>,
    size: Option<DialogSize>,
    confirm_size: Option<DialogSize>,
    move_size: Option<DialogSize>,
    find_visible_results: Option<usize>,
    find_width: Option<u16>,
    go_to_help: GoToHelpTheme,
}

impl WidgetTheme {
    crate::getters! {
        position: &Option<DialogPosition>,
        size: &Option<DialogSize>,
        confirm_size: &Option<DialogSize>,
        move_size: &Option<DialogSize>,
    }

    /// Returns the confirm dialog size, falling back to the general size, and then to the provided fallback.
    pub(crate) fn confirm_size_or(&self, fallback: DialogSize) -> DialogSize {
        self.confirm_size()
            .as_ref()
            .or_else(|| self.size().as_ref())
            .copied()
            .unwrap_or(fallback)
    }

    pub(crate) fn move_size_or(&self, fallback: DialogSize) -> DialogSize {
        self.move_size()
            .as_ref()
            .or_else(|| self.size().as_ref())
            .copied()
            .unwrap_or(fallback)
    }

    /// Returns the border style, falling back to the provided style for Reset colors.
    pub(crate) fn border_style_or(&self, fallback: Style) -> Style {
        self.border.style_or(&fallback.into())
    }

    /// Returns the background style, falling back to the provided style if Reset.
    pub(crate) fn color_or(&self, fallback: Style) -> Style {
        self.color.style_or(&fallback.into())
    }

    /// Returns the foreground style, falling back to the internal default theme if Reset.
    pub(crate) fn fg_or_theme(&self) -> Style {
        self.color_or(Theme::builtin().widget.color.fg_style())
    }

    /// Returns the background style, falling back to the internal default theme if Reset.
    pub(crate) fn bg_or_theme(&self) -> Style {
        self.color_or(Theme::builtin().widget.color.bg_style())
    }

    /// Returns the title style, falling back to the internal default theme if Reset.
    pub(crate) fn title_style_or_theme(&self) -> Style {
        self.title.style_or(&Theme::builtin().widget.title)
    }

    /// Returns the number of visible results in the find dialog, falling back to the provided fallback.
    pub(crate) fn find_visible_or(&self, fallback: usize) -> usize {
        self.find_visible_results.unwrap_or(fallback)
    }

    /// Returns the width of the find dialog, falling back to the provided fallback.
    pub(crate) fn find_width_or(&self, fallback: u16) -> u16 {
        self.find_width.unwrap_or(fallback)
    }

    pub(crate) fn go_to_help_size(&self) -> DialogSize {
        self.go_to_help
            .size
            .or(Theme::builtin().widget.go_to_help.size)
            .unwrap_or(DialogSize::Custom(38, 3))
    }

    pub(crate) fn go_to_help_position(&self) -> DialogPosition {
        self.go_to_help
            .position
            .or(Theme::builtin().widget.go_to_help.position)
            .unwrap_or(DialogPosition::Bottom)
    }

    pub(crate) fn value_style_or_theme(&self) -> Style {
        self.value.style_or(&Theme::builtin().widget.value)
    }

    pub(crate) fn label_style_or_theme(&self) -> Style {
        self.label.style_or(&Theme::builtin().widget.label)
    }

    pub(crate) fn from_palette(primary: Color, secondary: Color, surface: Color) -> Self {
        Self {
            title: ColorPair::new(primary, Color::Reset),
            label: ColorPair::new(secondary, Color::Reset),
            border: ColorPair::new(surface, Color::Reset),
            value: ColorPair::new(Color::Reset, Color::Reset),
            ..Self::default()
        }
    }
}

/// Default implementation for WidgetTheme
impl Default for WidgetTheme {
    fn default() -> Self {
        WidgetTheme {
            color: ColorPair::default(),
            border: ColorPair::default(),
            title: ColorPair::new(Color::Magenta, Color::Reset),
            label: ColorPair::new(Color::Blue, Color::Reset),
            value: ColorPair::new(Color::Cyan, Color::Reset),
            position: Some(DialogPosition::Center),
            size: Some(DialogSize::Small),
            confirm_size: Some(DialogSize::Large),
            move_size: Some(DialogSize::Custom(70, 14)),
            find_visible_results: Some(8),
            find_width: Some(60),
            go_to_help: GoToHelpTheme::default(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(default)]
pub(crate) struct SymlinkTheme {
    #[serde(deserialize_with = "deserialize_color_field")]
    directory: Color,
    #[serde(deserialize_with = "deserialize_color_field")]
    file: Color,
    #[serde(deserialize_with = "deserialize_color_field")]
    target: Color,
}

impl Default for SymlinkTheme {
    fn default() -> Self {
        Self {
            directory: Color::Cyan,
            file: Color::Indexed(150),
            target: Color::Magenta,
        }
    }
}

impl SymlinkTheme {
    crate::getters! {
        directory: Color,
        file: Color,
        target: Color,
    }

    pub(super) fn with_fallback(&self, fallback: Self) -> Self {
        SymlinkTheme {
            directory: self.directory.or(fallback.directory),
            file: self.file.or(fallback.file),
            target: self.target.or(fallback.target),
        }
    }

    pub(crate) fn from_palette(dir: Color, file: Color) -> Self {
        Self {
            directory: dir,
            file,
            target: Color::Magenta,
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub(crate) struct TabTheme {
    marker: String,
    separator: String,
    active: ColorPair,
    inactive: ColorPair,
    line_format: Option<String>,
}

impl Default for TabTheme {
    fn default() -> Self {
        TabTheme {
            marker: "".to_string(),
            separator: "".to_string(),
            active: ColorPair::new(Color::Yellow, Color::Reset),
            inactive: ColorPair::new(Color::Gray, Color::Reset),
            line_format: None,
        }
    }
}

impl TabTheme {
    crate::getters! {
        marker: &str,
        separator: &str,
    }

    pub(crate) fn active_style_or_theme(&self) -> Style {
        self.active.style_or(&Theme::builtin().tab.active)
    }

    /// Returns a Style for the inactive tab, using theme fallback if unset.
    pub(crate) fn inactive_style_or_theme(&self) -> Style {
        self.inactive.style_or(&Theme::builtin().tab.inactive)
    }

    pub(crate) fn format_tab(&self, idx: usize, is_current: bool, name: Option<&str>) -> String {
        let marker = if is_current { self.marker() } else { "" };
        let separator = self.separator();
        let format = self.line_format.as_deref().unwrap_or("[{idx}{marker}]");

        format
            .replace("{idx}", &(idx + 1).to_string())
            .replace("{marker}", marker)
            .replace("{separator}", separator)
            .replace("{name}", name.unwrap_or(""))
    }

    pub(crate) fn uses_name(&self) -> bool {
        if let Some(fmt) = &self.line_format {
            fmt.contains("{name}")
        } else {
            false
        }
    }

    pub(crate) fn from_palette(active: Color, inactive: Color) -> Self {
        Self {
            active: ColorPair::new(active, Color::Reset),
            inactive: ColorPair::new(inactive, Color::Reset),
            ..Self::default()
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(default)]
pub(crate) struct InfoStatusTheme {
    perms: ColorPair,
    size: ColorPair,
    date: ColorPair,
    modified: ColorPair,
    created: ColorPair,
    accessed: ColorPair,
    file_type: ColorPair,
    #[cfg(unix)]
    owner: ColorPair,
    #[cfg(unix)]
    group: ColorPair,
}

impl Default for InfoStatusTheme {
    fn default() -> Self {
        Self {
            perms: ColorPair::new(Color::LightGreen, Color::Reset),
            size: ColorPair::default(),
            date: ColorPair::default(),
            modified: ColorPair::default(),
            created: ColorPair::default(),
            accessed: ColorPair::default(),
            file_type: ColorPair::default(),
            #[cfg(unix)]
            owner: ColorPair::default(),
            #[cfg(unix)]
            group: ColorPair::default(),
        }
    }
}

impl InfoStatusTheme {
    fn resolve_date(&self, specific: &ColorPair) -> Style {
        let defaults = &Theme::builtin().info;

        specific.resolve(&self.date).style_or(&defaults.date)
    }

    pub(crate) fn perms_style(&self) -> Style {
        self.perms.style_or(&Theme::builtin().info.perms)
    }

    pub(crate) fn size_style(&self) -> Style {
        self.size.style_or(&Theme::builtin().info.size)
    }

    pub(crate) fn modified_style(&self) -> Style {
        self.resolve_date(&self.modified)
    }

    pub(crate) fn created_style(&self) -> Style {
        self.resolve_date(&self.created)
    }

    pub(crate) fn accessed_style(&self) -> Style {
        self.resolve_date(&self.accessed)
    }

    pub(crate) fn file_type_style(&self) -> Style {
        self.file_type.style_or(&Theme::builtin().info.file_type)
    }

    #[cfg(unix)]
    pub(crate) fn owner_style(&self) -> Style {
        self.owner.style_or(&Theme::builtin().info.owner)
    }

    #[cfg(unix)]
    pub(crate) fn group_style(&self) -> Style {
        self.group.style_or(&Theme::builtin().info.group)
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub(super) struct GoToHelpTheme {
    size: Option<DialogSize>,
    position: Option<DialogPosition>,
}

impl Default for GoToHelpTheme {
    fn default() -> Self {
        GoToHelpTheme {
            size: Some(DialogSize::Custom(58, 3)),
            position: Some(DialogPosition::Bottom),
        }
    }
}
