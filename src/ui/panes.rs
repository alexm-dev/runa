//! UI pane drawing module for runa.
//!
//! This module provides renderes/drawers for the parent, main and preview panes.
//! All layout and highlighting logic for items, cursor and file type coloring is handled here.
//!
//! Used internally by ui::render

use crate::app::{AppState, Clipboard, PreviewData};
use crate::core::FileEntry;
use crate::ui::icons::nerd_font_icon;
use ratatui::widgets::BorderType;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Styles used for rendering items in a pane
/// Includes styles for regular items, directories and selected items
pub(crate) struct PaneStyles {
    pub(crate) item: Style,
    pub(crate) dir: Style,
    pub(crate) selection: Style,
    pub(crate) symlink_dir: Color,
    pub(crate) symlink_file: Color,
    pub(crate) symlink_target: Color,
    pub(crate) executable_fg: Color,
}

impl PaneStyles {
    pub(crate) fn get_style(&self, is_dir: bool, is_selected: bool) -> Style {
        let mut style = if is_dir && self.dir.fg != Some(Color::Reset) {
            self.dir
        } else {
            self.item
        };

        if is_selected {
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

/// Context data for pane rendering functions
pub(crate) struct PaneContext<'a> {
    pub(crate) area: Rect,
    pub(crate) block: Block<'a>,
    pub(crate) border_type: BorderType,
    pub(crate) accent_style: Style,
    pub(crate) styles: PaneStyles,
    pub(crate) highlight_symbol: &'a str,
    pub(crate) padding_str: &'static str,
    pub(crate) show_icons: bool,
    pub(crate) show_marker: bool,
}

/// Options for preview pane rendering
pub(crate) struct PreviewOptions {
    pub(crate) use_underline: bool,
    pub(crate) underline_match_text: bool,
    pub(crate) underline_style: Style,
}

/// Marker and clipboard data for use in pane drawing functions
pub(crate) struct PaneMarkers<'a> {
    pub(crate) markers: Option<HashSet<OsString>>,
    pub(crate) clipboard: Option<HashSet<OsString>>,
    pub(crate) marker_icon: &'a str,
    pub(crate) marker_style: Style,
    pub(crate) clipboard_style: Style,
}

/// Draws the main file list pane in the UI
///
/// Highlights selection, markers and directories and handles styling for items.
pub(crate) fn draw_main(
    frame: &mut Frame,
    app: &AppState,
    context: PaneContext,
    clipboard: &Clipboard,
) {
    let selected_idx = app.visible_selected();
    let current_dir = app.nav().current_dir();

    if !app.is_loading() && app.nav().shown_entries_len() == 0 && !app.nav().filter().is_empty() {
        let style = context.styles.item;
        let line = Line::from(vec![
            Span::raw(context.padding_str),
            Span::styled("[No results for this filter]", style),
        ]);
        frame.render_widget(Paragraph::new(line).block(context.block), context.area);
        return;
    }

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

    let markers = make_main_pane_markers(app, current_dir, clipboard);

    let shown_len = app.nav().shown_entries_len();
    let mut items = Vec::with_capacity(shown_len);
    for (idx, entry) in app.nav().shown_entries().enumerate() {
        let is_selected = Some(idx) == selected_idx;
        let entry_style = context.styles.get_style(entry.is_dir(), is_selected);
        items.push(make_entry_row(
            entry,
            is_selected,
            entry_style,
            &context,
            &markers,
            None,
        ));
    }

    let mut state = ListState::default();
    if app.has_visible_entries() {
        state.select(selected_idx);
    }

    frame.render_stateful_widget(
        List::new(items)
            .block(
                context
                    .block
                    .border_style(context.accent_style)
                    .border_type(context.border_type),
            )
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
pub(crate) fn draw_preview(
    frame: &mut Frame,
    context: PaneContext,
    preview: &PreviewData,
    selected_idx: Option<usize>,
    opts: PreviewOptions,
    markers: &PaneMarkers,
) {
    match preview {
        PreviewData::Empty => {
            frame.render_widget(
                Paragraph::new("").block(
                    context
                        .block
                        .border_style(context.accent_style)
                        .border_type(context.border_type),
                ),
                context.area,
            );
        }

        PreviewData::File(text) => {
            frame.render_widget(
                Paragraph::new(text.clone())
                    .block(context.block.border_style(context.accent_style)),
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

            let mut items = Vec::with_capacity(entries.len());
            for (idx, entry) in entries.iter().enumerate() {
                let is_selected = Some(idx) == selected_idx;
                let style = context.styles.get_style(entry.is_dir(), is_selected);
                items.push(make_entry_row(
                    entry,
                    is_selected,
                    style,
                    &context,
                    markers,
                    Some(&opts),
                ));
            }

            let mut state = ListState::default();
            state.select(selected_idx.map(|idx| idx.min(entries.len().saturating_sub(1))));

            frame.render_stateful_widget(
                List::new(items)
                    .block(
                        context
                            .block
                            .border_style(context.accent_style)
                            .border_type(context.border_type),
                    )
                    .highlight_style(Style::default())
                    .highlight_symbol(context.highlight_symbol),
                context.area,
                &mut state,
            );
        }
    }
}

/// Draws the parent directory of the current working directory.
pub(crate) fn draw_parent(
    frame: &mut Frame,
    context: PaneContext,
    entries: &[FileEntry],
    selected_idx: Option<usize>,
    markers: &PaneMarkers,
) {
    if entries.is_empty() {
        frame.render_widget(
            Paragraph::new("").block(
                context
                    .block
                    .border_style(context.accent_style)
                    .border_type(context.border_type),
            ),
            context.area,
        );
        return;
    }

    let mut items = Vec::with_capacity(entries.len());
    for (idx, entry) in entries.iter().enumerate() {
        let is_selected = Some(idx) == selected_idx;
        let style = context.styles.get_style(entry.is_dir(), is_selected);
        items.push(make_entry_row(
            entry,
            is_selected,
            style,
            &context,
            markers,
            None,
        ));
    }

    let mut state = ListState::default();
    state.select(selected_idx.map(|idx| idx.min(entries.len().saturating_sub(1))));

    frame.render_stateful_widget(
        List::new(items)
            .block(
                context
                    .block
                    .border_style(context.accent_style)
                    .border_type(context.border_type),
            )
            .highlight_style(Style::default())
            .highlight_symbol(context.highlight_symbol),
        context.area,
        &mut state,
    );
}

/// Helper: Build marker and clipboard sets for a specific preview directory.
/// Used to decorate the preview pane with marker/copy icons.
pub(crate) fn pane_marker_sets(
    nav_markers: &HashSet<PathBuf>,
    clipboard: Option<&HashSet<PathBuf>>,
    dir: Option<&Path>,
) -> (Option<HashSet<OsString>>, Option<HashSet<OsString>>) {
    match dir {
        Some(dir) => {
            let markers = nav_markers
                .iter()
                .filter(|p| p.parent().map(|parent| parent == dir).unwrap_or(false))
                .filter_map(|p| p.file_name().map(|n| n.to_os_string()))
                .collect();
            let clipboard = clipboard.map(|set| {
                set.iter()
                    .filter(|p| p.parent().map(|parent| parent == dir).unwrap_or(false))
                    .filter_map(|p| p.file_name().map(|n| n.to_os_string()))
                    .collect()
            });
            (Some(markers), clipboard)
        }
        None => (None, None),
    }
}

/// Helper: Create a PaneMarkers struct for use in pane drawing functions.
/// Builds marker and clipboard sets for a specific directory.
pub(crate) fn make_pane_markers<'a>(
    nav_markers: &'a HashSet<PathBuf>,
    clipboard: Option<&'a HashSet<PathBuf>>,
    dir: Option<&'a Path>,
    marker_icon: &'a str,
    marker_style: Style,
    clipboard_style: Style,
) -> PaneMarkers<'a> {
    let (markers, clipboard) = pane_marker_sets(nav_markers, clipboard, dir);
    PaneMarkers {
        markers,
        clipboard,
        marker_icon,
        marker_style,
        clipboard_style,
    }
}

/// Helper: Create a PaneMarkers struct for the main pane.
/// Builds marker and clipboard sets for the current directory.
/// Used in main pane drawing function.
fn make_main_pane_markers<'a>(
    app: &'a AppState,
    current_dir: &'a Path,
    clipboard: &'a Clipboard,
) -> PaneMarkers<'a> {
    let marker_theme = app.config().theme().marker();
    let marker_icon = marker_theme.icon();

    let nav = app.nav();
    let markers = nav.markers();
    let local_markers = if markers.is_empty() {
        None
    } else {
        let set: HashSet<OsString> = markers
            .iter()
            .filter(|p| p.parent() == Some(current_dir))
            .filter_map(|p| p.file_name().map(|n| n.to_os_string()))
            .collect();

        if set.is_empty() { None } else { Some(set) }
    };

    let clipboard = clipboard.entries.as_ref().map(|set| {
        set.iter()
            .filter(|p| p.parent() == Some(current_dir))
            .filter_map(|p| p.file_name().map(|n| n.to_os_string()))
            .collect::<HashSet<OsString>>()
    });

    PaneMarkers {
        markers: local_markers,
        clipboard,
        marker_icon,
        marker_style: marker_theme.style_or_theme(),
        clipboard_style: marker_theme.clipboard_style_or_theme(),
    }
}

/// Helper: Create a ListItem row for a file entry with appropriate styles and markers.
/// Used in pane drawing functions.
/// # Returns
/// * `ListItem` - The constructed ListItem for the file entry.
fn make_entry_row<'a>(
    entry: &'a FileEntry,
    is_selected: bool,
    style: Style,
    context: &PaneContext,
    markers: &PaneMarkers,
    opts: Option<&PreviewOptions>,
) -> ListItem<'a> {
    let is_marked = markers
        .markers
        .as_ref()
        .is_some_and(|set| set.contains(entry.name()));

    let is_copied = markers
        .clipboard
        .as_ref()
        .is_some_and(|set| set.contains(entry.name()));

    let mut icon_style = if is_copied {
        markers.clipboard_style
    } else {
        markers.marker_style
    };
    if is_selected {
        icon_style = icon_style.bg(style.bg.unwrap_or_default());
    }

    let mut row_style = style;
    if let Some(opts) = opts
        && is_selected
        && opts.use_underline
    {
        row_style = row_style.add_modifier(Modifier::UNDERLINED);
        if let Some(color) = opts.underline_style.fg {
            row_style = row_style.underline_color(color);
            if opts.underline_match_text {
                row_style = row_style.fg(color);
            }
        }
        if let Some(bg) = opts
            .underline_style
            .bg
            .filter(|&color| color != Color::Reset)
        {
            row_style = row_style.bg(bg);
        }
    }

    let pad = if (is_marked || is_copied) && !context.padding_str.is_empty() {
        let first_char_len = context
            .padding_str
            .chars()
            .next()
            .map_or(0, |c| c.len_utf8());

        let mut s = String::with_capacity(markers.marker_icon.len() + context.padding_str.len());
        s.push_str(markers.marker_icon);
        s.push_str(&context.padding_str[first_char_len..]);

        Span::styled(s, icon_style)
    } else {
        Span::raw(context.padding_str)
    };

    let mut spans = Vec::with_capacity(8);
    spans.push(pad);

    let symlink_fg = if entry.is_broken_sym() {
        Color::Red
    } else if entry.is_dir() {
        context.styles.symlink_dir
    } else {
        context.styles.symlink_file
    };

    if entry.is_executable() && !entry.is_dir() && !entry.is_symlink() {
        row_style = row_style.fg(context.styles.executable_fg);
    }

    if context.show_icons {
        let icon = nerd_font_icon(entry);
        let mut icon_col = String::with_capacity(icon.len() + 1);
        icon_col.push_str(icon);
        icon_col.push(' ');

        let icon_render_style = if entry.is_symlink() {
            row_style.fg(symlink_fg).add_modifier(Modifier::BOLD)
        } else {
            row_style.add_modifier(Modifier::BOLD)
        };

        spans.push(Span::styled(icon_col, icon_render_style));
    }

    if entry.is_symlink() {
        spans.push(Span::styled(entry.name_str(), row_style.fg(symlink_fg)));
    } else {
        spans.push(Span::styled(entry.name_str(), row_style));
    }

    if entry.is_dir() && context.show_marker {
        let slash_style = if entry.is_symlink() {
            row_style.fg(symlink_fg)
        } else {
            row_style
        };
        spans.push(Span::styled("/", slash_style));
    }

    if entry.is_symlink() {
        if let Some(target) = entry.symlink() {
            let target_str = target.to_string_lossy();
            let mut sym_text = String::with_capacity(target_str.len() + 20);
            sym_text.push_str(" -> ");
            sym_text.push_str(&target_str);

            let target_style = if entry.is_broken_sym() {
                sym_text.push_str(" [broken]");
                row_style.fg(symlink_fg)
            } else {
                row_style.fg(context.styles.symlink_target)
            };

            spans.push(Span::styled(sym_text, target_style));
        } else if entry.is_broken_sym() {
            spans.push(Span::styled(" -> [broken]", row_style.fg(Color::Red)));
        }
    }

    ListItem::new(Line::from(spans)).style(row_style)
}
