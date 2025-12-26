use crate::app::AppState;
use crate::ui::{ActionMode, InputMode};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::time::Instant;
use unicode_width::UnicodeWidthStr;

pub enum InputKey {
    Char(char),
    Name(&'static str),
}

#[derive(Clone, Copy, Debug)]
pub enum PopupPosition {
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

// Deserialize so that the runa.toml custom position and size can be made simpler instead of just
// standard serde [derive(Deserialize)]
// position = "top_left"
// position = "bottomright"
// position = [25, 60]
// position = { x = 42, y = 80 }
impl<'de> Deserialize<'de> for PopupPosition {
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
            Helper::Str(ref s) if s.eq_ignore_ascii_case("center") => Ok(PopupPosition::Center),
            Helper::Str(ref s) if s.eq_ignore_ascii_case("top") => Ok(PopupPosition::Top),
            Helper::Str(ref s) if s.eq_ignore_ascii_case("bottom") => Ok(PopupPosition::Bottom),
            Helper::Str(ref s) if s.eq_ignore_ascii_case("left") => Ok(PopupPosition::Left),
            Helper::Str(ref s) if s.eq_ignore_ascii_case("right") => Ok(PopupPosition::Right),
            Helper::Str(ref s)
                if s.eq_ignore_ascii_case("top_left") || s.eq_ignore_ascii_case("topleft") =>
            {
                Ok(PopupPosition::TopLeft)
            }
            Helper::Str(ref s)
                if s.eq_ignore_ascii_case("top_right") || s.eq_ignore_ascii_case("topright") =>
            {
                Ok(PopupPosition::TopRight)
            }
            Helper::Str(ref s)
                if s.eq_ignore_ascii_case("bottom_left")
                    || s.eq_ignore_ascii_case("bottomleft") =>
            {
                Ok(PopupPosition::BottomLeft)
            }
            Helper::Str(ref s)
                if s.eq_ignore_ascii_case("bottom_right")
                    || s.eq_ignore_ascii_case("bottomright") =>
            {
                Ok(PopupPosition::BottomRight)
            }
            Helper::Str(s) => Err(D::Error::custom(format!("invalid PopupPosition: '{}'", s))),
            Helper::Arr([x, y]) => Ok(PopupPosition::Custom(x, y)),
            Helper::XY { x, y } => Ok(PopupPosition::Custom(x, y)),
        }
    }
}

/// Preset popup sizes.
#[derive(Clone, Copy, Debug)]
pub enum PopupSize {
    Small,
    Medium,
    Large,
    Custom(u16, u16),
}

impl<'de> Deserialize<'de> for PopupSize {
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
            Helper::Str(ref s) if s.eq_ignore_ascii_case("small") => Ok(PopupSize::Small),
            Helper::Str(ref s) if s.eq_ignore_ascii_case("medium") => Ok(PopupSize::Medium),
            Helper::Str(ref s) if s.eq_ignore_ascii_case("large") => Ok(PopupSize::Large),
            Helper::Str(s) => Err(D::Error::custom(format!("invalid PopupSize: '{}'", s))),
            Helper::Arr([w, h]) => Ok(PopupSize::Custom(w, h)),
            Helper::Obj { w, h } => Ok(PopupSize::Custom(w, h)),
        }
    }
}

impl PopupSize {
    pub fn percentages(&self) -> (u16, u16) {
        match self {
            PopupSize::Small => (24, 7),
            PopupSize::Medium => (32, 14),
            PopupSize::Large => (32, 40),
            PopupSize::Custom(w, h) => (*w, *h),
        }
    }
}

pub struct PopupStyle {
    pub border: Borders,
    pub border_style: Style,
    pub bg: Style,
    pub fg: Style,
    pub title: Option<Span<'static>>,
}

impl Default for PopupStyle {
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

pub fn popup_area(area: Rect, size: PopupSize, pos: PopupPosition) -> Rect {
    let (w_pct, h_pct) = size.percentages();
    let w = (area.width * w_pct / 100).max(1).min(area.width);
    let h = (area.height * h_pct / 100).max(1).min(area.height);

    match pos {
        PopupPosition::Center => Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        },
        PopupPosition::Top => Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y,
            width: w,
            height: h,
        },
        PopupPosition::Bottom => Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + area.height - h,
            width: w,
            height: h,
        },
        PopupPosition::Left => Rect {
            x: area.x,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        },
        PopupPosition::Right => Rect {
            x: area.x + area.width - w,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        },
        PopupPosition::TopLeft => Rect {
            x: area.x,
            y: area.y,
            width: w,
            height: h,
        },
        PopupPosition::TopRight => Rect {
            x: area.x + area.width - w,
            y: area.y,
            width: w,
            height: h,
        },
        PopupPosition::BottomLeft => Rect {
            x: area.x,
            y: area.y + area.height - h,
            width: w,
            height: h,
        },
        PopupPosition::BottomRight => Rect {
            x: area.x + area.width - w,
            y: area.y + area.height - h,
            width: w,
            height: h,
        },
        PopupPosition::Custom(xp, yp) => {
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

pub fn draw_popup(
    frame: &mut Frame,
    area: Rect,
    pos: PopupPosition,
    size: PopupSize,
    style: &PopupStyle,
    content: impl Into<String>,
    alignment: Option<Alignment>,
) {
    let popup = popup_area(area, size, pos);

    frame.render_widget(Clear, popup);

    let mut block = Block::default()
        .borders(style.border)
        .border_style(style.border_style)
        .style(style.bg);

    if let Some(title) = &style.title {
        block = block.title(title.clone());
    }

    let para = Paragraph::new(content.into())
        .block(block)
        .alignment(alignment.unwrap_or(Alignment::Left));
    frame.render_widget(para, popup);
}

pub fn get_pane_block(title: &str, app: &AppState) -> Block<'static> {
    let mut block = Block::default();
    if app.config().display().is_split() {
        block = block
            .borders(Borders::ALL)
            .border_style(app.config().theme().accent().as_style());
        if app.config().display().titles() {
            block = block.title(title.to_string());
        }
    }
    block
}

pub fn draw_separator(frame: &mut Frame, area: Rect, style: Style) {
    frame.render_widget(
        Block::default().borders(Borders::LEFT).border_style(style),
        area,
    );
}

pub fn draw_input_popup(frame: &mut Frame, app: &AppState, accent_style: Style) {
    if let ActionMode::Input { mode, prompt } = &app.actions().mode() {
        let widget = app.config().theme().widget();
        let posititon = widget.position().unwrap_or(PopupPosition::Center);
        let size = widget.size().unwrap_or(PopupSize::Small);
        let confirm_size = widget.confirm_size_or(PopupSize::Large);

        if *mode == InputMode::ConfirmDelete {
            let action_targets = app.nav().get_action_targets();
            let targets: Vec<String> = action_targets
                .iter()
                .map(|p| {
                    p.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                })
                .collect();
            let preview = if targets.len() == 1 {
                format!("\nFile to delete: {}", targets[0])
            } else if targets.len() > 1 {
                format!(
                    "\nFiles to delete ({}):\n{}",
                    targets.len(),
                    targets
                        .iter()
                        .map(|n| format!("  - {}", n))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            } else {
                String::new()
            };

            let popup_style = PopupStyle {
                border: Borders::ALL,
                border_style: widget.border_or(Style::default().fg(Color::Red)),
                bg: widget.bg_or(Style::default().bg(Color::Reset)),
                fg: widget.fg_or(Style::default().fg(Color::Reset)),
                title: Some(" Confirm Delete ".into()),
            };
            draw_popup(
                frame,
                frame.area(),
                posititon,
                confirm_size,
                &popup_style,
                format!("{prompt}{preview}"),
                Some(Alignment::Left),
            );
        } else {
            let popup_style = PopupStyle {
                border: Borders::ALL,
                border_style: widget.border_or(accent_style),
                bg: widget.bg_or(Style::default().bg(Color::Reset)),
                fg: widget.fg_or(Style::default().fg(Color::Reset)),
                title: Some(Span::styled(
                    format!(" {} ", prompt),
                    widget.fg_or(Style::default().fg(Color::Reset)),
                )),
            };
            draw_popup(
                frame,
                frame.area(),
                posititon,
                size,
                &popup_style,
                app.actions().input_buffer(),
                Some(Alignment::Left),
            );

            let input_text = app.actions().input_buffer();
            let x_offset = UnicodeWidthStr::width(input_text) as u16;
            let popup_area = popup_area(frame.area(), size, posititon);
            frame.set_cursor_position((popup_area.x + 1 + x_offset, popup_area.y + 1));
        }
    }
}

pub fn draw_status_line(frame: &mut Frame, app: &crate::app::AppState) {
    let area = frame.area();

    let count = match app.actions().clipboard() {
        Some(set) => set.len(),
        None => 0,
    };
    let filter = app.nav().filter();
    let now = Instant::now();

    let mut parts = Vec::new();
    if count > 0 && (app.notification_time().is_some_and(|until| until > now)) {
        let yank_msg = { format!("Yanked files: {count}") };
        parts.push(yank_msg);
    }
    if !filter.is_empty() {
        parts.push(format!("Filter: \"{filter}\""));
    }

    let msg = parts.join(" | ");
    if !msg.is_empty() {
        let rect = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        let line = Line::from(Span::styled(msg, Style::default().fg(Color::Gray)));
        let paragraph = Paragraph::new(line).alignment(ratatui::layout::Alignment::Right);
        frame.render_widget(paragraph, rect);
    }
}
