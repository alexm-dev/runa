//! Theme configuration options for runa
//!
//! This module defines the theme configuration options which are read from the runa.toml
//! configuration file.

use crate::ui::widgets::{DialogPosition, DialogSize};
use crate::utils::parse_color;
use ratatui::style::{Color, Style};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct ColorPair {
    #[serde(default, deserialize_with = "deserialize_color_field")]
    fg: Color,
    #[serde(default, deserialize_with = "deserialize_color_field")]
    bg: Color,

    #[serde(default, deserialize_with = "deserialize_optional_color_field")]
    selection_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_optional_color_field")]
    selection_bg: Option<Color>,
}

impl Default for ColorPair {
    fn default() -> Self {
        Self {
            fg: Color::Reset,
            bg: Color::Reset,
            selection_fg: None,
            selection_bg: None,
        }
    }
}

impl ColorPair {
    pub fn as_style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    pub fn selection_style(&self, global_default: Style) -> Style {
        let mut style = global_default;

        if let Some(fg) = self.selection_fg {
            style = style.fg(fg);
        }

        if let Some(bg) = self.selection_bg {
            style = style.bg(bg);
        }

        style
    }

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

    pub fn fg(&self) -> Color {
        self.fg
    }
    pub fn bg(&self) -> Color {
        self.bg
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct MarkerTheme {
    #[serde(default)]
    icon: String,
    #[serde(flatten)]
    color: ColorPair,
}

impl MarkerTheme {
    pub fn icon(&self) -> &str {
        &self.icon
    }
    pub fn color(&self) -> &ColorPair {
        &self.color
    }
}

impl Default for MarkerTheme {
    fn default() -> Self {
        MarkerTheme {
            icon: "*".to_string(),
            color: ColorPair {
                fg: Color::Yellow,
                bg: Color::Reset,
                selection_fg: None,
                selection_bg: None,
            },
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct WidgetTheme {
    #[serde(default)]
    color: ColorPair,
    #[serde(default)]
    border: ColorPair,
    #[serde(default)]
    title: ColorPair,
    #[serde(default)]
    position: Option<DialogPosition>,
    #[serde(default)]
    size: Option<DialogSize>,
    #[serde(default)]
    confirm_size: Option<DialogSize>,
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

    pub fn confirm_size_or(&self, fallback: DialogSize) -> DialogSize {
        self.confirm_size()
            .as_ref()
            .or_else(|| self.size().as_ref())
            .copied()
            .unwrap_or(fallback)
    }

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

    pub fn fg_or(&self, fallback: Style) -> Style {
        if self.color.fg() == Color::Reset {
            fallback
        } else {
            Style::default().fg(self.color.fg())
        }
    }

    pub fn bg_or(&self, fallback: Style) -> Style {
        if self.color.bg() == Color::Reset {
            fallback
        } else {
            Style::default().bg(self.color.bg())
        }
    }

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
}

impl Default for WidgetTheme {
    fn default() -> Self {
        WidgetTheme {
            color: ColorPair::default(),
            border: ColorPair::default(),
            title: ColorPair::default(),
            position: Some(DialogPosition::Center),
            size: Some(DialogSize::Medium),
            confirm_size: Some(DialogSize::Large),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Theme {
    selection: ColorPair,
    underline: ColorPair,
    accent: ColorPair,
    entry: ColorPair,
    directory: ColorPair,
    separator: ColorPair,
    selection_icon: String,
    parent: ColorPair,
    preview: ColorPair,
    path: ColorPair,
    status_line: ColorPair,
    marker: MarkerTheme,
    widget: WidgetTheme,
    /// info does not honor the .size field from widget.
    /// info gets auto-sized based on attributes enabled.
    info: WidgetTheme,
}

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

    pub fn parent(&self) -> ColorPair {
        self.parent
    }

    pub fn preview(&self) -> ColorPair {
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
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            accent: ColorPair::default(),
            selection: ColorPair::default(),
            underline: ColorPair::default(),
            entry: ColorPair::default(),
            directory: ColorPair {
                fg: Color::Cyan,
                ..ColorPair::default()
            },
            separator: ColorPair::default(),
            selection_icon: ">".into(),
            parent: ColorPair::default(),
            preview: ColorPair::default(),
            path: ColorPair {
                fg: Color::Magenta,
                ..ColorPair::default()
            },
            status_line: ColorPair::default(),
            marker: MarkerTheme::default(),
            widget: WidgetTheme::default(),
            info: WidgetTheme::default(),
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

// Helper function to deserialize optinals Colors for Themes
fn deserialize_optional_color_field<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) if s.to_lowercase() != "default" => Ok(Some(parse_color(&s))),
        _ => Ok(None),
    }
}
