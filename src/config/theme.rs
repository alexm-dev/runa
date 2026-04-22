//! Theme configuration options for runa
//!
//! This module defines the theme configuration options which are read from the runa.toml
//! configuration file.
//!
//! Also holds the internal themes and the logic to apply user overrides on top of them.

use std::collections::HashMap;
use std::sync::LazyLock;

use ratatui::style::{Color, Style};
use serde::{Deserialize, Deserializer};

use crate::config::presets;
use crate::ui::widgets::{DialogPosition, DialogSize};
use crate::utils::text;

/// Theme configuration options
/// Holds all color and style options for the application.
/// Also holds the internal themes and the logic to apply user overrides on top of them.
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
    #[serde(deserialize_with = "deserialize_color_map")]
    icon_color: HashMap<String, Color>,
    filename: HashMap<String, ColorPair>,
    ext: HashMap<String, ColorPair>,
    symlink: SymlinkTheme,
    marker: MarkerTheme,
    widget: WidgetTheme,
    tab: TabTheme,
    info: InfoStatusTheme,

    #[serde(skip)]
    filename_cache: HashMap<String, Style>,
    #[serde(skip)]
    extension_cache: HashMap<String, Style>,
    #[serde(skip)]
    icon_color_cache: HashMap<String, Color>,
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
            filename: HashMap::new(),
            ext: HashMap::new(),
            icon_color: HashMap::new(),
            exe_color: Color::LightGreen,
            symlink: SymlinkTheme::default(),
            marker: MarkerTheme::default(),
            widget: WidgetTheme::default(),
            tab: TabTheme::default(),
            info: InfoStatusTheme::default(),
            filename_cache: HashMap::new(),
            extension_cache: HashMap::new(),
            icon_color_cache: HashMap::new(),
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

    crate::getters! {
        exe_color: Color,
        icon_color_cache: &HashMap<String, Color>,
        selection_icon: &str,
        preview: &PaneTheme,
        marker: &MarkerTheme,
        widget: &WidgetTheme,
        info: &InfoStatusTheme,
        tab: &TabTheme,
    }

    // Getters for various theme properties with fallbacks to internal defaults
    // _style methods for getting Style instances with fallbacks to internal defaults
    define_styles! {
        accent_style => accent,
        selection_style => selection,
        underline_style => underline,
        entry_style => entry,
        directory_style => directory,
        path_style => path,
        status_line_style => status_line,
    }

    pub(crate) fn separator_style(&self) -> Style {
        let default = Theme::internal_defaults();
        if self.separator != default.separator {
            self.separator.style_or(&default.separator)
        } else {
            self.accent_style()
        }
    }

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

    pub(crate) fn entry_color_override(
        &self,
        name: &str,
        is_dir: bool,
        ext: Option<&str>,
    ) -> Option<Style> {
        if let Some(s) = self.filename_cache.get(name) {
            return Some(*s);
        }

        if !is_dir
            && let Some(ext) = ext
            && let Some(s) = self.extension_cache.get(ext)
        {
            return Some(*s);
        }

        Self::get_default_style(name, ext, is_dir)
    }

    fn get_default_style(name: &str, ext: Option<&str>, is_dir: bool) -> Option<Style> {
        match name {
            "Dockerfile" => return Some(Style::default().fg(Color::Cyan)),
            "Cargo.toml" | "LICENSE" | "README.md" => {
                return Some(Style::default().fg(Color::Yellow));
            }
            _ => {}
        }

        if !is_dir && let Some(e) = ext {
            let color = match e {
                "zip" | "tar" | "gz" | "7z" | "rar" => Color::Red,
                "jpg" | "jpeg" | "png" | "gif" | "svg" | "webm" => Color::Magenta,
                "tmp" => Color::Rgb(108, 121, 135),
                _ => return None,
            };
            return Some(Style::default().fg(color));
        }

        None
    }

    /// Apply user overrides on top of a preset theme if a known preset name is provided.
    /// If no preset name is provided or the name is unknown, returns the theme as is.
    #[inline(never)]
    pub(super) fn with_overrides(mut self) -> Self {
        let preset_name = self.name.clone();
        let defaults = Theme::internal_defaults();

        if self.accent != defaults.accent && self.separator == defaults.separator {
            self.separator = self.accent;
        }

        if let Some(name) = preset_name.as_deref()
            && let Some(mut base) = Self::get_preset_by_name(name)
        {
            base.apply_user_overrides(self);
            base.build_style_maps();
            return base;
        }

        self.build_style_maps();
        self
    }

    #[inline(never)]
    fn get_preset_by_name(name: &str) -> Option<Theme> {
        let (palette, icon) = match name {
            "gruvbox-dark-hard" => (presets::GRUV_DARK_HARD, "*"),
            "gruvbox-dark" => (presets::GRUV_DARK, "*"),
            "gruvbox-light" => (presets::GRUV_LIGHT, "*"),

            "catppuccin-mocha" | "catppuccin-macchiato" => (presets::MOCHA, "┃"),
            "catppuccin-frappe" => (presets::FRAPPE, "┃"),
            "catppuccin-latte" => (presets::LATTE, "┃"),

            "nord" => (presets::NORD, "*"),
            "two-dark" => (presets::TWO_DARK, "*"),
            "one-dark" => (presets::ONE_DARK, "*"),

            "solarized-dark" => (presets::SOLARIZED_DARK, "*"),
            "solarized-light" => (presets::SOLARIZED_LIGHT, "*"),

            "dracula" => (presets::DRACULA, "┃"),
            "monokai" => (presets::MONOKAI, "┃"),
            "nightfox" => (presets::NIGHTFOX, "┃"),
            "carbonfox" => (presets::CARBON, "┃"),

            "tokyonight-storm" => (presets::TOKYO_STORM, "┃"),
            "tokyonight" | "tokyonight-night" => (presets::TOKYO_NIGHT, "┃"),
            "tokyonight-day" => (presets::TOKYO_DAY, "┃"),

            "everforest" => (presets::FOREST, "*"),
            "rose-pine" | "rose_pine" => (presets::ROSE_PINE, "*"),

            _ => return None,
        };

        // One single call-site for make_theme prevents monomorphization bloat
        Some(make_theme(name, palette, icon))
    }

    /// Map internal theme name to bat theme name for syntax highlighting.
    /// If no name is set, defaults to "TwoDark".
    pub(super) fn bat_theme_name(&self) -> &'static str {
        self.name
            .as_deref()
            .map(Theme::map_to_bat_theme)
            .unwrap_or("TwoDark")
    }

    /// Helper function to map internal theme names to bat theme names.
    /// Used by bat for syntax highlighting.
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
        let defaults = Theme::internal_defaults();

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
            ext,
            filename,
            icon_color,
            status_line,
            symlink,
            selection_icon,
            marker,
            widget,
            tab,
            info,
        ]);

        if user.name.is_some() {
            self.name = user.name.clone();
        }
    }

    #[inline(never)]
    fn build_style_maps(&mut self) {
        let fallback = ColorPair::default();

        self.filename_cache = self
            .filename
            .iter()
            .map(|(k, v)| (k.clone(), v.style_or(&fallback)))
            .collect();

        self.extension_cache = self
            .ext
            .iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), v.style_or(&fallback)))
            .collect();

        self.icon_color_cache = self
            .icon_color
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
    }
}

/// ColorPair struct to hold foreground and background colors.
/// Used throughout the theme configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ColorPair {
    fg: Color,
    bg: Color,
}

impl<'de> Deserialize<'de> for ColorPair {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ColorFormat {
            Short(String),
            Full {
                #[serde(default, deserialize_with = "deserialize_color_field")]
                fg: Color,
                #[serde(default, deserialize_with = "deserialize_color_field")]
                bg: Color,
            },
        }

        let inner = ColorFormat::deserialize(deserializer)?;
        match inner {
            ColorFormat::Short(s) => Ok(ColorPair {
                fg: text::parse_color(&s),
                bg: Color::Reset,
            }),
            ColorFormat::Full { fg, bg } => Ok(ColorPair { fg, bg }),
        }
    }
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
    crate::getters! {
        icon: &str,
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
        self.fg_or(Style::default().fg(Theme::internal_defaults().widget.color.fg))
    }

    /// Returns the background style, falling back to the internal default theme if Reset.
    pub(crate) fn bg_or_theme(&self) -> Style {
        self.bg_or(Style::default().bg(Theme::internal_defaults().widget.color.bg))
    }

    /// Returns the title style, falling back to the internal default theme if Reset.
    pub(crate) fn title_style_or_theme(&self) -> Style {
        self.title
            .style_or(&Theme::internal_defaults().widget.title)
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
            title: ColorPair {
                fg: Color::Magenta,
                ..ColorPair::default()
            },
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
            active: ColorPair {
                fg: Color::Yellow,
                ..Default::default()
            },
            inactive: ColorPair {
                fg: Color::Gray,
                ..Default::default()
            },
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
        self.active.style_or(&Theme::internal_defaults().tab.active)
    }

    /// Returns a Style for the inactive tab, using theme fallback if unset.
    pub(crate) fn inactive_style_or_theme(&self) -> Style {
        self.inactive
            .style_or(&Theme::internal_defaults().tab.inactive)
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
            perms: ColorPair {
                fg: Color::LightGreen,
                ..ColorPair::default()
            },
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
        let defaults = &Theme::internal_defaults().info;

        specific.resolve(&self.date).style_or(&defaults.date)
    }

    pub(crate) fn perms_style(&self) -> Style {
        self.perms.style_or(&Theme::internal_defaults().info.perms)
    }

    pub(crate) fn size_style(&self) -> Style {
        self.size.style_or(&Theme::internal_defaults().info.size)
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
        self.file_type
            .style_or(&Theme::internal_defaults().info.file_type)
    }

    #[cfg(unix)]
    pub(crate) fn owner_style(&self) -> Style {
        self.owner.style_or(&Theme::internal_defaults().info.owner)
    }

    #[cfg(unix)]
    pub(crate) fn group_style(&self) -> Style {
        self.group.style_or(&Theme::internal_defaults().info.group)
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
    Ok(text::parse_color(&s))
}

pub(crate) fn deserialize_color_map<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, Color>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw_map: HashMap<String, String> = HashMap::deserialize(deserializer)?;

    let processed_map = raw_map
        .into_iter()
        .map(|(key, val)| (key, text::parse_color(&val)))
        .collect();

    Ok(processed_map)
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

        tab: TabTheme {
            active: ColorPair {
                fg: primary,
                ..ColorPair::default()
            },
            inactive: ColorPair {
                fg: muted,
                ..ColorPair::default()
            },
            ..TabTheme::default()
        },
        ..Theme::default()
    }
}
