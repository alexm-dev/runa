//! Draw widget module which holds all the draw functions for the render to use.
//!
//! Relies on helpers and data structs from widgets::dialog
//!
//! All draw functions are then used by ui::rende] to then draw widgets such a input dialog,
//! which is used by file action functions like rename and more..

use crate::app::AppState;
use crate::app::actions::{ActionMode, InputMode};
use crate::config::display::EntryCountPosition;
use crate::core::formatter::{format_file_size, format_file_time, format_file_type};
use crate::core::{FileInfo, FileType};
use crate::ui::widgets::{
    DialogLayout, DialogPosition, DialogSize, DialogStyle, dialog_area, draw_dialog,
};
use crate::utils::clean_display_path;

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
pub(crate) fn draw_status_line(frame: &mut Frame, app: &AppState) {
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

    if app.config().display().entry_count() == EntryCountPosition::Header {
        let total = app.nav().shown_entries_len();
        if total == 0 {
            parts.push("0/0".to_string());
        } else {
            let idx_text = app
                .visible_selected()
                .map(|idx| (idx + 1).to_string())
                .unwrap_or_else(|| "0".to_string());

            parts.push(format!("{}/{}", idx_text, total));
        }
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

pub(crate) fn draw_footer_line(frame: &mut Frame, app: &AppState) {
    if app.config().display().entry_count() != EntryCountPosition::Footer {
        return;
    }

    let total = app.nav().shown_entries_len();

    let msg = if total == 0 {
        "0/0".to_string()
    } else {
        let idx = app.visible_selected().map(|i| i + 1).unwrap_or(0);
        format!("{}/{}", idx, total)
    };

    let area = frame.area();
    let rect = Rect {
        x: area.x,
        y: area.y + area.height - 1,
        width: area.width,
        height: 1,
    };

    let style = app.config().theme().status_line_style();
    let para = Paragraph::new(Line::from(Span::styled(msg, style)))
        .alignment(Alignment::Right)
        .block(
            ratatui::widgets::Block::default().padding(ratatui::widgets::Padding::horizontal(1)),
        );

    frame.render_widget(para, rect);
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
    );
}

pub(crate) fn draw_keybind_help(frame: &mut Frame, app: &AppState, accent_style: Style) {
    let keys = app.config().keys();
    let widget = app.config().theme().widget();
    let area = frame.area();

    let position = dialog_position_unified(widget.position(), app, DialogPosition::Center);

    let size = DialogSize::Custom(
        area.width.saturating_sub(6).clamp(40, 90),
        area.height.saturating_sub(6).clamp(12, 28),
    );

    let border_type = app.config().display().border_shape().as_border_type();

    let fmt_keys = |list: &[String]| -> String {
        if list.is_empty() {
            "—".to_string()
        } else {
            list.join(", ")
        }
    };

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
                (fmt_keys(keys.go_to_top()), "g … Go to top"),
                (fmt_keys(keys.go_to_home()), "g … Go to home"),
                (fmt_keys(keys.go_to_path()), "g … Go to path"),
            ],
        ),
    ];

    let mut all_rows: Vec<Line> = Vec::new();

    let header_style = Style::default().add_modifier(Modifier::BOLD);
    let key_style = accent_style.add_modifier(Modifier::BOLD);

    let dialog_rect = dialog_area(area, size, position);
    let inner_width = dialog_rect.width.saturating_sub(2) as usize;
    let two_col = inner_width >= 70;

    let col_gap = 4;
    let col_width = if two_col {
        (inner_width.saturating_sub(col_gap)) / 2
    } else {
        inner_width
    };

    for (section_name, rows) in sections {
        all_rows.push(Line::from(Span::styled(
            format!("{section_name}:"),
            header_style,
        )));

        let mut rendered: Vec<String> = rows
            .into_iter()
            .map(|(k, desc)| format!("{:<18}  {}", k, desc))
            .collect();

        if two_col {
            let left_count = rendered.len().div_ceil(2);
            let (left, right) = rendered.split_at(left_count);

            for i in 0..left_count {
                let l = left.get(i).cloned().unwrap_or_default();
                let r = right.get(i).cloned().unwrap_or_default();

                let l = if l.len() > col_width {
                    format!("{}…", &l[..col_width.saturating_sub(1)])
                } else {
                    l
                };
                let r = if r.len() > col_width {
                    format!("{}…", &r[..col_width.saturating_sub(1)])
                } else {
                    r
                };

                let pad = col_width.saturating_sub(l.len());
                let combined = if r.is_empty() {
                    l
                } else {
                    let mut s = String::with_capacity(col_width + col_gap + r.len());
                    s.push_str(&l);
                    s.push_str(&" ".repeat(pad));
                    s.push_str(&" ".repeat(col_gap));
                    s.push_str(&r);
                    s
                };

                let (kpart, rest) = combined.split_once("  ").unwrap_or((combined.as_str(), ""));
                all_rows.push(Line::from(vec![
                    Span::styled(kpart.to_string(), key_style),
                    Span::raw("  "),
                    Span::raw(rest.to_string()),
                ]));
            }
        } else {
            for s in rendered.drain(..) {
                let (kpart, rest) = s.split_once("  ").unwrap_or((s.as_str(), ""));
                all_rows.push(Line::from(vec![
                    Span::styled(kpart.to_string(), key_style),
                    Span::raw("  "),
                    Span::raw(rest.to_string()),
                ]));
            }
        }

        all_rows.push(Line::raw(""));
    }

    while matches!(all_rows.last(), Some(l) if l.width() == 0) {
        all_rows.pop();
    }

    let dialog_style = DialogStyle {
        border: Borders::ALL,
        border_style: widget.border_style_or(accent_style),
        bg: widget.bg_or_theme(),
        fg: widget.fg_or_theme(),
        title: Some(Span::styled(" Keybinds ", widget.title_style_or_theme())),
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
        Text::from(all_rows),
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
