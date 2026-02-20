//! Draw widget module which holds all the draw functions for the render to use.
//!
//! Relies on helpers and data structs from widgets::dialog
//!
//! All draw functions are then used by ui::rende] to then draw widgets such a input dialog,
//! which is used by file action functions like rename and more..

use crate::app::actions::{ActionMode, InputMode};
use crate::app::{AppState, Clipboard};
use crate::core::formatter::{format_file_size, format_file_time, format_file_type};
use crate::core::worker::Workers;
use crate::core::{FileInfo, FileType};
use crate::ui::widgets::{
    DialogLayout, DialogPosition, DialogSize, DialogStyle, StatusPosition, dialog_area, draw_dialog,
};
use crate::utils::clean_display_path;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Draws the seperator line when enabled inside runa.toml
pub(crate) fn draw_separator(frame: &mut Frame, area: Rect, style: Style) {
    frame.render_widget(
        Block::default().borders(Borders::LEFT).border_style(style),
        area,
    );
}

/// Either for ConfirmDelete or for anything else that requires input.
/// For other than ConfirmDelete, calculates the exact input field.
pub(crate) fn draw_input_dialog(frame: &mut Frame, app: &AppState, accent_style: Style) {
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
                ..Default::default()
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
                Some(app.actions().scroll()),
            );
        } else if *mode == InputMode::MoveFile {
            let targets_set = app.nav().get_action_targets();
            let mut action_targets: Vec<_> = targets_set.iter().collect();
            action_targets.sort();

            let preview = if !action_targets.is_empty() {
                if action_targets.len() == 1 {
                    format!(
                        "File to move: {}",
                        clean_display_path(&action_targets[0].to_string_lossy())
                    )
                } else {
                    format!(
                        "Files to move ({}):\n{}",
                        action_targets.len(),
                        action_targets
                            .iter()
                            .map(|p| format!("  - {}", clean_display_path(&p.to_string_lossy())))
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
                ..Default::default()
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
                Some(app.actions().scroll()),
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
                ..Default::default()
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
                None,
            );

            frame
                .set_cursor_position((dialog_area.x + 1 + cursor_offset as u16, dialog_area.y + 1));
        }
    }
}

/// Draw the status bar at the top or bottom of the screen, depending on the configuration.
/// Displays information such as active tasks, clipboard count, markers, filter, and entry count.
/// The content and position of the status bar are determined by runa.toml and the current state of the application.
pub(crate) fn draw_status_bar(
    frame: &mut Frame,
    app: &AppState,
    position: StatusPosition,
    workers: &Workers,
    clipboard: &Clipboard,
) {
    if position == StatusPosition::None {
        return;
    }

    let area = frame.area();
    let display_cfg = app.config().display();
    let status_cfg = display_cfg.status();
    let theme = app.config().theme();
    let base_style = theme.status_line_style();
    let marker_theme = theme.marker();
    let use_icons = display_cfg.icons();

    let mut spans = Vec::with_capacity(10);
    let separator = Span::styled(" | ", base_style);

    let add_sep = |s: &mut Vec<Span>| {
        if !s.is_empty() {
            s.push(separator.clone());
        }
    };

    if status_cfg.tabs() == position && !app.tab_line().is_empty() {
        spans.extend(app.tab_line().iter().cloned());
    }

    if status_cfg.tasks() == position {
        let queued_ops = workers.fileop_tx().len();
        let active_ops = workers.active().load(Ordering::Relaxed);
        let total_ops = queued_ops + active_ops;

        if total_ops > 0
            && let Some(start) = app.worker_time()
            && start.elapsed() >= Duration::from_millis(200)
        {
            let task_msg = if active_ops > 0 {
                let symbols: &[&str] = if use_icons {
                    &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
                } else {
                    &["-", "\\", "|", "/"]
                };
                let tick = start.elapsed().as_millis() / 100;
                let spinner = symbols[tick as usize % symbols.len()];
                format!("Tasks: {} FileOp({})", spinner, total_ops)
            } else {
                format!("Tasks: FileOp({})", total_ops)
            };
            spans.push(Span::styled(task_msg, base_style));
        }
    }

    if status_cfg.clipboard() == position
        && let Some(clipboard_set) = &clipboard.entries
    {
        let count = clipboard_set.len();
        let now = Instant::now();

        if count > 0 && app.notification_time().is_some_and(|until| until > now) {
            add_sep(&mut spans);
            let icon = if use_icons { "󰆏 " } else { "" };
            let label = "copied";
            let style = marker_theme.clipboard_style_or_theme();
            spans.push(Span::styled(format!("{}{} {}", icon, count, label), style));
        }
    }

    if status_cfg.markers() == position {
        let markers = app.nav().markers();
        let marker_count = markers.len();

        if marker_count > 0 {
            let is_redundant = if let Some(clipboard_set) = &clipboard.entries {
                if clipboard_set.len() != marker_count {
                    false
                } else {
                    markers.iter().all(|path| clipboard_set.contains(path))
                }
            } else {
                false
            };

            if !is_redundant {
                add_sep(&mut spans);
                let style = marker_theme.style_or_theme();
                spans.push(Span::styled(format!("{} marked", marker_count), style));
            }
        }
    }

    if status_cfg.filter() == position {
        let filter = app.nav().filter();
        if !filter.is_empty() {
            add_sep(&mut spans);
            spans.push(Span::styled(format!("Filter: \"{}\"", filter), base_style));
        }
    }

    if status_cfg.entry_count() == position {
        let total = app.nav().shown_entries_len();
        if total == 0 {
            add_sep(&mut spans);
            spans.push(Span::styled("0/0", base_style));
        } else {
            add_sep(&mut spans);
            let idx_text = app
                .visible_selected()
                .map(|idx| (idx + 1).to_string())
                .unwrap_or_else(|| "0".to_string());
            spans.push(Span::styled(format!("{}/{}", idx_text, total), base_style));
        }
    }

    if spans.is_empty() {
        return;
    }

    let y = match position {
        StatusPosition::Header => area.y,
        StatusPosition::Footer => area.y + area.height - 1,
        StatusPosition::None => return,
    };

    let rect = Rect {
        x: area.x,
        y,
        width: area.width,
        height: 1,
    };

    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .alignment(Alignment::Right)
            .block(
                ratatui::widgets::Block::default()
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            ),
        rect,
    );
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
pub(crate) fn draw_show_info_dialog(
    frame: &mut Frame,
    app: &AppState,
    accent_style: Style,
    info: &FileInfo,
) {
    let theme = app.config().theme();
    let widget_info = theme.info();
    let info_cfg = &app.config().display().info();

    let label_style = theme.widget().label_style_or_theme();
    let value_style = theme.widget().value_style_or_theme();

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
        ..Default::default()
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
        None,
    );
}

/// Draws the fuzzy find dialog widget
///
/// Draws the input field and the result field as one widget.
/// Sets a find result indicator in the input line to the right.
/// Find result indicator being on the input line makes the actual input line smaller.
pub(crate) fn draw_find_dialog(frame: &mut Frame, app: &AppState, accent_style: Style) {
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
        ..Default::default()
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
        None,
    );
    frame.set_cursor_position((dialog_rect.x + 1 + cursor_x as u16, dialog_rect.y + 1));
}

pub(crate) fn draw_prefix_help_overlay(frame: &mut Frame, app: &AppState, accent_style: Style) {
    let keys = app.config().keys();
    let go_to_top_keys = keys.go_to_top();
    let go_to_home_keys = keys.go_to_home();
    let go_to_path_keys = keys.go_to_path();

    let mut g_prefixes: Vec<(String, &'static str)> =
        Vec::with_capacity(go_to_top_keys.len() + go_to_path_keys.len());
    if let Some(k) = go_to_top_keys.first() {
        g_prefixes.push((k.clone(), "Go to top"));
    }
    if let Some(k) = go_to_home_keys.first() {
        g_prefixes.push((k.clone(), "Go to home"));
    }
    if let Some(k) = go_to_path_keys.first() {
        g_prefixes.push((k.clone(), "Go to path"));
    }

    let mut spans = Vec::with_capacity(4 * g_prefixes.len() + 2);
    for (i, (ch, desc)) in g_prefixes.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("    "));
        }
        spans.push(Span::styled(format!("[{}]", ch), accent_style));
        spans.push(Span::raw(" "));
        spans.push(Span::raw(*desc));
    }

    spans.push(Span::raw(" "));
    let line = Line::from(spans);

    let widget = app.config().theme().widget();
    let area = frame.area();

    let size = widget.go_to_help_size();
    let position = widget.go_to_help_position();

    let border_type = app.config().display().border_shape().as_border_type();
    let dialog_style = DialogStyle {
        border: Borders::ALL,
        border_style: widget.border_style_or(accent_style),
        bg: widget.bg_or_theme(),
        fg: widget.fg_or_theme(),
        title: Some(Span::styled("Go to", widget.title_style_or_theme())),
        ..Default::default()
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
        line,
        Some(Alignment::Center),
        None,
    );
}

/// Draws a simple message overlay dialog at the bottom right
/// Used for notifications such as "fd is not available" etc.
pub(crate) fn draw_message_overlay(
    frame: &mut Frame,
    app: &AppState,
    accent_style: Style,
    text: &str,
) {
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
        ..Default::default()
    };

    let mut dialog_rect = dialog_area(area, dialog_size, position);

    if dialog_rect.y + dialog_rect.height >= area.y + area.height && dialog_rect.y > area.y {
        dialog_rect.y -= 1;
    }

    let custom_layout = DialogLayout {
        area: dialog_rect,
        position: DialogPosition::BottomLeft,
        size: dialog_size,
    };

    draw_dialog(
        frame,
        custom_layout,
        border_type,
        &dialog_style,
        text,
        Some(Alignment::Left),
        None,
    );
}

pub(crate) fn draw_keybind_help(frame: &mut Frame, app: &AppState, accent_style: Style) {
    let keys = app.config().keys();
    let widget = app.config().theme().widget();
    let area = frame.area();

    let position = dialog_position_unified(widget.position(), app, DialogPosition::Center);

    let border_type = app.config().display().border_shape().as_border_type();
    let dim_style = Style::default().add_modifier(Modifier::DIM);
    let header_style = app.config().theme().widget().label_style_or_theme();
    let key_style = app.config().theme().widget().value_style_or_theme();

    let max_keys_to_show: usize = 3;
    let key_pad: usize = 3;

    let fmt_key_token = |k: &str| -> String {
        if k == " " || k.trim().is_empty() || k.eq_ignore_ascii_case("space") {
            "Space".to_string()
        } else {
            k.to_string()
        }
    };

    let fmt_keys = |list: &[String]| -> String {
        if list.is_empty() {
            return "-".to_string();
        }
        let mut out: Vec<String> = list
            .iter()
            .take(max_keys_to_show)
            .map(|k| fmt_key_token(k))
            .collect();
        out.reverse();
        if list.len() > max_keys_to_show {
            out.push("...".to_string());
        }
        out.join(", ")
    };

    let fmt_prefix =
        |leader: &str, list: &[String]| -> String { format!("{leader} + {}", fmt_keys(list)) };

    let sections: Vec<(&str, Vec<(String, &'static str)>)> = vec![
        (
            "Navigation",
            vec![
                (fmt_keys(keys.go_up()), "Move selection up"),
                (fmt_keys(keys.go_down()), "Move selection down"),
                (fmt_keys(keys.go_parent()), "Go to parent directory"),
                (fmt_keys(keys.go_into_dir()), "Enter directory"),
                (fmt_keys(keys.toggle_marker()), "Toggle marker"),
                (fmt_keys(keys.clear_markers()), "Clear markers"),
                (fmt_keys(keys.clear_filter()), "Clear filter"),
                (fmt_keys(keys.clear_all()), "Clear all markers and filters"),
                (fmt_keys(keys.go_to_bottom()), "Go to bottom"),
            ],
        ),
        (
            "Tabs",
            vec![
                (fmt_keys(keys.tab_new()), "Create a new tab"),
                (fmt_keys(keys.tab_close()), "Close the selected tab"),
                (fmt_keys(keys.tab_cycle()), "Cycle between tabs"),
                (fmt_keys(keys.tab_next()), "Switch to the next tab"),
                (fmt_keys(keys.tab_prev()), "Switch to the previous tab"),
            ],
        ),
        (
            "File",
            vec![
                (fmt_keys(keys.open_file()), "Open file in editor"),
                (fmt_keys(keys.copy()), "Copy/Yank selection"),
                (fmt_keys(keys.paste()), "Paste"),
                (fmt_keys(keys.rename()), "Rename"),
                (fmt_keys(keys.create()), "Create file"),
                (fmt_keys(keys.create_directory()), "Create directory"),
                (fmt_keys(keys.delete()), "Delete / move to trash"),
                (fmt_keys(keys.alternate_delete()), "Alternate delete mode"),
                (fmt_keys(keys.filter()), "Filter entries"),
                (fmt_keys(keys.find()), "Find (fuzzy)"),
                (fmt_keys(keys.move_file()), "Move file(s)"),
                (fmt_keys(keys.show_info()), "Toggle file info"),
                (fmt_keys(keys.clear_clipboard()), "Clear copied entries"),
            ],
        ),
        (
            "System",
            vec![
                (fmt_keys(keys.quit()), "Quit"),
                (fmt_keys(keys.keybind_help()), "Toggle keybind help"),
            ],
        ),
        (
            "Prefix",
            vec![
                (fmt_prefix("g", keys.go_to_top()), "Go to top"),
                (fmt_prefix("g", keys.go_to_home()), "Go to home"),
                (fmt_prefix("g", keys.go_to_path()), "Go to path"),
            ],
        ),
    ];

    let key_w: usize = sections
        .iter()
        .flat_map(|(_, rows)| rows.iter().map(|(k, _)| k.len()))
        .max()
        .unwrap_or(10)
        .clamp(10, 28);

    let margin = 6;
    let margin_spacer = " ".repeat(margin);

    let mk_line = |key_text: &str, desc: &str| -> Line<'static> {
        let key_string = format!("{:>width$}", key_text, width = key_w);

        Line::from(vec![
            Span::raw(margin_spacer.to_string()),
            Span::styled(key_string, key_style),
            Span::styled(" ".repeat(key_pad), dim_style),
            Span::raw(desc.to_string()),
        ])
    };

    let mut lines: Vec<Line> = Vec::new();
    let section_title_indent = " ".repeat(key_w + key_pad);

    for (section_name, rows) in sections {
        lines.push(Line::from(vec![
            Span::raw(margin_spacer.to_string()),
            Span::raw(&section_title_indent),
            Span::styled(format!("{section_name}:"), header_style),
        ]));

        for (k, desc) in rows {
            lines.push(mk_line(&k, desc));
        }
        lines.push(Line::raw(""));
    }
    while matches!(lines.last(), Some(l) if l.width() == 0) {
        lines.pop();
    }

    lines.push(Line::raw(""));

    let content_height = lines.len() as u16;

    let dynamic_width = (area.width * 60 / 100).clamp(45, 80);

    let max_allowed_height = (area.height * 80 / 100).max(12);
    let dynamic_height = (content_height + 1).min(max_allowed_height);

    let size = DialogSize::Custom(dynamic_width, dynamic_height);

    let dialog_style = DialogStyle {
        border: Borders::ALL,
        border_style: widget.border_style_or(accent_style),
        bg: widget.bg_or_theme(),
        fg: widget.fg_or_theme(),
        title: Some(Span::styled(" Keybinds ", widget.title_style_or_theme())),
        ..Default::default()
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
        Text::from(lines),
        Some(Alignment::Left),
        Some(app.actions().scroll()),
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
