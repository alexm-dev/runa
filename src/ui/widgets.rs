use crate::app::AppState;
use crate::ui::{ActionMode, InputMode};
use crate::ui::{Constraint, Direction, Layout};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::time::Instant;
use unicode_width::UnicodeWidthStr;

pub enum InputKey {
    Char(char),
    Name(&'static str),
}

#[derive(Clone, Copy)]
pub enum PopupPosition {
    Center,
    Top,
    Bottom,
    Left,
    Right,
    Custom(u16, u16), // (x%, y%)
}

/// Preset popup sizes.
#[derive(Clone, Copy)]
pub enum PopupSize {
    Small,
    Medium,
    Large,
    Custom(u16, u16), // (w%, h%)
}

impl PopupSize {
    pub fn percentages(&self) -> (u16, u16) {
        match self {
            PopupSize::Small => (32, 10),
            PopupSize::Medium => (48, 14),
            PopupSize::Large => (80, 20),
            PopupSize::Custom(w, h) => (*w, *h),
        }
    }
}

pub struct PopupStyle {
    pub border: Borders,
    pub border_style: Style,
    pub bg: Style,
    pub title: Option<String>,
}

impl Default for PopupStyle {
    fn default() -> Self {
        Self {
            border: Borders::ALL,
            border_style: Style::default().fg(Color::White),
            bg: Style::default().bg(Color::Black),
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
        PopupPosition::Custom(xp, yp) => {
            // Clamp custom offset so popup always fits
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
    let mut b = Block::default();
    if app.config().display().is_split() {
        b = b
            .borders(Borders::ALL)
            .border_style(app.config().theme().accent().as_style());
        if app.config().display().titles() {
            b = b.title(title.to_string());
        }
    }
    b
}

pub fn draw_separator(frame: &mut Frame, area: Rect, style: Style) {
    frame.render_widget(
        Block::default().borders(Borders::LEFT).border_style(style),
        area,
    );
}

pub fn draw_confirm_popup(frame: &mut Frame, area: Rect, prompt: &str) {
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(40),
        ])
        .split(area);

    let popup_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(vertical_chunks[1])[1];

    frame.render_widget(ratatui::widgets::Clear, popup_area);

    let block = Block::default()
        .title(" Confirm Delete ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Red));

    let text = Paragraph::new(format!("\n{}", prompt))
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(text, popup_area);
}

pub fn draw_input_popup(frame: &mut Frame, app: &AppState, accent_style: Style) {
    if let ActionMode::Input { mode, prompt } = &app.actions().mode() {
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
                border_style: Style::default().fg(ratatui::style::Color::Red),
                bg: app.config().theme().notification().as_style(),
                title: Some(" Confirm Delete ".into()),
            };
            draw_popup(
                frame,
                frame.area(),
                PopupPosition::Center,
                PopupSize::Medium,
                &popup_style,
                format!("{prompt}{preview}"),
                Some(Alignment::Left),
            );
        } else {
            let popup_style = PopupStyle {
                border: Borders::ALL,
                border_style: accent_style,
                bg: app.config().theme().notification().as_style(),
                title: Some(format!(" {} ", prompt)),
            };
            draw_popup(
                frame,
                frame.area(),
                PopupPosition::Center,
                PopupSize::Medium,
                &popup_style,
                app.actions().input_buffer(),
                Some(Alignment::Left),
            );

            let input_text = app.actions().input_buffer();
            let x_offset = UnicodeWidthStr::width(input_text) as u16;
            let popup_area = popup_area(frame.area(), PopupSize::Medium, PopupPosition::Center);
            frame.set_cursor_position((popup_area.x + 1 + x_offset, popup_area.y + 1));
        }
    }
}

pub fn draw_status_line(frame: &mut Frame, app: &crate::app::AppState) {
    let area = frame.area();

    let (count, is_cut) = match app.actions().clipboard() {
        Some(set) => (set.len(), app.actions().is_cut()),
        None => (0, false),
    };
    let filter = app.nav().filter();
    let now = Instant::now();

    let msg = if let Some(until) = app.notification_time() {
        if until > &now && count > 0 {
            if is_cut {
                format!("Cut: {count}")
            } else {
                format!("Yanked files: {count}")
            }
        } else if !filter.is_empty() {
            format!("Filter: \"{filter}\"")
        } else {
            String::new()
        }
    } else if !filter.is_empty() {
        format!("Filter: \"{filter}\"")
    } else {
        String::new()
    };

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
