//! Draw widget module which holds all the draw functions for the render to use.
//!
//! Relies on helpers and data structs from widgets::dialog
//!
//! All draw functions are then used by ui::rende] to then draw widgets such a input dialog,
//! which is used by file action functions like rename and more..

use crate::app::AppState;
use crate::app::actions::{ActionMode, InputMode};
use crate::core::{FileInfo, FileType, format_file_size, format_file_time, format_file_type};
use crate::ui::widgets::{
    DialogLayout, DialogPosition, DialogSize, DialogStyle, dialog_area, draw_dialog,
};
use crate::utils::readable_path;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
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
        let move_size = widget.move_size_or(DialogSize::Custom(70, 14));
        let border_type = app.config().display().border_shape().as_border_type();

        if let InputMode::ConfirmDelete { is_trash } = mode {
            let confirm_size = widget.confirm_size_or(DialogSize::Large);
            let mut targets: Vec<String> = app
                .nav()
                .get_action_targets()
                .into_iter()
                .map(|p| {
                    p.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                })
                .collect();
            targets.sort();

            let heading = if *is_trash {
                "Files to move to trash"
            } else {
                "Files to delete"
            };

            let preview = if !targets.is_empty() {
                format!(
                    "{heading} ({}):\n{}",
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

            let dialog_area = dialog_area(frame.area(), confirm_size, position);
            let visible_width = dialog_area.width.saturating_sub(2) as usize;

            let mut dialog_lines = vec![
                Line::raw(prompt),
                Line::from(vec![Span::styled(
                    "─".repeat(visible_width),
                    widget.border_style_or(Style::default().fg(Color::Red)),
                )]),
            ];
            if !preview.is_empty() {
                for line in preview.lines() {
                    dialog_lines.push(Line::raw(line));
                }
            }
            let dialog_text = Text::from(dialog_lines);

            let dialog_style = DialogStyle {
                border: Borders::ALL,
                border_style: widget.border_style_or(Style::default().fg(Color::Red)),
                bg: widget.bg_or_theme(),
                fg: widget.fg_or_theme(),
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
                dialog_text,
                Some(Alignment::Left),
            );
        } else if *mode == InputMode::MoveFile {
            let targets_set = app.nav().get_action_targets();
            let mut action_targets: Vec<_> = targets_set.iter().collect();
            action_targets.sort();

            let preview = if !action_targets.is_empty() {
                if action_targets.len() == 1 {
                    format!("File to move: {}", readable_path(action_targets[0]))
                } else {
                    format!(
                        "Files to move ({}):\n{}",
                        action_targets.len(),
                        action_targets
                            .iter()
                            .map(|p| format!("  - {}", readable_path(p)))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                }
            } else {
                String::new()
            };

            let dialog_style = DialogStyle {
                border: Borders::ALL,
                border_style: widget.border_style_or(accent_style),
                bg: widget.bg_or_theme(),
                fg: widget.fg_or_theme(),
                title: Some(Span::styled(
                    format!(" {} ", prompt),
                    widget.title_style_or_theme(),
                )),
            };

            let dialog_layout = DialogLayout {
                area: frame.area(),
                position,
                size: move_size,
            };

            let input_text = app.actions().input_buffer();
            let cursor_pos = app.actions().input_cursor_pos();
            let dialog_area = dialog_area(frame.area(), move_size, position);
            let visible_width = dialog_area.width.saturating_sub(2) as usize;

            let (display_input, cursor_offset) =
                input_field_view(input_text, cursor_pos, visible_width);

            let want_separator = move_size != DialogSize::Small && !preview.is_empty();
            let mut dialog_lines = vec![Line::raw(display_input)];
            if want_separator {
                dialog_lines.push(Line::from(vec![Span::styled(
                    "─".repeat(visible_width),
                    accent_style,
                )]));
            }
            if !preview.is_empty() {
                for lines in preview.lines() {
                    dialog_lines.push(Line::raw(lines));
                }
            }
            let dialog_text = Text::from(dialog_lines);

            draw_dialog(
                frame,
                dialog_layout,
                border_type,
                &dialog_style,
                dialog_text,
                Some(Alignment::Left),
            );

            frame
                .set_cursor_position((dialog_area.x + 1 + cursor_offset as u16, dialog_area.y + 1));
        } else {
            let dialog_style = DialogStyle {
                border: Borders::ALL,
                border_style: widget.border_style_or(accent_style),
                bg: widget.bg_or_theme(),
                fg: widget.fg_or_theme(),
                title: Some(Span::styled(
                    format!(" {} ", prompt),
                    widget.title_style_or_theme(),
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
pub fn draw_status_line(frame: &mut Frame, app: &AppState) {
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
        let style = app.config().theme().status_line_style();
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
    let theme = app.config().theme();
    let widget_info = theme.info();
    let info_cfg = &app.config().display().info();

    let label_style = theme.directory_style();
    let value_style = theme.entry_style();

    let position = dialog_position_unified(info_cfg.position(), app, DialogPosition::BottomLeft);
    let border_type = app.config().display().border_shape().as_border_type();

    let mut lines: Vec<Line> = Vec::with_capacity(5);

    let mut add_line = |label: &str, value: String| {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<11}", label), label_style),
            Span::styled(value, value_style),
        ]));
    };

    if info_cfg.name() {
        add_line("Name:", info.name().to_string_lossy().into_owned());
    }
    if info_cfg.file_type() {
        add_line("Type:", format_file_type(info.file_type()).into());
    }
    if info_cfg.size() {
        add_line(
            "Size:",
            format_file_size(*info.size(), info.file_type() == &FileType::Directory),
        );
    }
    if info_cfg.modified() {
        add_line("Modified:", format_file_time(*info.modified()));
    }
    if info_cfg.perms() {
        add_line("Perms:", info.attributes().to_string());
    }

    if lines.is_empty() {
        return;
    }

    let max_width = lines.iter().map(|l| l.width()).max().unwrap_or(0);

    let min_width = 27;
    let border_pad = 2;
    let right_pad = 2;
    let area = frame.area();

    let raw_width = (max_width + right_pad).max(min_width) + border_pad;
    let width = raw_width.min(area.width as usize) as u16;
    let height = (lines.len() + border_pad).min(area.height as usize) as u16;

    let dialog_style = DialogStyle {
        border: Borders::ALL,
        border_style: widget_info.border_style_or(accent_style),
        bg: widget_info.bg_or_theme(),
        fg: widget_info.fg_or_theme(),
        title: Some(Span::styled(
            " File Info ",
            widget_info.title_style_or_theme(),
        )),
    };

    let dialog_layout = DialogLayout {
        area,
        position,
        size: DialogSize::Custom(width, height),
    };

    draw_dialog(
        frame,
        dialog_layout,
        border_type,
        &dialog_style,
        Text::from(lines),
        Some(Alignment::Left),
    );
}

/// Draws the fuzzy find dialog widget
///
/// Draws the input field and the result field as one widget.
/// Sets a find result indicator in the input line to the right.
/// Find result indicator being on the input line makes the actual input line smaller.
pub fn draw_find_dialog(frame: &mut Frame, app: &AppState, accent_style: Style) {
    let actions = app.actions();
    let widget = app.config().theme().widget();
    let base_dir = app.nav().current_dir();
    let area = frame.area();

    let position = dialog_position_unified(widget.position(), app, DialogPosition::Center);
    let columns = widget
        .find_width_or(area.width.saturating_sub(8).clamp(20, 80))
        .min(area.width)
        .max(20);

    let max_visible = widget.find_visible_or(5);
    let rows = max_visible as u16 + 4;

    let size = DialogSize::Custom(columns, rows);
    let border_type = app.config().display().border_shape().as_border_type();

    let input_text = actions.input_buffer();
    let cursor_pos = actions.input_cursor_pos();
    let results = actions.find_results();
    let selected = actions.find_selected();
    let area = frame.area();
    let dialog_rect = dialog_area(area, size, position);

    let total = results.len();
    let selected = selected.min(total.saturating_sub(1));
    let mut scroll = 0;

    if selected < scroll {
        scroll = selected;
    } else if selected >= scroll + max_visible {
        scroll = selected + 1 - max_visible;
    }

    let mut display_lines = Vec::with_capacity(max_visible + 2);

    let indicator = format!(
        "[{} / {}]",
        if total == 0 { 0 } else { selected + 1 },
        total
    );
    let field_width = dialog_rect.width.saturating_sub(2) as usize;
    let indicator_width = indicator.width() + 2;
    let max_input_width = field_width.saturating_sub(indicator_width);

    let (display_input, cursor_x) = if input_text.width() <= max_input_width {
        (
            input_text.to_string(),
            input_text[..cursor_pos.min(input_text.len())].width(),
        )
    } else {
        let mut cur_width = 0;
        let mut start = input_text.len();
        for (idx, ch) in input_text.char_indices().rev() {
            cur_width += ch.width().unwrap_or(0);
            if cur_width > max_input_width {
                start = idx + ch.len_utf8();
                break;
            }
        }
        let display = input_text[start..].to_string();
        let cursor = if cursor_pos < start {
            0
        } else {
            input_text[start..cursor_pos.min(input_text.len())].width()
        };
        (display, cursor)
    };
    let pad_width = max_input_width.saturating_sub(display_input.width());
    let mut line_input = vec![Span::styled(
        display_input,
        Style::default().add_modifier(Modifier::BOLD),
    )];
    if pad_width > 0 {
        line_input.push(Span::raw(" ".repeat(pad_width)));
    }
    line_input.push(Span::raw("  "));
    line_input.push(Span::styled(
        indicator,
        Style::default().fg(Color::DarkGray),
    ));
    display_lines.push(Line::from(line_input));

    let horizontal_line = Span::styled("─".repeat(field_width), accent_style);
    display_lines.push(Line::from(horizontal_line));

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
                Span::raw(r.relative(base_dir)),
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
        bg: widget.bg_or_theme(),
        fg: widget.fg_or_theme(),
        title: Some(Span::styled(" Find ", widget.title_style_or_theme())),
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
    frame.set_cursor_position((dialog_rect.x + 1 + cursor_x as u16, dialog_rect.y + 1));
}

/// Draws a simple message overlay dialog at the bottom right
/// Used for notifications such as "fd is not available" etc.
pub fn draw_message_overlay(frame: &mut Frame, app: &AppState, accent_style: Style, text: &str) {
    let widget = app.config().theme().widget();
    let position = DialogPosition::BottomRight;
    let border_type = app.config().display().border_shape().as_border_type();

    let mut max_line_width = 0;
    let mut line_count = 0;
    for line in text.lines() {
        max_line_width = max_line_width.max(line.len());
        line_count += 1;
    }

    let min_width = 27;
    let border_pad = 2;
    let right_pad = 2;
    let area = frame.area();

    let width =
        ((max_line_width + right_pad).max(min_width) + border_pad).min(area.width as usize) as u16;
    let height = ((line_count + border_pad).min(area.height as usize)) as u16;

    let dialog_size = DialogSize::Custom(width, height);

    let dialog_style = DialogStyle {
        border: Borders::ALL,
        border_style: widget.border_style_or(accent_style),
        bg: widget.bg_or_theme(),
        fg: widget.fg_or_theme(),
        title: Some(Span::styled(" Message ", widget.title_style_or_theme())),
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
        text,
        Some(Alignment::Left),
    );
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
