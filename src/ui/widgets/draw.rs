//! Draw widget module which holds all the draw functions for the render to use.
//!
//! Relies on helpers and data structs from [widgets::dialog]
//!
//! All draw functions are then used by [ui::render] to then draw widgets such a input dialog,
//! which is used by file action functions like rename and more..

use crate::app::AppState;
use crate::app::actions::{ActionMode, InputMode};
use crate::core::{FileInfo, file_manager::FileType};
use crate::ui::widgets::{
    DialogLayout, DialogPosition, DialogSize, DialogStyle, dialog_area, draw_dialog,
};
use crate::utils::{format_file_size, format_file_time, format_file_type};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::time::Instant;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Draws the seperator line when enabled inside runa.toml
pub fn draw_separator(frame: &mut Frame, area: Rect, style: Style) {
    frame.render_widget(
        Block::default().borders(Borders::LEFT).border_style(style),
        area,
    );
}

/// Either for ConfirmDelete or for anything else that requires input.
/// For other than ConfirmDelete, calculates the exact input field.
pub fn draw_input_dialog(frame: &mut Frame, app: &AppState, accent_style: Style) {
    if let ActionMode::Input { mode, prompt } = &app.actions().mode() {
        let widget = app.config().theme().widget();
        let position = dialog_position_unified(widget.position(), app, DialogPosition::Center);
        let size = widget.size().unwrap_or(DialogSize::Small);
        let confirm_size = widget.confirm_size_or(DialogSize::Large);
        let border_type = app.config().display().border_shape().as_border_type();

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

            let dialog_style = DialogStyle {
                border: Borders::ALL,
                border_style: widget.border_style_or(Style::default().fg(Color::Red)),
                bg: widget.bg_or(Style::default().bg(Color::Reset)),
                fg: widget.fg_or(Style::default().fg(Color::Reset)),
                title: Some(Span::styled(
                    " Confirm Delete ",
                    Style::default().fg(Color::Red),
                )),
            };

            let dialog_layout = DialogLayout {
                area: frame.area(),
                position,
                size: confirm_size,
            };

            draw_dialog(
                frame,
                dialog_layout,
                border_type,
                &dialog_style,
                format!("{prompt}{preview}"),
                Some(Alignment::Left),
            );
        } else {
            let dialog_style = DialogStyle {
                border: Borders::ALL,
                border_style: widget.border_style_or(accent_style),
                bg: widget.bg_or(Style::default().bg(Color::Reset)),
                fg: widget.fg_or(Style::default().fg(Color::Reset)),
                title: Some(Span::styled(
                    format!(" {} ", prompt),
                    widget.title_style_or(Style::default()),
                )),
            };

            let dialog_layout = DialogLayout {
                area: frame.area(),
                position,
                size,
            };

            let input_text = app.actions().input_buffer();
            let cursor_pos = app.actions().input_cursor_pos();
            let dialog_area = dialog_area(frame.area(), size, position);
            let visible_width = dialog_area.width.saturating_sub(2) as usize;

            let (display_input, cursor_offset) =
                input_field_view(input_text, cursor_pos, visible_width);

            draw_dialog(
                frame,
                dialog_layout,
                border_type,
                &dialog_style,
                display_input,
                Some(Alignment::Left),
            );

            frame
                .set_cursor_position((dialog_area.x + 1 + cursor_offset as u16, dialog_area.y + 1));
        }
    }
}

/// Draw the status line at the top right
/// Used for indication of number of copied/yanked files and the current applied filter
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
        let pad = 2;
        let padded_width = area.width.saturating_sub(pad);
        let rect = Rect {
            x: area.x,
            y: area.y,
            width: padded_width,
            height: 1,
        };
        let style = app.config().theme().status_line().as_style();
        let line = Line::from(Span::styled(msg, style));
        let paragraph = Paragraph::new(line).alignment(ratatui::layout::Alignment::Right);
        frame.render_widget(paragraph, rect);
    }
}

/// Helper function to calculate cursor offset for cursor moving
/// Handles horizontal truncation, variable width with unicode_width and clamps cursor to buffer.
/// Is used for draw widgets/dialogs with input fields.
fn input_field_view(input_text: &str, cursor_pos: usize, visible_width: usize) -> (&str, usize) {
    let cursor_pos = cursor_pos.min(input_text.len());
    let input_width = input_text.width();
    if input_width <= visible_width {
        let cursor_offset =
            unicode_width::UnicodeWidthStr::width(&input_text[..cursor_pos.min(input_text.len())]);
        (input_text, cursor_offset)
    } else {
        let mut current_w = 0;
        let mut start = input_text.len();
        for (idx, ch) in input_text.char_indices().rev() {
            current_w += ch.width().unwrap_or(0);
            if current_w > visible_width {
                start = idx + ch.len_utf8();
                break;
            }
        }

        let cursor_offset = if cursor_pos < start {
            0
        } else {
            unicode_width::UnicodeWidthStr::width(
                &input_text[start..cursor_pos.min(input_text.len())],
            )
        };

        (&input_text[start..], cursor_offset)
    }
}

/// Draw the show info dialog with file information
/// such as name, type, size, modified time and permissions.
///
/// Takes the app state, accent style and the overlay to check if it is ShowInfo
/// and draws the dialog accordingly.
pub fn draw_show_info_dialog(
    frame: &mut Frame,
    app: &AppState,
    accent_style: Style,
    info: &FileInfo,
) {
    let widget_info = app.config().theme().info();
    let info_cfg = &app.config().display().info();
    let position = dialog_position_unified(info_cfg.position(), app, DialogPosition::BottomLeft);
    let border_type = app.config().display().border_shape().as_border_type();

    let mut lines = Vec::new();
    if info_cfg.name() {
        lines.push(format!("Name:      {}", info.name().to_string_lossy()));
    }
    if info_cfg.file_type() {
        lines.push(format!("Type:      {}", format_file_type(info.file_type())));
    }
    if info_cfg.size() {
        lines.push(format!(
            "Size:      {}",
            format_file_size(*info.size(), info.file_type() == &FileType::Directory)
        ));
    }
    if info_cfg.modified() {
        lines.push(format!("Modified:  {}", format_file_time(*info.modified())));
    }
    if info_cfg.perms() {
        lines.push(format!("Perms:     {}", info.attributes()));
    }
    if lines.is_empty() {
        return;
    }

    let content_width = lines.iter().map(|s| s.chars().count()).max().unwrap_or(0);
    let min_width = 27;
    let border_pad = 2;
    let right_pad = 2;
    let width = (content_width + right_pad).max(min_width) + border_pad;
    let area = frame.area();

    // Clamp to frame area
    let width = width.min(area.width as usize);
    let height = (lines.len() + border_pad).min(area.height as usize);

    // Convert dialog size percantes to cell positions.
    let w_pct = ((width as f32 / area.width as f32) * 100.0).ceil() as u16;
    let h_pct = ((height as f32 / area.height as f32) * 100.0).ceil() as u16;
    let dialog_size = DialogSize::Custom(w_pct, h_pct);

    let dialog_style = DialogStyle {
        border: Borders::ALL,
        border_style: widget_info.border_style_or(accent_style),
        bg: widget_info.bg_or(Style::default().bg(ratatui::style::Color::Reset)),
        fg: widget_info.fg_or(Style::default().fg(ratatui::style::Color::Reset)),
        title: Some(Span::styled(
            " File Info ",
            widget_info.title_style_or(Style::default()),
        )),
    };

    let dialog_layout = DialogLayout {
        area,
        position,
        size: dialog_size,
    };

    draw_dialog(
        frame,
        dialog_layout,
        border_type,
        &dialog_style,
        lines.join("\n"),
        Some(Alignment::Left),
    );
}

/// Draws the fuzzy find dialog widget
///
/// Draws the input field and the result field as one widget.
///
/// Scrolls the entries of the find result. (max_visible = 15).
pub fn draw_find_dialog(frame: &mut Frame, app: &AppState, accent_style: Style) {
    let actions = app.actions();
    let widget = app.config().theme().widget();
    let position = dialog_position_unified(widget.position(), app, DialogPosition::Center);
    let size = widget.find_size_or(DialogSize::Medium);
    let border_type = app.config().display().border_shape().as_border_type();

    let input_text = actions.input_buffer();
    let results = actions.find_results();
    let selected = actions.find_selected();
    let area = frame.area();
    let dialog_rect = dialog_area(area, size, position);

    let max_visible = dialog_rect.height.saturating_sub(4).max(1) as usize;
    let total = results.len();
    let selected = selected.min(total.saturating_sub(1));
    let mut scroll = 0;

    if selected < scroll {
        scroll = selected;
    } else if selected >= scroll + max_visible {
        scroll = selected + 1 - max_visible;
    }

    let mut display_lines = Vec::with_capacity(max_visible + 2);

    display_lines.push(Line::from(vec![Span::styled(
        input_text,
        Style::default().add_modifier(Modifier::BOLD),
    )]));
    display_lines.push(Line::from(""));

    if results.is_empty() {
        display_lines.push(Line::from(Span::styled(
            " No matches",
            Style::default().fg(Color::DarkGray),
        )));
        for _ in 1..max_visible {
            display_lines.push(Line::from(""));
        }
    } else {
        for (idx, r) in results.iter().enumerate().skip(scroll).take(max_visible) {
            let marker = if idx == selected { "› " } else { "  " };
            let marker_style = if idx == selected {
                accent_style
            } else {
                Style::default()
            };
            display_lines.push(Line::from(vec![
                Span::styled(marker, marker_style),
                Span::raw(r.relative()),
            ]));
        }
        let lines_drawn = results
            .iter()
            .enumerate()
            .skip(scroll)
            .take(max_visible)
            .count();
        for _ in lines_drawn..max_visible {
            display_lines.push(Line::from(""));
        }
    }

    let dialog_style = DialogStyle {
        border: Borders::ALL,
        border_style: widget.border_style_or(accent_style),
        bg: widget.bg_or(Style::default().bg(Color::Reset)),
        fg: widget.fg_or(Style::default().fg(Color::Reset)),
        title: Some(Span::styled(
            " Find ",
            widget.title_style_or(Style::default()),
        )),
    };

    draw_dialog(
        frame,
        DialogLayout {
            area,
            position,
            size,
        },
        border_type,
        &dialog_style,
        display_lines,
        Some(Alignment::Left),
    );

    let visible_text = &input_text[..app.actions().input_cursor_pos().min(input_text.len())];
    frame.set_cursor_position((
        dialog_rect.x + 1 + visible_text.width() as u16,
        dialog_rect.y + 1,
    ));
}

/// Helper function to make adjusted dialog positions for unified borders
/// Returns a dialog position adjusted for unified borders (app-wide title/status).
fn adjusted_dialog_position(pos: DialogPosition, is_unified: bool) -> DialogPosition {
    match (is_unified, pos) {
        (true, DialogPosition::TopRight) => DialogPosition::Custom(100, 3),
        (true, DialogPosition::TopLeft) => DialogPosition::Custom(0, 3),
        (true, DialogPosition::Custom(x, 0)) => DialogPosition::Custom(x, 3),
        _ => pos,
    }
}

/// Calculates the final position for a dialog, handling unified border nudging.
/// Wrapper function to be used by draw widget functions to calculate the positions.
fn dialog_position_unified(
    configured: &Option<DialogPosition>,
    app: &AppState,
    fallback: DialogPosition,
) -> DialogPosition {
    let display_cfg = app.config().display();
    let base = configured.unwrap_or(fallback);
    adjusted_dialog_position(base, display_cfg.is_unified())
}
