//! Theme configuration options for runa
//!
//! This module defines the theme configuration options which are read from the runa.toml
//! configuration file.
//!
//! Also holds the internal themes and the logic to apply user overrides on top of them.

use crate::config::presets::*;
use crate::ui::widgets::{DialogPosition, DialogSize};
use crate::utils::parse_color;

use ratatui::style::{Color, Style};
use serde::Deserialize;

use std::sync::LazyLock;

/// Theme configuration options
/// Holds all color and style options for the application.
/// Also holds the internal themes and the logic to apply user overrides on top of them.
/// # Examples
/// ```toml
/// [theme]
/// name = "gruvbox-dark"
/// [theme.entry]
/// fg = "white"
/// bg = "black"
/// [theme.selection]
/// bg = "grey"
/// ```
#[derive(Deserialize, Debug)]
#[serde(default)]
pub(crate) struct Theme {
    name: Option<String>,
    selection: ColorPair,
    underline: ColorPair,
    accent: ColorPair,
    entry: ColorPair,
    directory: ColorPair,
    separator: ColorPair,
    selection_icon: String,
    parent: PaneTheme,
    preview: PaneTheme,
    path: ColorPair,
    status_line: ColorPair,
    #[serde(deserialize_with = "deserialize_color_field")]
    exe_color: Color,
    symlink: SymlinkTheme,
    marker: MarkerTheme,
    widget: WidgetTheme,
    /// info does not honor the .size field from widget.
    /// info gets auto-sized based on attributes enabled.
    info: WidgetTheme,
    tab: TabTheme,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            name: None,
            accent: ColorPair {
                fg: Color::Indexed(238),
                ..ColorPair::default()
            },
            selection: ColorPair {
                bg: Color::Indexed(236),
                ..ColorPair::default()
            },
            underline: ColorPair::default(),
            entry: ColorPair::default(),
            directory: ColorPair {
                fg: Color::Blue,
                ..ColorPair::default()
            },
            separator: ColorPair {
                fg: Color::Indexed(238),
                ..ColorPair::default()
            },
            selection_icon: "".into(),
            parent: PaneTheme::default(),
            preview: PaneTheme::default(),
            path: ColorPair {
                fg: Color::Magenta,
                ..ColorPair::default()
            },
            status_line: ColorPair::default(),
            exe_color: Color::LightGreen,
            symlink: SymlinkTheme::default(),
            marker: MarkerTheme::default(),
            widget: WidgetTheme::default(),
            info: WidgetTheme {
                title: ColorPair {
                    fg: Color::Magenta,
                    ..ColorPair::default()
                },
                ..WidgetTheme::default()
            },
            tab: TabTheme::default(),
        }
    }
}

/// Macro to override a field in the target theme if it differs from the default theme.
/// This is used to apply user-defined overrides on top of a preset theme.
macro_rules! override_themes {
    ($target:ident, $user:ident, $default:ident, [$($field:ident),* $(,)?]) => {
        $(
            if $user.$field != $default.$field {
                $target.$field = $user.$field.clone();
            }
        )*
    };
}

macro_rules! define_styles {
    ($($fn_name:ident => $field:ident),* $(,)?) => {
        $(
            pub(crate) fn $fn_name(&self) -> Style {
                self.$field.style_or(&Theme::internal_defaults().$field)
            }
        )*
    };
}

/// Theme implementation
/// Provides methods to access theme properties and apply user overrides.
impl Theme {
    /// Get internal default theme reference
    /// Used for fallback when a color is set to Reset
    /// This avoids recreating the default theme multiple times
    /// by using a static Lazy instance.
    pub(crate) fn internal_defaults() -> &'static Self {
        static DEFAULT: LazyLock<Theme> = LazyLock::new(Theme::default);
        &DEFAULT
    }

    // Getters for various theme properties with fallbacks to internal defaults
    // _style methods for getting Style instances with fallbacks to internal defaults
    define_styles! {
        accent_style => accent,
        selection_style => selection,
        underline_style => underline,
        entry_style => entry,
        directory_style => directory,
        separator_style => separator,
        path_style => path,
        status_line_style => status_line,
    }

    /// Symlink theme getter to use SymlinkTheme fields directly
    pub(crate) fn symlink_theme(&self) -> SymlinkTheme {
        let defaults = Theme::internal_defaults().symlink;
        SymlinkTheme {
            directory: self.symlink.directory.or(defaults.directory),
            file: self.symlink.file.or(defaults.file),
            target: self.symlink.target.or(defaults.target),
        }
    }

    // Pane-specific style getters

    pub(crate) fn parent_selection_style(&self) -> Style {
        if self.parent.selection_mode == SelectionMode::Off {
            return Style::default();
        }
        self.parent.selection_style(&self.selection)
    }

    pub(crate) fn preview_selection_style(&self) -> Style {
        if self.preview.selection_mode == SelectionMode::Off {
            return Style::default();
        }
        self.preview.selection_style(&self.selection)
    }

    pub(crate) fn preview_item_style(&self) -> Style {
        self.preview.entry_style(&self.entry)
    }
    pub(crate) fn parent_item_style(&self) -> Style {
        self.parent.entry_style(&self.entry)
    }

    // Accessor methods for various theme properties

    pub(crate) fn exe_color(&self) -> Color {
        self.exe_color
    }

    #[inline]
    pub(crate) fn selection_icon(&self) -> &str {
        &self.selection_icon
    }

    #[inline]
    pub(crate) fn preview(&self) -> &PaneTheme {
        &self.preview
    }

    #[inline]
    pub(crate) fn marker(&self) -> &MarkerTheme {
        &self.marker
    }

    #[inline]
    pub(crate) fn widget(&self) -> &WidgetTheme {
        &self.widget
    }

    #[inline]
    pub(crate) fn info(&self) -> &WidgetTheme {
        &self.info
    }

    #[inline]
    pub(crate) fn tab(&self) -> &TabTheme {
        &self.tab
    }

    /// Apply user overrides on top of a preset theme if a known preset name is provided.
    /// If no preset name is provided or the name is unknown, returns the theme as is.
    pub(crate) fn with_overrides(self) -> Self {
        let preset = match self.name.as_deref() {
            Some("gruvbox-dark-hard") => Some(gruvbox_dark_hard()),
            Some("gruvbox-dark") => Some(gruvbox_dark()),
            Some("gruvbox-light") => Some(gruvbox_light()),

            Some("catppuccin-mocha") => Some(catppuccin_mocha()),
            Some("catppuccin-frappe") => Some(catppuccin_frappe()),
            Some("catppuccin-macchiato") => Some(catppuccin_mocha()),
            Some("catppuccin-latte") => Some(catppuccin_latte()),

            Some("nord") => Some(nord()),

            Some("two-dark") => Some(two_dark()),
            Some("one-dark") => Some(one_dark()),

            Some("solarized-dark") => Some(solarized_dark()),
            Some("solarized-light") => Some(solarized_light()),

            Some("dracula") => Some(dracula()),

            Some("monokai") => Some(monokai()),

            Some("nightfox") => Some(nightfox()),
            Some("carbonfox") => Some(carbonfox()),

            Some("tokyonight") => Some(tokyonight_night()),
            Some("tokyonight-storm") => Some(tokyonight_storm()),
            Some("tokyonight-day") => Some(tokyonight_day()),

            Some("everforest") => Some(everforest()),
            Some("rose-pine") | Some("rose_pine") => Some(rose_pine()),

            _ => None,
        };

        if let Some(mut base) = preset {
            base.apply_user_overrides(self);
            base
        } else {
            self
        }
    }

    /// Map internal theme name to bat theme name for syntax highlighting.
    /// If no name is set, defaults to "TwoDark".
    /// Returns a static string slice representing the bat theme name.
    pub(crate) fn bat_theme_name(&self) -> &'static str {
        self.name
            .as_deref()
            .map(Theme::map_to_bat_theme)
            .unwrap_or("TwoDark")
    }

    /// Helper function to map internal theme names to bat theme names.
    /// Used by bat for syntax highlighting.
    /// # Returns:
    /// A static string slice representing the corresponding bat theme name.
    fn map_to_bat_theme(internal_theme: &str) -> &'static str {
        match internal_theme {
            "default" => "TwoDark",
            "two-dark" => "TwoDark",
            "one-dark" => "OneHalfDark",
            "gruvbox-dark" | "gruvbox-dark-hard" | "gruvbox" => "gruvbox-dark",
            "gruvbox-light" => "gruvbox-light",
            "tokyonight-night" | "tokyonight" | "tokyonight-storm" => "TwoDark",
            "catppuccin-latte" => "Catppuccin Latte",
            "catppuccin-frappe" => "Catppuccin Frappe",
            "catppuccin-macchiato" => "Catppuccin Macchiato",
            "catppuccin-mocha" | "catppuccin" => "Catppuccin Mocha",
            "nightfox" | "carbonfox" | "rose-pine" | "everforest" => "TwoDark",
            "monokai" => "Monokai Extended (default)",
            "nord" => "Nord",
            "solarized-dark" => "Solarized (dark)",
            "solarized-light" => "Solarized (light)",
            "dracula" => "Dracula",
            _ => "TwoDark",
        }
    }

    /// Apply user overrides on top of the current theme.
    /// Compares each field with the default theme and overrides if changed
    /// This allows to only specify the fields they want to change
    fn apply_user_overrides(&mut self, user: Theme) {
        let defaults = Theme::default();

        #[rustfmt::skip]
        override_themes!(self, user, defaults, [
            accent,
            selection,
            underline,
            entry,
            directory,
            separator,
            parent,
            preview,
            path,
            status_line,
            symlink,
            selection_icon,
            marker,
            widget,
            info,
        ]);

        if user.name.is_some() {
            self.name = user.name.clone();
        }
    }
}

/// ColorPair struct to hold foreground and background colors.
/// Used throughout the theme configuration.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
pub(crate) struct ColorPair {
    #[serde(default, deserialize_with = "deserialize_color_field")]
    fg: Color,
    #[serde(default, deserialize_with = "deserialize_color_field")]
    bg: Color,
}

/// Default implementation for ColorPair
/// Sets both foreground and background to Color::Reset
impl Default for ColorPair {
    fn default() -> Self {
        Self {
            fg: Color::Reset,
            bg: Color::Reset,
        }
    }
}

/// ColorPair implementation
/// Provides methods to convert to Style and get effective styles.
impl ColorPair {
    /// Resolves the ColorPair by replacing Reset colors with those from another ColorPair.
    pub(crate) fn resolve(&self, other: &ColorPair) -> Self {
        Self {
            fg: if self.fg == Color::Reset {
                other.fg
            } else {
                self.fg
            },
            bg: if self.bg == Color::Reset {
                other.bg
            } else {
                self.bg
            },
        }
    }

    /// Converts the ColorPair to a Style, falling back to the provided fallback ColorPair for Reset colors.
    pub(crate) fn style_or(&self, fallback: &ColorPair) -> Style {
        let resovled = self.resolve(fallback);
        Style::default().fg(resovled.fg).bg(resovled.bg)
    }
}

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
    pub(crate) fn selection_style(&self, fallback: &ColorPair) -> Style {
        let default = &Theme::internal_defaults().selection;
        match self.selection {
            Some(pane_sel) => pane_sel.style_or(&fallback.resolve(default)),
            None => fallback.style_or(default),
        }
    }

    /// Returns the entry style, falling back to the provided fallback ColorPair.
    /// If entry color is Reset, uses the fallback.
    pub(crate) fn entry_style(&self, fallback: &ColorPair) -> Style {
        self.color.style_or(fallback)
    }

    /// Returns the pane color style, falling back to the provided fallback ColorPair.
    pub(crate) fn style_or(&self, fallback: &ColorPair) -> Style {
        self.color.style_or(fallback)
    }

    /// Returns the pane color style, falling back to the internal default theme's entry style.
    /// This method uses the internal default theme as the fallback.
    pub(crate) fn effective_style_or_theme(&self) -> Style {
        self.style_or(&Theme::internal_defaults().entry)
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
    /// Returns the marker icon.
    #[inline]
    pub(crate) fn icon(&self) -> &str {
        &self.icon
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
}

impl Default for MarkerTheme {
    fn default() -> Self {
        MarkerTheme {
            icon: "*".to_string(),
            color: ColorPair {
                fg: Color::Yellow,
                bg: Color::Reset,
            },
            clipboard: Some(ColorPair {
                fg: Color::Green,
                bg: Color::Reset,
            }),
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
    #[inline]
    pub(crate) fn position(&self) -> &Option<DialogPosition> {
        &self.position
    }

    #[inline]
    pub(crate) fn size(&self) -> &Option<DialogSize> {
        &self.size
    }

    #[inline]
    pub(crate) fn confirm_size(&self) -> &Option<DialogSize> {
        &self.confirm_size
    }

    /// Returns the confirm dialog size, falling back to the general size, and then to the provided fallback.
    pub(crate) fn confirm_size_or(&self, fallback: DialogSize) -> DialogSize {
        self.confirm_size()
            .as_ref()
            .or_else(|| self.size().as_ref())
            .copied()
            .unwrap_or(fallback)
    }

    pub(crate) fn move_size(&self) -> &Option<DialogSize> {
        &self.move_size
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
        self.border.style_or(&ColorPair {
            fg: fallback.fg.unwrap_or(Color::Reset),
            bg: fallback.bg.unwrap_or(Color::Reset),
        })
    }

    /// Returns the foreground style, falling back to the provided style if Reset.
    pub(crate) fn fg_or(&self, fallback: Style) -> Style {
        self.color.style_or(&ColorPair {
            fg: fallback.fg.unwrap_or(Color::Reset),
            bg: fallback.bg.unwrap_or(Color::Reset),
        })
    }

    /// Returns the background style, falling back to the provided style if Reset.
    pub(crate) fn bg_or(&self, fallback: Style) -> Style {
        self.color.style_or(&ColorPair {
            fg: fallback.fg.unwrap_or(Color::Reset),
            bg: fallback.bg.unwrap_or(Color::Reset),
        })
    }

    /// Returns the foreground style, falling back to the internal default theme if Reset.
    pub(crate) fn fg_or_theme(&self) -> Style {
        self.fg_or(Style::default().fg(Theme::internal_defaults().info.color.fg))
    }

    /// Returns the background style, falling back to the internal default theme if Reset.
    pub(crate) fn bg_or_theme(&self) -> Style {
        self.bg_or(Style::default().bg(Theme::internal_defaults().info.color.bg))
    }

    /// Returns the title style, falling back to the internal default theme if Reset.
    pub(crate) fn title_style_or_theme(&self) -> Style {
        self.title.style_or(&Theme::internal_defaults().info.title)
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
            .or(Theme::internal_defaults().widget.go_to_help.size)
            .unwrap_or(DialogSize::Custom(38, 3))
    }

    pub(crate) fn go_to_help_position(&self) -> DialogPosition {
        self.go_to_help
            .position
            .or(Theme::internal_defaults().widget.go_to_help.position)
            .unwrap_or(DialogPosition::Bottom)
    }

    pub(crate) fn value_style_or_theme(&self) -> Style {
        self.value
            .style_or(&Theme::internal_defaults().widget.value)
    }

    pub(crate) fn label_style_or_theme(&self) -> Style {
        self.label
            .style_or(&Theme::internal_defaults().widget.label)
    }
}

/// Default implementation for WidgetTheme
impl Default for WidgetTheme {
    fn default() -> Self {
        WidgetTheme {
            color: ColorPair::default(),
            border: ColorPair::default(),
            title: ColorPair::default(),
            label: ColorPair {
                fg: Color::Blue,
                ..ColorPair::default()
            },
            value: ColorPair {
                fg: Color::Cyan,
                ..ColorPair::default()
            },
            position: Some(DialogPosition::Center),
            size: Some(DialogSize::Small),
            confirm_size: Some(DialogSize::Large),
            move_size: Some(DialogSize::Custom(70, 14)),
            find_visible_results: Some(5),
            find_width: Some(40),
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
    pub(crate) fn directory(&self) -> Color {
        self.directory
    }

    pub(crate) fn file(&self) -> Color {
        self.file
    }

    pub(crate) fn target(&self) -> Color {
        self.target
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub(crate) struct TabTheme {
    marker: String,
    separator: String,
    active: ColorPair,
    inactive: ColorPair,
}

impl Default for TabTheme {
    fn default() -> Self {
        TabTheme {
            marker: "*".to_string(),
            separator: ":".to_string(),
            active: ColorPair {
                fg: Color::Yellow,
                ..Default::default()
            },
            inactive: ColorPair {
                fg: Color::Gray,
                ..Default::default()
            },
        }
    }
}

impl TabTheme {
    pub(crate) fn marker(&self) -> &str {
        &self.marker
    }

    pub(crate) fn separator(&self) -> &str {
        &self.separator
    }

    pub fn active_style_or_theme(&self) -> Style {
        self.active.style_or(&Theme::internal_defaults().tab.active)
    }

    /// Returns a Style for the inactive tab, using theme fallback if unset.
    pub fn inactive_style_or_theme(&self) -> Style {
        self.inactive
            .style_or(&Theme::internal_defaults().tab.inactive)
    }
}

/// Trait to provide a fallback color if the original color is Reset.
/// Is used when a field is Color::Reset to fallback to another color.
/// Useful for when a field is ratatui::style::Color instead of ColorPair.
trait ColorFallback {
    fn or(self, fallback: Color) -> Color;
}

/// Implementation of ColorFallback for Color.
/// If the color is Reset, returns the fallback color, otherwise returns self.
impl ColorFallback for Color {
    fn or(self, fallback: Color) -> Color {
        if let Color::Reset = self {
            fallback
        } else {
            self
        }
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

// Helper function to deserialize Theme colors
fn deserialize_color_field<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(parse_color(&s))
}

/// Helper function to convert RGB tuples to [Color] instances.
fn rgb(c: (u8, u8, u8)) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

/// Palette struct to apply internal themes to the central [make_theme] function.
pub(crate) struct Palette {
    pub(crate) base: (u8, u8, u8),
    pub(crate) surface: (u8, u8, u8),
    pub(crate) overlay: (u8, u8, u8),
    pub(crate) primary: (u8, u8, u8),
    pub(crate) secondary: (u8, u8, u8),
    pub(crate) directory: (u8, u8, u8),
}

/// Centralized function to create a Theme from a Palette.
/// Used by all internal themes to avoid code duplication.
pub(crate) fn make_theme(name: &str, palette: Palette, icon: &str) -> Theme {
    let primary = rgb(palette.primary);
    let secondary = rgb(palette.secondary);
    let muted = rgb(palette.overlay);
    let surface = rgb(palette.surface);
    let base_bg = rgb(palette.base);
    let dir_color = rgb(palette.directory);

    Theme {
        name: Some(name.to_string()),
        accent: ColorPair {
            fg: surface,
            ..ColorPair::default()
        },
        selection: ColorPair {
            bg: surface,
            ..ColorPair::default()
        },
        directory: ColorPair {
            fg: dir_color,
            ..ColorPair::default()
        },
        separator: ColorPair {
            fg: surface,
            ..ColorPair::default()
        },
        path: ColorPair {
            fg: muted,
            ..ColorPair::default()
        },
        status_line: ColorPair {
            fg: Color::Reset,
            bg: base_bg,
        },
        symlink: SymlinkTheme {
            directory: secondary,
            file: secondary,
            target: Color::Magenta,
        },
        marker: MarkerTheme {
            icon: icon.to_string(),
            color: ColorPair {
                fg: primary,
                ..ColorPair::default()
            },
            clipboard: Some(ColorPair {
                fg: secondary,
                ..ColorPair::default()
            }),
        },

        widget: WidgetTheme {
            title: ColorPair {
                fg: primary,
                ..ColorPair::default()
            },
            label: ColorPair {
                fg: secondary,
                ..ColorPair::default()
            },
            value: ColorPair {
                fg: Color::Reset,
                ..ColorPair::default()
            },
            border: ColorPair {
                fg: surface,
                ..ColorPair::default()
            },
            ..WidgetTheme::default()
        },

        info: WidgetTheme {
            title: ColorPair {
                fg: secondary,
                ..ColorPair::default()
            },
            label: ColorPair {
                fg: dir_color,
                ..ColorPair::default()
            },
            value: ColorPair {
                fg: muted,
                ..ColorPair::default()
            },
            border: ColorPair {
                fg: surface,
                ..ColorPair::default()
            },
            ..WidgetTheme::default()
        },
        ..Theme::default()
    }
}
