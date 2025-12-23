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
            selection_icon: "> ".into(),
            parent: ColorPair::default(),
            preview: ColorPair::default(),
            path: ColorPair {
                fg: Color::Magenta,
                ..ColorPair::default()
            },
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
