//! runa TUI dialog widget module.
//!
//! This module mostly holds dialog widget logic to help draw functions with positioning, size,
//! area and style.

use crate::app::AppState;
use ratatui::widgets::BorderType;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Clear, Paragraph},
};
use serde::de::Error;
use serde::{Deserialize, Deserializer};

/// Input keys used to input events.
///
/// Used to determine over character keys and named keys.
pub enum InputKey {
    Char(char),
    Name(&'static str),
}

/// Specifies possible dialog positions within the TUI frame.
/// Also possible to customize the position via the runa.toml
///
/// Is used to determine where dialog/widgets such as dialogs and input boxes are rendered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DialogPosition {
    Center,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Custom(u16, u16),
}

/// Deserialize so that the runa.toml custom position and size can be made simpler instead of just
/// standard serde [derive(Deserialize)]
/// position = "top_left"
/// position = "bottomright"
/// position = [25, 60]
/// position = { x = 42, y = 80 }
impl<'de> Deserialize<'de> for DialogPosition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Str(String),
            Arr([u16; 2]),
            XY { x: u16, y: u16 },
        }

        match Helper::deserialize(deserializer)? {
            Helper::Str(ref s) if s.eq_ignore_ascii_case("center") => Ok(DialogPosition::Center),
            Helper::Str(ref s) if s.eq_ignore_ascii_case("top") => Ok(DialogPosition::Top),
            Helper::Str(ref s) if s.eq_ignore_ascii_case("bottom") => Ok(DialogPosition::Bottom),
            Helper::Str(ref s) if s.eq_ignore_ascii_case("left") => Ok(DialogPosition::Left),
            Helper::Str(ref s) if s.eq_ignore_ascii_case("right") => Ok(DialogPosition::Right),
            Helper::Str(ref s)
                if s.eq_ignore_ascii_case("top_left") || s.eq_ignore_ascii_case("topleft") =>
            {
                Ok(DialogPosition::TopLeft)
            }
            Helper::Str(ref s)
                if s.eq_ignore_ascii_case("top_right") || s.eq_ignore_ascii_case("topright") =>
            {
                Ok(DialogPosition::TopRight)
            }
            Helper::Str(ref s)
                if s.eq_ignore_ascii_case("bottom_left")
                    || s.eq_ignore_ascii_case("bottomleft") =>
            {
                Ok(DialogPosition::BottomLeft)
            }
            Helper::Str(ref s)
                if s.eq_ignore_ascii_case("bottom_right")
                    || s.eq_ignore_ascii_case("bottomright") =>
            {
                Ok(DialogPosition::BottomRight)
            }
            Helper::Str(s) => Err(D::Error::custom(format!("invalid DialogPosition: '{}'", s))),
            Helper::Arr([x, y]) => Ok(DialogPosition::Custom(x, y)),
            Helper::XY { x, y } => Ok(DialogPosition::Custom(x, y)),
        }
    }
}

/// Preset for all dialogs/widgets sizes as well as a customized size via the runa.toml
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DialogSize {
    Small,
    Medium,
    Large,
    Custom(u16, u16),
}

/// Deserializer so that the runa.toml configuration can be made simpler to configure the size of
/// dialogs/widgets
///
/// size = "small"
/// size = [10, 10]
/// size = { w = 10, h = 20 }
impl<'de> Deserialize<'de> for DialogSize {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Str(String),
            Arr([u16; 2]),
            Obj { w: u16, h: u16 },
        }

        match Helper::deserialize(deserializer)? {
            Helper::Str(ref s) if s.eq_ignore_ascii_case("small") => Ok(DialogSize::Small),
            Helper::Str(ref s) if s.eq_ignore_ascii_case("medium") => Ok(DialogSize::Medium),
            Helper::Str(ref s) if s.eq_ignore_ascii_case("large") => Ok(DialogSize::Large),
            Helper::Str(s) => Err(D::Error::custom(format!("invalid DialogSize: '{}'", s))),
            Helper::Arr([w, h]) => Ok(DialogSize::Custom(w, h)),
            Helper::Obj { w, h } => Ok(DialogSize::Custom(w, h)),
        }
    }
}

/// Struct to hold the dialog style.
///
/// Includes the dialog border, border_style, the background/foreground and the title.
pub struct DialogStyle {
    pub border: Borders,
    pub border_style: Style,
    pub bg: Style,
    pub fg: Style,
    pub title: Option<Span<'static>>,
}

impl Default for DialogStyle {
    fn default() -> Self {
        Self {
            border: Borders::ALL,
            border_style: Style::default().fg(Color::White),
            bg: Style::default().bg(Color::Black),
            fg: Style::default().fg(Color::Reset),
            title: None,
        }
    }
}

/// Struct to hold the overall layout of a dialog widget
pub struct DialogLayout {
    pub area: Rect,
    pub position: DialogPosition,
    pub size: DialogSize,
}

/// Function to correctly calculate the area of the dialog
///
/// Returns the Rect of the calculated are of the dialog
pub fn dialog_area(area: Rect, size: DialogSize, pos: DialogPosition) -> Rect {
    let min_w = 7;
    let min_h = 3;

    let (w, h) = match size {
        DialogSize::Small => (
            (area.width * 24 / 100).max(min_w).min(area.width),
            (area.height * 7 / 100).max(min_h).min(area.height),
        ),
        DialogSize::Medium => (
            (area.width * 26 / 100).max(min_w).min(area.width),
            (area.height * 14 / 100).max(min_h).min(area.height),
        ),
        DialogSize::Large => (
            (area.width * 32 / 100).max(min_w).min(area.width),
            (area.height * 40 / 100).max(min_h).min(area.height),
        ),
        DialogSize::Custom(w_cells, h_cells) => (
            w_cells.max(min_w).min(area.width),
            h_cells.max(min_h).min(area.height),
        ),
    };

    match pos {
        DialogPosition::Center => Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        },
        DialogPosition::Top => Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y,
            width: w,
            height: h,
        },
        DialogPosition::Bottom => Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + area.height - h,
            width: w,
            height: h,
        },
        DialogPosition::Left => Rect {
            x: area.x,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        },
        DialogPosition::Right => Rect {
            x: area.x + area.width - w,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        },
        DialogPosition::TopLeft => Rect {
            x: area.x,
            y: area.y,
            width: w,
            height: h,
        },
        DialogPosition::TopRight => Rect {
            x: area.x + area.width - w,
            y: area.y,
            width: w,
            height: h,
        },
        DialogPosition::BottomLeft => Rect {
            x: area.x,
            y: area.y + area.height - h,
            width: w,
            height: h,
        },
        DialogPosition::BottomRight => Rect {
            x: area.x + area.width - w,
            y: area.y + area.height - h,
            width: w,
            height: h,
        },
        DialogPosition::Custom(xp, yp) => {
            let x = area.x + ((area.width - w) * xp / 100).min(area.width - w);
            let y = area.y + ((area.height - h) * yp / 100).min(area.height - h);
            Rect {
                x,
                y,
                width: w,
                height: h,
            }
        }
    }
}

/// Draws the dialog widgets
/// Takes the frame area as a rect, sets the position of the dialog and the overall style.
pub fn draw_dialog<'a, T>(
    frame: &mut Frame,
    layout: DialogLayout,
    border: BorderType,
    style: &DialogStyle,
    content: T,
    alignment: Option<Alignment>,
) where
    T: Into<Text<'a>>,
{
    let dialog = dialog_area(layout.area, layout.size, layout.position);

    frame.render_widget(Clear, dialog);

    let mut block = Block::default()
        .borders(style.border)
        .border_style(style.border_style)
        .border_type(border)
        .style(style.bg);

    if let Some(title) = &style.title {
        block = block.title(title.clone());
    }

    // Now uses T: Into<Text>, which is way faster for Vec<Line>
    let para = Paragraph::new(content.into())
        .block(block)
        .alignment(alignment.unwrap_or(Alignment::Left))
        .style(style.fg);

    frame.render_widget(para, dialog);
}

/// Getter for the overall pane block,
pub fn get_pane_block(title: &str, app: &AppState) -> Block<'static> {
    let mut block = Block::default();
    if app.config().display().is_split() {
        block = block
            .borders(Borders::ALL)
            .border_style(app.config().theme().accent_style());
        if app.config().display().titles() {
            block = block.title(title.to_string());
        }
    }
    block
}
