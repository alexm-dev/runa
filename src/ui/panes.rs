//! UI pane drawing module for runa.
//!
//! This module provides renderes/drawers for the parent, main and preview panes.
//! All layout and highlighting logic for items, cursor and file type coloring is handled here.
//!
//! Used internally by [ui::render]

use crate::app::{AppState, PreviewData};
use crate::core::{FileEntry, symlink_target_resolved};
use crate::ui::icons::nerd_font_icon;
use ansi_to_tui::IntoText;
use ratatui::text::Text;
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
pub struct PaneStyles {
    pub item: Style,
    pub dir: Style,
    pub selection: Style,
    pub symlink: Color,
}

impl PaneStyles {
    pub fn get_style(&self, is_dir: bool, is_selected: bool) -> Style {
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
    pub fn get_symlink_style(&self, base_style: Style) -> Style {
        base_style.fg(self.symlink)
    }
}

/// Context data for pane rendering functions
pub struct PaneContext<'a> {
    pub area: Rect,
    pub block: Block<'a>,
    pub border_type: BorderType,
    pub accent_style: Style,
    pub styles: PaneStyles,
    pub highlight_symbol: &'a str,
    pub entry_padding: u8,
    pub padding_str: &'static str,
    pub show_icons: bool,
    pub show_marker: bool,
}

/// Options for preview pane rendering
pub struct PreviewOptions {
    pub use_underline: bool,
    pub underline_match_text: bool,
    pub underline_style: Style,
}

/// Marker and clipboard data for use in pane drawing functions
pub struct PaneMarkers<'a> {
    pub markers: Option<HashSet<OsString>>,
    pub clipboard: Option<HashSet<OsString>>,
    pub marker_icon: &'a str,
    pub marker_style: Style,
    pub clipboard_style: Style,
}

/// Draws the main file list pane in the UI
///
/// Highlights selection, markers and directories and handles styling for items.
pub fn draw_main(frame: &mut Frame, app: &AppState, context: PaneContext) {
    let selected_idx = app.visible_selected();
    let markers = app.nav().markers();
    let marker_theme = app.config().theme().marker();
    let marker_icon = marker_theme.icon();
    let marker_pad = " ".repeat(unicode_width::UnicodeWidthStr::width(marker_icon));
    let entry_padding = context.entry_padding as usize;
    let current_dir = app.nav().current_dir();
    let clipboard = app.actions().clipboard();

    let padding_str = if entry_padding > 1 {
        " ".repeat(entry_padding - 1)
    } else {
        String::new()
    };

    if app.nav().shown_entries_len() == 0 && !app.nav().filter().is_empty() {
        let style = context.styles.item;
        let line = Line::from(vec![
            Span::raw(&padding_str),
            Span::styled("[Now results for this filter]", style),
        ]);
        frame.render_widget(Paragraph::new(line).block(context.block), context.area);
        return;
    }

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
        let entry_path = current_dir.join(entry.name());
        let is_copied = clipboard
            .as_ref()
            .map(|set| set.contains(&entry_path))
            .unwrap_or(false);

        let name_str = if entry.is_dir() && context.show_marker {
            entry.display_name()
        } else {
            entry.name_str()
        };

        let entry_style = context.styles.get_style(entry.is_dir(), is_selected);
        let mut spans = Vec::with_capacity(8);

        if entry_padding == 0 {
            if context.show_icons {
                let icon = nerd_font_icon(entry);
                let mut icon_col = String::with_capacity(icon.len() + 1);
                icon_col.push_str(icon);
                icon_col.push(' ');
                spans.push(Span::styled(
                    icon_col,
                    entry_style.add_modifier(Modifier::BOLD),
                ));
            }
            spans.push(Span::raw(name_str));
        } else {
            let mut marker_style = if is_copied {
                marker_theme.clipboard_style_or_theme()
            } else {
                marker_theme.style_or_theme()
            };

            if is_selected {
                marker_style = marker_style.bg(entry_style.bg.unwrap_or_default());
            }

            if is_marked || is_copied {
                spans.push(Span::styled(marker_icon, marker_style));
            } else {
                spans.push(Span::styled(&marker_pad, marker_style));
            }

            if entry_padding > 1 {
                spans.push(Span::raw(&padding_str));
            }
            if context.show_icons {
                let icon = nerd_font_icon(entry);
                let mut icon_col = String::with_capacity(icon.len() + 1);
                icon_col.push_str(icon);
                icon_col.push(' ');
                spans.push(Span::styled(
                    icon_col,
                    entry_style.add_modifier(Modifier::BOLD),
                ));
            }
            spans.push(Span::raw(name_str));
            if entry.is_symlink()
                && let Some(target) = symlink_target_resolved(entry, current_dir)
            {
                let mut sym_text = String::with_capacity(4 + target.to_string_lossy().len());
                sym_text.push_str(" -> ");
                sym_text.push_str(&target.to_string_lossy());
                spans.push(Span::styled(
                    sym_text,
                    context.styles.get_symlink_style(entry_style),
                ));
            }
        }

        let line = Line::from(spans);
        ListItem::new(line).style(entry_style)
    });

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
pub fn draw_preview(
    path: Option<&Path>,
    frame: &mut Frame,
    context: PaneContext,
    preview: &PreviewData,
    selected_idx: Option<usize>,
    opts: PreviewOptions,
    markers: &PaneMarkers,
) {
    match preview {
        PreviewData::Empty => {
            frame.render_widget(Paragraph::new("").block(context.block), context.area);
        }

        PreviewData::File(lines) => {
            let raw = lines.join("\n");
            let text = raw.into_text().unwrap_or_else(|_| Text::from(raw));

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

            let items: Vec<ListItem> = entries
                .iter()
                .enumerate()
                .map(|(idx, entry)| {
                    let is_selected = Some(idx) == selected_idx;
                    let style = context.styles.get_style(entry.is_dir(), is_selected);
                    make_entry_row(
                        entry,
                        path,
                        is_selected,
                        style,
                        &context,
                        markers,
                        Some(&opts),
                    )
                })
                .collect();

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
pub fn draw_parent(
    path: Option<&Path>,
    frame: &mut Frame,
    context: PaneContext,
    entries: &[FileEntry],
    selected_idx: Option<usize>,
    markers: &PaneMarkers,
) {
    if entries.is_empty() {
        frame.render_widget(Paragraph::new("").block(context.block), context.area);
        return;
    }

    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let is_selected = Some(idx) == selected_idx;
            let style = context.styles.get_style(entry.is_dir(), is_selected);
            make_entry_row(entry, path, is_selected, style, &context, markers, None)
        })
        .collect();

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
pub fn pane_marker_sets(
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
pub fn make_pane_markers<'a>(
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

/// Helper: Create a ListItem row for a file entry with appropriate styles and markers.
/// Used in pane drawing functions.
///
/// # Ar
fn make_entry_row<'a>(
    entry: &'a FileEntry,
    current_dir: Option<&Path>,
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

    let pad = if is_marked || is_copied {
        let mut pad = context.padding_str.to_owned();
        if !pad.is_empty() {
            pad.replace_range(0..1, markers.marker_icon);
        }
        Span::styled(pad, icon_style)
    } else {
        Span::raw(context.padding_str)
    };

    let mut spans = vec![pad];
    if context.show_icons {
        let icon = nerd_font_icon(entry);
        let mut icon_col = String::with_capacity(icon.len() + 1);
        icon_col.push_str(icon);
        icon_col.push(' ');
        spans.push(Span::styled(
            icon_col,
            row_style.add_modifier(Modifier::BOLD),
        ));
    }
    let name_str = if entry.is_dir() && context.show_marker {
        entry.display_name()
    } else {
        entry.name_str()
    };

    spans.push(Span::raw(name_str));

    if entry.is_symlink()
        && let Some(dir) = current_dir
        && let Some(target) = symlink_target_resolved(entry, dir)
    {
        let mut sym_text = String::with_capacity(4 + target.to_string_lossy().len());
        sym_text.push_str(" -> ");
        sym_text.push_str(&target.to_string_lossy());
        spans.push(Span::styled(
            sym_text,
            context.styles.get_symlink_style(row_style),
        ));
    }

    let line = Line::from(spans);
    ListItem::new(line).style(row_style)
}
