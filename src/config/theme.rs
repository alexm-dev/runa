//! Theme configuration options for runa
//!
//! This module defines the theme configuration options which are read from the runa.toml
//! configuration file.
//!
//! Also holds the internal themes and the logic to apply user overrides on top of them.

use crate::ui::widgets::{DialogPosition, DialogSize};
use crate::utils::parse_color;
use ratatui::style::{Color, Style};
use serde::Deserialize;

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
pub struct Theme {
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
    marker: MarkerTheme,
    widget: WidgetTheme,
    /// info does not honor the .size field from widget.
    /// info gets auto-sized based on attributes enabled.
    info: WidgetTheme,
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
            marker: MarkerTheme::default(),
            widget: WidgetTheme::default(),
            info: WidgetTheme {
                title: ColorPair {
                    fg: Color::Magenta,
                    ..ColorPair::default()
                },
                ..WidgetTheme::default()
            },
        }
    }
}

/// Macro to override a field in the target theme if it differs from the default theme.
/// This is used to apply user-defined overrides on top of a preset theme.
macro_rules! override_if_changed {
    ($target:ident, $user:ident, $default:ident, $field:ident) => {
        if $user.$field != $default.$field {
            $target.$field = $user.$field.clone();
        }
    };
}

/// Theme implementation
/// Provides methods to access theme properties and apply user overrides.
impl Theme {
    pub fn accent(&self) -> ColorPair {
        self.accent
    }

    pub fn selection(&self) -> ColorPair {
        self.selection
    }

    pub fn underline(&self) -> ColorPair {
        self.underline
    }

    pub fn entry(&self) -> ColorPair {
        self.entry
    }

    pub fn directory(&self) -> ColorPair {
        self.directory
    }

    pub fn separator(&self) -> ColorPair {
        self.separator
    }

    pub fn selection_icon(&self) -> &str {
        &self.selection_icon
    }

    pub fn parent(&self) -> PaneTheme {
        self.parent
    }

    pub fn preview(&self) -> PaneTheme {
        self.preview
    }

    pub fn path(&self) -> ColorPair {
        self.path
    }

    pub fn status_line(&self) -> ColorPair {
        self.status_line
    }

    pub fn marker(&self) -> &MarkerTheme {
        &self.marker
    }

    pub fn widget(&self) -> &WidgetTheme {
        &self.widget
    }

    pub fn info(&self) -> &WidgetTheme {
        &self.info
    }

    pub fn with_overrides(self) -> Self {
        let preset = match self.name.as_deref() {
            Some("gruvbox-dark-hard") => Some(gruvbox_dark_hard()),
            Some("gruvbox-dark") => Some(gruvbox_dark()),
            Some("gruvbox-light") => Some(gruvbox_light()),

            Some("catppuccin-mocha") => Some(catppuccin_mocha()),
            Some("catppuccin-frappe") => Some(catppuccin_frappe()),
            Some("catppuccin-macchiato") => Some(catppuccin_mocha()),
            Some("catppuccin-latte") => Some(catppuccin_latte()),

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

    pub fn bat_theme_name(&self) -> &'static str {
        self.name
            .as_deref()
            .map(Theme::map_to_bat_theme)
            .unwrap_or("base16-256.dark")
    }

    fn map_to_bat_theme(internal_theme: &str) -> &'static str {
        match internal_theme {
            "gruvbox-dark" | "gruvbox-dark-hard" | "gruvbox" => "gruvbox-dark",
            "gruvbox-light" => "gruvbox-light",
            "tokyonight-night" | "tokyonight" | "tokyonight-storm" => "TwoDark",
            "catppuccin-latte" => "Catppuccin Latte",
            "catppuccin-frappe" => "Catppuccin Frappe",
            "catppuccin-macchiato" => "Catppuccin Macchiato",
            "catppuccin-mocha" | "catppuccin" => "Catppuccin Mocha",
            "nightfox" | "carbonfox" | "rose-pine" | "everforest" => "TwoDark",
            _ => "TwoDark",
        }
    }

    /// Apply user overrides on top of the current theme.
    /// Compares each field with the default theme and overrides if changed
    /// This allows to only specify the fields they want to change
    fn apply_user_overrides(&mut self, user: Theme) {
        let defaults = Theme::default();

        override_if_changed!(self, user, defaults, accent);
        override_if_changed!(self, user, defaults, selection);
        override_if_changed!(self, user, defaults, underline);
        override_if_changed!(self, user, defaults, entry);
        override_if_changed!(self, user, defaults, directory);
        override_if_changed!(self, user, defaults, separator);
        override_if_changed!(self, user, defaults, parent);
        override_if_changed!(self, user, defaults, preview);
        override_if_changed!(self, user, defaults, path);
        override_if_changed!(self, user, defaults, status_line);
        override_if_changed!(self, user, defaults, selection_icon);
        override_if_changed!(self, user, defaults, marker);
        override_if_changed!(self, user, defaults, widget);
        override_if_changed!(self, user, defaults, info);

        if user.name.is_some() {
            self.name = user.name.clone();
        }
    }
}

/// ColorPair struct to hold foreground and background colors.
/// Used throughout the theme configuration.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct ColorPair {
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
    /// Convert ColorPair to Style
    pub fn as_style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    /// Get effective style, using fallback colors if Reset is set
    pub fn effective_style(&self, fallback: &ColorPair) -> Style {
        let fg = if self.fg == Color::Reset {
            fallback.fg
        } else {
            self.fg
        };
        let bg = if self.bg == Color::Reset {
            fallback.bg
        } else {
            self.bg
        };
        Style::default().fg(fg).bg(bg)
    }

    // Getters for fg and bg colors
    pub fn fg(&self) -> Color {
        self.fg
    }
    pub fn bg(&self) -> Color {
        self.bg
    }
}

/// PaneTheme struct to hold color and selection styles for panes.
/// Used for parent and preview panes.
#[derive(Deserialize, Debug, PartialEq, Clone, Copy, Default)]
#[serde(default)]
pub struct PaneTheme {
    color: ColorPair,
    selection: Option<ColorPair>,
}

/// Similar to ColorPair implementation
/// Provides methods to convert to Style and get effective styles.
impl PaneTheme {
    pub fn as_style(&self) -> Style {
        self.color.as_style()
    }

    pub fn selection_style(&self, fallback: Style) -> Style {
        match self.selection {
            Some(sel) => sel.as_style(),
            None => fallback,
        }
    }

    pub fn effective_style(&self, fallback: &ColorPair) -> Style {
        self.color.effective_style(fallback)
    }

    pub fn fg(&self) -> Color {
        self.color.fg()
    }
    pub fn bg(&self) -> Color {
        self.color.bg()
    }
}

/// MarkerTheme struct to hold marker icon and colors.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct MarkerTheme {
    icon: String,
    #[serde(flatten)]
    color: ColorPair,
    /// Optional clipboard color pair
    /// sets the color of the copy/paste marker
    clipboard: Option<ColorPair>,
}

impl MarkerTheme {
    pub fn icon(&self) -> &str {
        &self.icon
    }
    pub fn color(&self) -> &ColorPair {
        &self.color
    }
    pub fn clipboard(&self) -> Option<&ColorPair> {
        self.clipboard.as_ref()
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
pub struct WidgetTheme {
    color: ColorPair,
    border: ColorPair,
    title: ColorPair,
    position: Option<DialogPosition>,
    size: Option<DialogSize>,
    confirm_size: Option<DialogSize>,
    find_visible_results: Option<usize>,
    find_width: Option<u16>,
}

impl WidgetTheme {
    pub fn position(&self) -> &Option<DialogPosition> {
        &self.position
    }

    pub fn size(&self) -> &Option<DialogSize> {
        &self.size
    }

    pub fn confirm_size(&self) -> &Option<DialogSize> {
        &self.confirm_size
    }

    /// Returns the confirm dialog size, falling back to the general size, and then to the provided fallback.
    pub fn confirm_size_or(&self, fallback: DialogSize) -> DialogSize {
        self.confirm_size()
            .as_ref()
            .or_else(|| self.size().as_ref())
            .copied()
            .unwrap_or(fallback)
    }

    /// Returns the border style, falling back to the provided style for Reset colors.
    pub fn border_style_or(&self, fallback: Style) -> Style {
        let fg = if self.border.fg != Color::Reset {
            self.border.fg
        } else {
            fallback.fg.unwrap_or(Color::Reset)
        };

        let bg = if self.border.bg != Color::Reset {
            self.border.bg
        } else {
            fallback.bg.unwrap_or(Color::Reset)
        };

        fallback.fg(fg).bg(bg)
    }

    /// Returns the foreground style, falling back to the provided style if Reset.
    pub fn fg_or(&self, fallback: Style) -> Style {
        if self.color.fg() == Color::Reset {
            fallback
        } else {
            Style::default().fg(self.color.fg())
        }
    }

    /// Returns the background style, falling back to the provided style if Reset.
    pub fn bg_or(&self, fallback: Style) -> Style {
        if self.color.bg() == Color::Reset {
            fallback
        } else {
            Style::default().bg(self.color.bg())
        }
    }

    /// Returns the title style, falling back to the provided style for Reset colors.
    pub fn title_style_or(&self, fallback: Style) -> Style {
        let fg = if self.title.fg != Color::Reset {
            self.title.fg
        } else {
            fallback.fg.unwrap_or(Color::Reset)
        };
        let bg = if self.title.bg != Color::Reset {
            self.title.bg
        } else {
            fallback.bg.unwrap_or(Color::Reset)
        };
        fallback.fg(fg).bg(bg)
    }

    pub fn find_visible_or(&self, fallback: usize) -> usize {
        self.find_visible_results.unwrap_or(fallback)
    }

    pub fn find_width_or(&self, fallback: u16) -> u16 {
        self.find_width.unwrap_or(fallback)
    }
}

/// Default implementation for WidgetTheme
impl Default for WidgetTheme {
    fn default() -> Self {
        WidgetTheme {
            color: ColorPair::default(),
            border: ColorPair::default(),
            title: ColorPair::default(),
            position: Some(DialogPosition::Center),
            size: Some(DialogSize::Small),
            confirm_size: Some(DialogSize::Large),
            find_visible_results: Some(5),
            find_width: Some(40),
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
pub struct Palette {
    pub base: (u8, u8, u8),
    pub surface: (u8, u8, u8),
    pub overlay: (u8, u8, u8),
    pub primary: (u8, u8, u8),
    pub secondary: (u8, u8, u8),
    pub directory: (u8, u8, u8),
}

/// Centralized function to create a Theme from a Palette.
pub fn make_theme(name: &str, palette: Palette, icon: &str) -> Theme {
    let primary = rgb(palette.primary);
    let secondary = rgb(palette.secondary);
    let muted = rgb(palette.overlay);
    let struct_color = rgb(palette.surface);
    let base_bg = rgb(palette.base);
    let dir_color = rgb(palette.directory);

    Theme {
        name: Some(name.to_string()),
        accent: ColorPair {
            fg: struct_color,
            ..ColorPair::default()
        },
        selection: ColorPair {
            bg: struct_color,
            ..ColorPair::default()
        },
        directory: ColorPair {
            fg: dir_color,
            ..ColorPair::default()
        },
        separator: ColorPair {
            fg: struct_color,
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
                fg: muted,
                ..ColorPair::default()
            },
            border: ColorPair {
                fg: struct_color,
                ..ColorPair::default()
            },
            ..WidgetTheme::default()
        },
        info: WidgetTheme {
            title: ColorPair {
                fg: secondary,
                ..ColorPair::default()
            },
            border: ColorPair {
                fg: struct_color,
                ..ColorPair::default()
            },
            ..WidgetTheme::default()
        },
        ..Theme::default()
    }
}

// Theme palettes

const TOKYO_STORM: Palette = Palette {
    base: (36, 40, 59),
    surface: (41, 46, 66),
    overlay: (86, 95, 137),
    primary: (187, 154, 247),
    secondary: (125, 207, 255),
    directory: (122, 162, 247),
};

const TOKYO_NIGHT: Palette = Palette {
    base: (26, 27, 38),
    surface: (44, 51, 78),
    overlay: (86, 95, 137),
    primary: (187, 154, 247),
    secondary: (125, 207, 255),
    directory: (122, 162, 247),
};

const TOKYO_DAY: Palette = Palette {
    base: (225, 226, 231),
    surface: (196, 199, 209),
    overlay: (168, 175, 199),
    primary: (152, 94, 171),
    secondary: (52, 90, 183),
    directory: (52, 90, 183),
};

pub fn tokyonight_storm() -> Theme {
    make_theme("tokyonight-storm", TOKYO_STORM, "┃")
}
pub fn tokyonight_night() -> Theme {
    make_theme("tokyonight-night", TOKYO_NIGHT, "┃")
}
pub fn tokyonight_day() -> Theme {
    make_theme("tokyonight-day", TOKYO_DAY, "┃")
}

const GRUV_DARK_HARD: Palette = Palette {
    base: (29, 32, 33),
    surface: (60, 56, 54),
    overlay: (146, 131, 116),
    primary: (211, 134, 155),
    secondary: (142, 192, 124),
    directory: (131, 165, 152),
};

const GRUV_DARK: Palette = Palette {
    base: (40, 40, 40),
    surface: (60, 56, 54),
    overlay: (146, 131, 116),
    primary: (211, 134, 155),
    secondary: (142, 192, 124),
    directory: (131, 165, 152),
};

const GRUV_LIGHT: Palette = Palette {
    base: (251, 241, 199),
    surface: (213, 196, 161),
    overlay: (124, 111, 100),
    primary: (143, 63, 113),
    secondary: (66, 123, 88),
    directory: (7, 102, 120),
};

pub fn gruvbox_dark_hard() -> Theme {
    make_theme("gruvbox-dark-hard", GRUV_DARK_HARD, "*")
}
pub fn gruvbox_dark() -> Theme {
    make_theme("gruvbox-dark", GRUV_DARK, "*")
}
pub fn gruvbox_light() -> Theme {
    make_theme("gruvbox-light", GRUV_LIGHT, "*")
}

const MOCHA: Palette = Palette {
    base: (30, 30, 46),
    surface: (49, 50, 68),
    overlay: (108, 112, 134),
    primary: (203, 166, 247),
    secondary: (148, 226, 213),
    directory: (137, 180, 250),
};

const FRAPPE: Palette = Palette {
    base: (48, 52, 70),
    surface: (65, 69, 89),
    overlay: (115, 121, 148),
    primary: (202, 158, 230),
    secondary: (129, 200, 190),
    directory: (140, 170, 238),
};

const LATTE: Palette = Palette {
    base: (239, 241, 245),
    surface: (204, 208, 218),
    overlay: (156, 160, 176),
    primary: (136, 57, 239),
    secondary: (23, 146, 153),
    directory: (30, 102, 245),
};

pub fn catppuccin_mocha() -> Theme {
    make_theme("catppuccin-mocha", MOCHA, "┃")
}
pub fn catppuccin_frappe() -> Theme {
    make_theme("catppuccin-frappe", FRAPPE, "┃")
}
pub fn catppuccin_latte() -> Theme {
    make_theme("catppuccin-latte", LATTE, "┃")
}

const CARBON: Palette = Palette {
    base: (22, 22, 22),
    surface: (42, 42, 42),
    overlay: (82, 82, 82),
    primary: (190, 149, 233),
    secondary: (61, 187, 199),
    directory: (120, 169, 235),
};

const NIGHTFOX: Palette = Palette {
    base: (25, 30, 36),
    surface: (43, 51, 63),
    overlay: (87, 91, 112),
    primary: (195, 157, 239),
    secondary: (99, 199, 209),
    directory: (113, 161, 236),
};

pub fn carbonfox() -> Theme {
    make_theme("carbonfox", CARBON, "┃")
}
pub fn nightfox() -> Theme {
    make_theme("nightfox", NIGHTFOX, "┃")
}

const FOREST: Palette = Palette {
    base: (43, 51, 57),
    surface: (74, 82, 88),
    overlay: (133, 146, 137),
    primary: (167, 192, 128),
    secondary: (230, 126, 128),
    directory: (127, 187, 179),
};

const ROSE_PINE: Palette = Palette {
    base: (25, 23, 36),
    surface: (31, 29, 46),
    overlay: (110, 106, 134),
    primary: (196, 167, 231),
    secondary: (235, 188, 186),
    directory: (49, 116, 143),
};

pub fn everforest() -> Theme {
    make_theme("everforest", FOREST, "*")
}
pub fn rose_pine() -> Theme {
    make_theme("rose_pine", ROSE_PINE, "*")
}
