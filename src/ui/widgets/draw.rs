//! Draw widget module which holds all the draw functions for the render to use.
//!
//! Relies on helpers and data structs from widgets::dialog
//!
//! All draw functions are then used by ui::rende] to then draw widgets such a input dialog,
//! which is used by file action functions like rename and more..

use crate::app::actions::{ActionMode, InputMode};
use crate::app::{AppState, Clipboard};
use crate::config::display::{StatusSegment, StatusTag};
use crate::config::input::InputKeys;
use crate::core::{metadata::FileMetadataCache, worker::Workers};
use crate::ui::widgets::{
    DialogLayout, DialogPosition, DialogSize, DialogStyle, StatusPosition, dialog_area, draw_dialog,
};
use crate::utils::path::clean_display_path;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use std::sync::atomic::Ordering;
use std::time::Duration;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(crate) fn draw_separator(frame: &mut Frame, area: Rect, style: Style, border_type: BorderType) {
    frame.render_widget(
        Block::default()
            .borders(Borders::LEFT)
            .border_style(style)
            .border_type(border_type),
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

        match mode {
            InputMode::ConfirmDelete { is_trash } => {
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

                let dialog_layout = DialogLayout {
                    area: frame.area(),
                    position,
                    size: confirm_size,
                };

                draw_dialog(
                    frame,
                    dialog_layout,
                    border_type,
                    &get_dialog_style(
                        app,
                        Style::default().fg(Color::Red),
                        "Confirm Delete",
                        Some(Style::default().fg(Color::Red)),
                    ),
                    dialog_text,
                    Some(Alignment::Left),
                    Some(app.actions().scroll()),
                );
            }

            InputMode::MoveFile => {
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
                                .map(|p| format!(
                                    "  - {}",
                                    clean_display_path(&p.to_string_lossy())
                                ))
                                .collect::<Vec<_>>()
                                .join("\n")
                        )
                    }
                } else {
                    String::new()
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
                    &get_dialog_style(app, accent_style, prompt, None),
                    dialog_text,
                    Some(Alignment::Left),
                    Some(app.actions().scroll()),
                );

                frame.set_cursor_position((
                    dialog_area.x + 1 + cursor_offset as u16,
                    dialog_area.y + 1,
                ));
            }

            InputMode::ConfirmOverwrite { is_dir, old, new } => {
                let confirm_size = widget.confirm_size_or(DialogSize::Large);

                let target_name = new
                    .as_ref()
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();

                let preview = if let Some(src) = old {
                    let src_name = src
                        .as_ref()
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    format!(
                        "Rename will overwrite target:\n  From: {}\n  To:   {}",
                        src_name, target_name
                    )
                } else {
                    let kind = if *is_dir { "directory" } else { "file" };
                    format!("This {kind} will be overwritten:\n  {}", target_name)
                };

                let dialog_area = dialog_area(frame.area(), confirm_size, position);
                let visible_width = dialog_area.width.saturating_sub(2) as usize;

                let mut dialog_lines = vec![
                    Line::raw(prompt),
                    Line::from(vec![Span::styled(
                        "─".repeat(visible_width),
                        widget.border_style_or(Style::default().fg(Color::Yellow)),
                    )]),
                ];

                if !preview.is_empty() {
                    for line in preview.lines() {
                        dialog_lines.push(Line::raw(line));
                    }
                }

                let dialog_text = Text::from(dialog_lines);

                let dialog_layout = DialogLayout {
                    area: frame.area(),
                    position,
                    size: confirm_size,
                };

                draw_dialog(
                    frame,
                    dialog_layout,
                    border_type,
                    &get_dialog_style(
                        app,
                        Style::default().fg(Color::Yellow),
                        "Confirm Overwrite",
                        Some(Style::default().fg(Color::Yellow)),
                    ),
                    dialog_text,
                    Some(Alignment::Left),
                    Some(app.actions().scroll()),
                );
            }

            _ => {
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
                    &get_dialog_style(app, accent_style, prompt, None),
                    display_input,
                    Some(Alignment::Left),
                    None,
                );

                frame.set_cursor_position((
                    dialog_area.x + 1 + cursor_offset as u16,
                    dialog_area.y + 1,
                ));
            }
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

    let mut left_spans = Vec::with_capacity(12);

    if position == StatusPosition::Footer
        && display_cfg.info().status_bar()
        && let Some(file_meta) = app.selected_metadata()
    {
        let info_theme = theme.info();
        let separator_style = theme.accent_style();

        let segments = display_cfg.info().segments();

        for segment in segments {
            match segment {
                StatusSegment::Literal(lit) => {
                    left_spans.push(Span::styled(lit, separator_style));
                }
                StatusSegment::Tag(tag) => match tag {
                    StatusTag::Perms => {
                        left_spans.push(Span::styled(file_meta.perms(), info_theme.perms_style()));
                    }
                    StatusTag::Size => {
                        let padded_size = format!("{:>8}", file_meta.size());
                        left_spans.push(Span::styled(padded_size, info_theme.size_style()));
                    }
                    StatusTag::Mtime => {
                        let modified = file_meta.modified();
                        left_spans.push(Span::styled(modified, info_theme.modified_style()));
                    }
                    StatusTag::Btime => {
                        let created = file_meta.created();
                        left_spans.push(Span::styled(created, info_theme.created_style()));
                    }
                    StatusTag::Atime => {
                        let accessed = file_meta.accessed();
                        left_spans.push(Span::styled(accessed, info_theme.accessed_style()));
                    }
                    StatusTag::Type => {
                        left_spans.push(Span::styled(
                            file_meta.file_type(),
                            info_theme.file_type_style(),
                        ));
                    }
                    #[cfg(unix)]
                    StatusTag::Owner => {
                        let owner = file_meta.owner();
                        left_spans.push(Span::styled(owner, info_theme.owner_style()));
                    }

                    #[cfg(unix)]
                    StatusTag::Group => {
                        let group = file_meta.group();
                        left_spans.push(Span::styled(group, info_theme.group_style()));
                    }
                },
            }
        }
    }

    let mut spans = Vec::with_capacity(21);
    let separator = Span::styled(" | ", base_style);
    let add_sep = |s: &mut Vec<Span>| {
        if !s.is_empty() {
            s.push(separator.clone());
        }
    };

    if status_cfg.tasks() == position {
        let queued_ops = workers.fileop_tx().len();
        let active_ops = workers.active().load(Ordering::Relaxed);
        let total_ops = queued_ops + active_ops;

        if total_ops > 0
            && let Some(start) = app.worker_time()
            && start.elapsed() >= Duration::from_millis(200)
        {
            if active_ops > 0 {
                let symbols: &[&str] = if use_icons {
                    &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
                } else {
                    &["-", "\\", "|", "/"]
                };

                let tick = start.elapsed().as_millis() / 100;
                let spinner = symbols[tick as usize % symbols.len()];

                spans.push(Span::styled("Tasks: ", base_style));
                spans.push(Span::styled(spinner, base_style));
                spans.push(Span::styled(" FileOp(", base_style));
                spans.push(Span::styled(total_ops.to_string(), base_style));
                spans.push(Span::styled(")", base_style));
            } else {
                spans.push(Span::styled("Tasks: FileOp(", base_style));
                spans.push(Span::styled(total_ops.to_string(), base_style));
                spans.push(Span::styled(")", base_style));
            }
        }
    }

    if status_cfg.clipboard() == position
        && let Some(clipboard_set) = &clipboard.entries
    {
        let count = clipboard_set.len();
        if count > 0 {
            add_sep(&mut spans);
            let icon = if use_icons { "󰆏 " } else { "" };
            let style = marker_theme.clipboard_style_or_theme();
            spans.push(Span::styled(icon, style));
            spans.push(Span::styled(count.to_string(), style));
            spans.push(Span::styled(" copied", style));
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
                spans.push(Span::styled(marker_count.to_string(), style));
                spans.push(Span::styled(" marked", style));
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

    if status_cfg.tabs() == position && !app.tab_line().is_empty() {
        add_sep(&mut spans);
        spans.extend(app.tab_line().iter().cloned());
    }

    if status_cfg.entry_count() == position {
        let total = app.nav().shown_entries_len();
        add_sep(&mut spans);
        let count_str = if total == 0 {
            "0/0".to_string()
        } else {
            let idx_text = app
                .visible_selected()
                .map(|idx| (idx + 1).to_string())
                .unwrap_or_else(|| "0".to_string());
            format!("{}/{}", idx_text, total)
        };
        spans.push(Span::styled(count_str, base_style));
    }

    if left_spans.is_empty() && spans.is_empty() {
        return;
    }

    let y = match position {
        StatusPosition::Header => area.y,
        StatusPosition::Footer => area.y + area.height - 1,
        _ => return,
    };

    let rect = Rect::new(area.x, y, area.width, 1);
    let block =
        ratatui::widgets::Block::default().padding(ratatui::widgets::Padding::horizontal(1));

    if position == StatusPosition::Footer && !left_spans.is_empty() {
        let inner = block.inner(rect);

        let chunks = Layout::horizontal([
            Constraint::Length(left_spans.iter().map(|s| s.width() as u16).sum()),
            Constraint::Min(0),
            Constraint::Length(spans.iter().map(|s| s.width() as u16).sum()),
        ])
        .split(inner);

        frame.render_widget(Paragraph::new(Line::from(left_spans)), chunks[0]);

        frame.render_widget(
            Paragraph::new(Line::from(spans)).alignment(Alignment::Right),
            chunks[2],
        );

        frame.render_widget(block, rect);
    } else {
        frame.render_widget(
            Paragraph::new(Line::from(spans))
                .alignment(Alignment::Right)
                .block(block),
            rect,
        );
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
pub(crate) fn draw_show_info_dialog(
    frame: &mut Frame,
    app: &AppState,
    accent_style: Style,
    meta_cache: &FileMetadataCache,
) {
    let theme = app.config().theme();
    let info_cfg = &app.config().display().info();

    let label_style = theme.widget().label_style_or_theme();
    let value_style = theme.widget().value_style_or_theme();

    let position = dialog_position_unified(info_cfg.position(), app, DialogPosition::BottomLeft);
    let border_type = app.config().display().border_shape().as_border_type();

    let mut lines: Vec<Line> = Vec::with_capacity(9);

    let mut add_line = |label: &str, value: &str| {
        if !value.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(format!("{:<11}", label), label_style),
                Span::styled(value.to_string(), value_style),
            ]));
        }
    };

    if info_cfg.name() {
        add_line("Name:", meta_cache.name());
    }
    if info_cfg.file_type() {
        add_line("Type:", meta_cache.file_type());
    }
    if info_cfg.size() {
        add_line("Size:", meta_cache.size());
    }
    if info_cfg.modified() {
        add_line("Modified:", meta_cache.modified());
    }
    if info_cfg.created() {
        add_line("Created:", meta_cache.created());
    }
    if info_cfg.accessed() {
        add_line("Accessed:", meta_cache.accessed());
    }
    if info_cfg.perms() {
        add_line("Perms:", meta_cache.perms());
    }

    #[cfg(unix)]
    {
        if info_cfg.owner() {
            add_line("Owner:", meta_cache.owner());
        }

        if info_cfg.group() {
            add_line("Group:", meta_cache.group());
        }
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

    let dialog_layout = DialogLayout {
        area,
        position,
        size: DialogSize::Custom(width, height),
    };

    draw_dialog(
        frame,
        dialog_layout,
        border_type,
        &get_dialog_style(app, accent_style, "File Info", None),
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
    let results = actions.find().results();
    let selected = actions.find().selected();
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

    let (display_input, cursor_x) = input_field_view(input_text, cursor_pos, max_input_width);

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

    draw_dialog(
        frame,
        DialogLayout {
            area,
            position,
            size,
        },
        border_type,
        &get_dialog_style(app, accent_style, "Find", None),
        display_lines,
        Some(Alignment::Left),
        None,
    );
    frame.set_cursor_position((dialog_rect.x + 1 + cursor_x as u16, dialog_rect.y + 1));
}

pub(crate) fn draw_prefix_help_overlay(frame: &mut Frame, app: &AppState, accent_style: Style) {
    let widget = app.config().theme().widget();
    let area = frame.area();
    let border_type = app.config().display().border_shape().as_border_type();
    let keys = app.config().keys();
    let go_to_top_keys = keys.go_to_top();
    let go_to_home_keys = keys.go_to_home();
    let go_to_path_keys = keys.go_to_path();

    let is_sort_prefix = app.actions().prefix_recognizer().is_sort_state();
    if is_sort_prefix {
        fn mk_line(items: &[(&[String], &str)], accent_style: Style) -> Line<'static> {
            let mut spans: Vec<Span<'static>> = Vec::with_capacity(items.len() * 4);
            for (i, (keys, desc)) in items.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::raw("    "));
                }
                let primary_key = keys.first().map(|s| s.as_str()).unwrap_or("?");
                spans.push(Span::styled(format!("[{}]", primary_key), accent_style));
                spans.push(Span::raw(" "));
                spans.push(Span::raw((*desc).to_string()));
            }
            Line::from(spans)
        }

        let row1: [(&[String], &str); 4] = [
            (keys.sort_by_name(), "Name"),
            (keys.sort_by_modified(), "Modified"),
            (keys.sort_by_created(), "Created"),
            (keys.sort_by_accessed(), "Accessed"),
        ];
        let row2: [(&[String], &str); 3] = [
            (keys.sort_by_size(), "Size"),
            (keys.sort_by_extension(), "Ext"),
            (keys.sort_by_natural(), "Natural"),
        ];

        let lines: Vec<Line<'static>> =
            vec![mk_line(&row1, accent_style), mk_line(&row2, accent_style)];

        let area = frame.area();
        let border_pad = 2usize;
        let inner_pad = 2usize;

        let max_line_w = lines.iter().map(|l| l.width()).max().unwrap_or(0);

        let min_width = 24usize;
        let max_width = (area.width as usize).saturating_sub(2).max(min_width);

        let width = (max_line_w + inner_pad + border_pad)
            .max(min_width)
            .min(max_width) as u16;

        let height = (lines.len() + border_pad).min(area.height as usize) as u16;

        draw_dialog(
            frame,
            DialogLayout {
                area,
                position: widget.go_to_help_position(),
                size: DialogSize::Custom(width, height),
            },
            border_type,
            &get_dialog_style(app, accent_style, "Sort", None),
            Text::from(lines),
            Some(Alignment::Center),
            None,
        );
        return;
    }

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

    let size = widget.go_to_help_size();
    let position = widget.go_to_help_position();

    draw_dialog(
        frame,
        DialogLayout {
            area,
            position,
            size,
        },
        border_type,
        &get_dialog_style(app, accent_style, "Go to", None),
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
        &get_dialog_style(app, accent_style, "Message", None),
        text,
        Some(Alignment::Left),
        None,
    );
}

struct HelpEntry {
    key: InputKeys,
    desc: &'static str,
}

struct HelpSection {
    name: &'static str,
    entries: &'static [HelpEntry],
}

#[rustfmt::skip]
const HELP_DATA: &[HelpSection] = &[
    HelpSection {
        name: "Navigation",
        entries: &[
            HelpEntry { key: InputKeys::GoUp, desc: "Move selection up" },
            HelpEntry { key: InputKeys::GoDown, desc: "Move selection down" },
            HelpEntry { key: InputKeys::GoParent, desc: "Go to parent directory" },
            HelpEntry { key: InputKeys::GoIntoDir, desc: "Enter directory" },
            HelpEntry { key: InputKeys::ToggleMarker, desc: "Toggle marker" },
            HelpEntry { key: InputKeys::ClearMarkers, desc: "Clear markers" },
            HelpEntry { key: InputKeys::ClearFilter, desc: "Clear filter" },
            HelpEntry { key: InputKeys::ClearAll, desc: "Clear all markers and filters" },
            HelpEntry { key: InputKeys::SelectAll, desc: "Select all entries in directory" },
            HelpEntry { key: InputKeys::GoToBottom, desc: "Go to bottom" },
            HelpEntry { key: InputKeys::ScrollUp, desc: "Scroll widget up" },
            HelpEntry { key: InputKeys::ScrollDown, desc: "Scroll widget down" },
        ],
    },
    HelpSection {
        name: "Tabs",
        entries: &[
            HelpEntry { key: InputKeys::TabNew, desc: "Create a new tab" },
            HelpEntry { key: InputKeys::TabClose, desc: "Close the selected tab" },
            HelpEntry { key: InputKeys::TabNext, desc: "Switch to next tab" },
            HelpEntry { key: InputKeys::TabPrev, desc: "Switch to previous tab" },
        ],
    },
    HelpSection {
        name: "File",
        entries: &[
            HelpEntry { key: InputKeys::OpenFile, desc: "Open file in editor" },
            HelpEntry { key: InputKeys::Copy, desc: "Copy/Yank selection" },
            HelpEntry { key: InputKeys::Paste, desc: "Paste" },
            HelpEntry { key: InputKeys::Rename, desc: "Rename" },
            HelpEntry { key: InputKeys::Create, desc: "Create file" },
            HelpEntry { key: InputKeys::CreateDirectory, desc: "Create directory" },
            HelpEntry { key: InputKeys::Delete, desc: "Delete / move to trash" },
            HelpEntry { key: InputKeys::AlternateDelete, desc: "Alternate delete mode" },
            HelpEntry { key: InputKeys::Filter, desc: "Filter entries" },
            HelpEntry { key: InputKeys::Find, desc: "Find (fuzzy)" },
            HelpEntry { key: InputKeys::MoveFile, desc: "Move file(s)" },
            HelpEntry { key: InputKeys::ShowInfo, desc: "Toggle file info" },
            HelpEntry { key: InputKeys::ClearClipboard, desc: "Clear copied entries" },
        ],
    },
    HelpSection {
        name: "Go To",
        entries: &[
            HelpEntry { key: InputKeys::PrefixGoTo, desc: "Go to prefix" },
            HelpEntry { key: InputKeys::GoToTop, desc: "Go to top" },
            HelpEntry { key: InputKeys::GoToHome, desc: "Go to home" },
            HelpEntry { key: InputKeys::GoToPath, desc: "Go to path" },
        ],
    },
    HelpSection {
        name: "Sort",
        entries: &[
            HelpEntry { key: InputKeys::Sort, desc: "Sort prefix" },
            HelpEntry { key: InputKeys::SortByName, desc: "Sort by name" },
            HelpEntry { key: InputKeys::SortByNatural, desc: "Sort by natural order" },
            HelpEntry { key: InputKeys::SortByExtension, desc: "Sort by extension" },
            HelpEntry { key: InputKeys::SortBySize, desc: "Sort by size" },
            HelpEntry { key: InputKeys::SortByModified, desc: "Sort by modified time" },
            HelpEntry { key: InputKeys::SortByCreated, desc: "Sort by created time" },
            HelpEntry { key: InputKeys::SortByAccessed, desc: "Sort by accessed time" },
        ],
    },
    HelpSection {
        name: "System",
        entries: &[
            HelpEntry { key: InputKeys::Quit, desc: "Quit" },
            HelpEntry { key: InputKeys::KeybindHelp, desc: "Toggle keybind help" },
        ],
    },
];

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

    let go_to_prefix = keys.prefix_go_to();
    let sort_prefix = keys.sort();
    let go_leader = go_to_prefix.first().map(|s| s.as_str()).unwrap_or("g");
    let sort_leader = sort_prefix.first().map(|s| s.as_str()).unwrap_or("o");

    let fmt_prefix =
        |leader: &str, list: &[String]| -> String { format!("{leader} + {}", fmt_keys(list)) };

    let get_keys = |k: &InputKeys| -> &[String] {
        match k {
            InputKeys::GoUp => keys.go_up(),
            InputKeys::GoDown => keys.go_down(),
            InputKeys::GoParent => keys.go_parent(),
            InputKeys::GoIntoDir => keys.go_into_dir(),
            InputKeys::ToggleMarker => keys.toggle_marker(),
            InputKeys::ClearMarkers => keys.clear_markers(),
            InputKeys::ClearFilter => keys.clear_filter(),
            InputKeys::ClearAll => keys.clear_all(),
            InputKeys::SelectAll => keys.select_all(),
            InputKeys::GoToBottom => keys.go_to_bottom(),
            InputKeys::ScrollUp => keys.scroll_up(),
            InputKeys::ScrollDown => keys.scroll_down(),
            InputKeys::TabNew => keys.tab_new(),
            InputKeys::TabClose => keys.tab_close(),
            InputKeys::TabNext => keys.tab_next(),
            InputKeys::TabPrev => keys.tab_prev(),
            InputKeys::OpenFile => keys.open_file(),
            InputKeys::Copy => keys.copy(),
            InputKeys::Paste => keys.paste(),
            InputKeys::Rename => keys.rename(),
            InputKeys::Create => keys.create(),
            InputKeys::CreateDirectory => keys.create_directory(),
            InputKeys::Delete => keys.delete(),
            InputKeys::AlternateDelete => keys.alternate_delete(),
            InputKeys::Filter => keys.filter(),
            InputKeys::Find => keys.find(),
            InputKeys::MoveFile => keys.move_file(),
            InputKeys::ShowInfo => keys.show_info(),
            InputKeys::ClearClipboard => keys.clear_clipboard(),
            InputKeys::PrefixGoTo => keys.prefix_go_to(),
            InputKeys::GoToTop => keys.go_to_top(),
            InputKeys::GoToHome => keys.go_to_home(),
            InputKeys::GoToPath => keys.go_to_path(),
            InputKeys::Sort => keys.sort(),
            InputKeys::SortByName => keys.sort_by_name(),
            InputKeys::SortByNatural => keys.sort_by_natural(),
            InputKeys::SortByExtension => keys.sort_by_extension(),
            InputKeys::SortBySize => keys.sort_by_size(),
            InputKeys::SortByModified => keys.sort_by_modified(),
            InputKeys::SortByCreated => keys.sort_by_created(),
            InputKeys::SortByAccessed => keys.sort_by_accessed(),
            InputKeys::Quit => keys.quit(),
            InputKeys::KeybindHelp => keys.keybind_help(),
        }
    };

    let sections: Vec<(&str, Vec<(String, &'static str)>)> = HELP_DATA
        .iter()
        .map(|section| {
            let mut rows: Vec<(String, &'static str)> = section
                .entries
                .iter()
                .map(|entry| {
                    let key_text = match section.name {
                        "Go To" if entry.key != InputKeys::PrefixGoTo => {
                            fmt_prefix(go_leader, get_keys(&entry.key))
                        }
                        "Sort" if entry.key != InputKeys::Sort => {
                            fmt_prefix(sort_leader, get_keys(&entry.key))
                        }
                        _ => fmt_keys(get_keys(&entry.key)),
                    };
                    (key_text, entry.desc)
                })
                .collect();

            if section.name == "Tabs" {
                rows.push(("[0-9]".into(), "Switch to tab by index"));
            }
            (section.name, rows)
        })
        .collect();

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

    draw_dialog(
        frame,
        DialogLayout {
            area,
            position,
            size: DialogSize::Custom(dynamic_width, dynamic_height),
        },
        border_type,
        &get_dialog_style(app, accent_style, "Keybinds", None),
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

fn get_dialog_style(
    app: &AppState,
    accent: Style,
    title: &str,
    title_style_override: Option<Style>,
) -> DialogStyle {
    let widget_cfg = app.config().theme().widget();
    let title_style = title_style_override.unwrap_or_else(|| widget_cfg.title_style_or_theme());
    DialogStyle {
        border: Borders::ALL,
        border_style: widget_cfg.border_style_or(accent),
        bg: widget_cfg.bg_or_theme(),
        fg: widget_cfg.fg_or_theme(),
        title: Some(Span::styled(format!(" {title} "), title_style)),
        ..Default::default()
    }
}
