//! UI pane drawing module for runa.
//!
//! This module provides renderes/drawers for the parent, main and preview panes.
//! All layout and highlighting logic for items, cursor and file type coloring is handled here.
//!
//! Used internally by [ui::render]

use crate::app::{AppState, PreviewData};
use crate::file_manager::FileEntry;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};
use std::collections::HashSet;

pub struct PaneStyles {
    pub item: Style,
    pub dir: Style,
    pub selection: Style,
}

impl PaneStyles {
    pub fn get_style(&self, is_dir: bool, is_selected: bool) -> Style {
        let mut style = if is_dir && self.dir.fg != Some(Color::Reset) {
            self.dir
        } else {
            self.item
        };

        if is_selected {
            style = style.add_modifier(Modifier::BOLD);
            if let Some(bg) = self.selection.bg
                && bg != Color::Reset
            {
                style = style.bg(bg);
            }

            if let Some(fg) = self.selection.fg
                && fg != Color::Reset
            {
                style = style.fg(fg);
            }
        }
        style
    }
}

pub struct PaneContext<'a> {
    pub area: Rect,
    pub block: Block<'a>,
    pub accent_style: Style,
    pub styles: PaneStyles,
    pub highlight_symbol: &'a str,
    pub entry_padding: u8,
    pub padding_str: &'static str,
}

pub struct PreviewOptions {
    pub use_underline: bool,
    pub underline_match_text: bool,
    pub underline_style: Style,
}

/// Draws the main file list pane in the UI
///
/// Highlights selection, markers and directories and handles styling for items.
pub fn draw_main(frame: &mut Frame, app: &AppState, context: PaneContext) {
    let show_marker = app.config().display().dir_marker();
    let selected_idx = app.visible_selected();
    let markers = app.nav().markers();
    let marker_theme = app.config().theme().marker();
    let marker_icon = marker_theme.icon();
    let marker_pad = " ".repeat(unicode_width::UnicodeWidthStr::width(marker_icon));
    let entry_padding = context.entry_padding as usize;
    let current_dir = app.nav().current_dir();

    let padding_str = if entry_padding > 1 {
        " ".repeat(entry_padding - 1)
    } else {
        String::new()
    };

    let local_markers: HashSet<&std::ffi::OsStr> = if markers.is_empty() {
        HashSet::new()
    } else {
        markers
            .iter()
            .filter(|path| path.parent() == Some(current_dir))
            .map(|path| path.file_name().unwrap_or_default())
            .collect()
    };

    if !app.has_visible_entries() {
        let style = context.styles.item;
        let line = Line::from(vec![
            Span::raw(context.padding_str),
            Span::styled("[Empty]", style),
        ]);

        frame.render_widget(
            Paragraph::new(line).block(context.block.border_style(context.accent_style)),
            context.area,
        );
        return;
    }

    let items = app.nav().shown_entries().enumerate().map(|(idx, entry)| {
        let is_selected = Some(idx) == selected_idx;
        let is_marked = local_markers.contains(entry.name().as_os_str());

        let name_str = if entry.is_dir() && show_marker {
            entry.display_name()
        } else {
            entry.name_str()
        };

        let entry_style = context.styles.get_style(entry.is_dir(), is_selected);
        let mut spans = Vec::with_capacity(4);

        if entry_padding == 0 {
            spans.push(Span::raw(name_str));
        } else {
            let mut marker_style = marker_theme.color().as_style();
            if is_selected {
                marker_style = marker_style.bg(entry_style.bg.unwrap_or_default());
            }

            if is_marked {
                spans.push(Span::styled(marker_icon, marker_style));
            } else {
                spans.push(Span::styled(&marker_pad, marker_style));
            }

            if entry_padding > 1 {
                spans.push(Span::raw(&padding_str));
            }
            // File name
            spans.push(Span::raw(name_str));
        }

        let line = Line::from(spans);
        ListItem::new(line).style(entry_style)
    });

    let mut state = ratatui::widgets::ListState::default();
    if app.has_visible_entries() {
        state.select(selected_idx);
    }

    frame.render_stateful_widget(
        ratatui::widgets::List::new(items)
            .block(context.block.border_style(context.accent_style))
            .highlight_style(Style::default())
            .highlight_symbol(context.highlight_symbol)
            .scroll_padding(app.config().display().scroll_padding()),
        context.area,
        &mut state,
    );
}

/// Draws the preview pane, showing either the file content or directory listing
///
/// Also applies underline/selection styles and manages cursor position
pub fn draw_preview(
    frame: &mut Frame,
    context: PaneContext,
    preview: &PreviewData,
    selected_idx: Option<usize>,
    opts: PreviewOptions,
) {
    match preview {
        PreviewData::Empty => {
            frame.render_widget(Paragraph::new("").block(context.block), context.area);
        }

        PreviewData::File(lines) => {
            let text: Vec<Line> = lines.iter().map(|s| Line::from(s.as_str())).collect();

            frame.render_widget(
                Paragraph::new(text).block(context.block.border_style(context.accent_style)),
                context.area,
            );
        }

        PreviewData::Directory(entries) => {
            if entries.is_empty() {
                let style = context.styles.item;
                let line = Line::from(vec![Span::raw(context.padding_str), Span::raw("[Empty]")]);

                let items = vec![ListItem::new(line).style(style)];
                let mut state = ListState::default();
                frame.render_stateful_widget(
                    List::new(items)
                        .block(context.block.border_style(context.accent_style))
                        .highlight_style(Style::default())
                        .highlight_symbol(context.highlight_symbol),
                    context.area,
                    &mut state,
                );

                return;
            }

            let items = entries.iter().enumerate().map(|(idx, entry)| {
                let is_selected = Some(idx) == selected_idx;
                let mut style = context.styles.get_style(entry.is_dir(), is_selected);

                if is_selected && opts.use_underline {
                    style = style.add_modifier(Modifier::UNDERLINED);
                    if let Some(color) = opts.underline_style.fg {
                        style = style.underline_color(color);
                        if opts.underline_match_text {
                            style = style.fg(color);
                        }
                    }
                    if let Some(bg) = opts
                        .underline_style
                        .bg
                        .filter(|&color| color != Color::Reset)
                    {
                        style = style.bg(bg);
                    }
                }

                let line = Line::from(vec![
                    Span::raw(context.padding_str),
                    Span::raw(entry.display_name()),
                ]);
                ListItem::new(line).style(style)
            });

            let mut state = ListState::default();
            state.select(selected_idx.map(|idx| idx.min(entries.len().saturating_sub(1))));

            frame.render_stateful_widget(
                List::new(items)
                    .block(context.block.border_style(context.accent_style))
                    .highlight_style(Style::default())
                    .highlight_symbol(context.highlight_symbol),
                context.area,
                &mut state,
            );
        }
    }
}

/// Draws the parent directory of the current working directory.
pub fn draw_parent(
    frame: &mut Frame,
    context: PaneContext,
    entries: &[FileEntry],
    selected_idx: Option<usize>,
) {
    if entries.is_empty() {
        // No extra args: if it's empty, we assume/render root indicator
        let line = Line::from(vec![
            Span::raw(context.padding_str),
            Span::styled("/", context.styles.dir),
        ]);
        frame.render_widget(Paragraph::new(line).block(context.block), context.area);
        return;
    }

    let items = entries.iter().enumerate().map(|(idx, entry)| {
        let is_selected = Some(idx) == selected_idx;
        let style = context.styles.get_style(entry.is_dir(), is_selected);
        let line = Line::from(vec![
            Span::raw(context.padding_str),
            Span::raw(entry.display_name()),
        ]);
        ListItem::new(line).style(style)
    });

    let mut state = ListState::default();
    state.select(selected_idx.map(|idx| idx.min(entries.len().saturating_sub(1))));

    frame.render_stateful_widget(
        List::new(items)
            .block(context.block.border_style(context.accent_style))
            .highlight_style(Style::default())
            .highlight_symbol(context.highlight_symbol),
        context.area,
        &mut state,
    );
}
