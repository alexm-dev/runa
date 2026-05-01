//! UI pane drawing module for runa.
//!
//! This module provides renderes/drawers for the parent, main and preview panes.
//! All layout and highlighting logic for items, cursor and file type coloring is handled here.
//!
//! Used internally by ui::render

use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use ratatui::widgets::BorderType;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::{AppState, Clipboard, PreviewData};
use crate::config::{Display, Theme};
use crate::core::FileEntry;
use crate::ui::icons;
use crate::utils::text::StrBuffer;

const MAX_RIGHT_COLUMN_WIDTH: u16 = 24;
const MIN_LEFT_WIDTH: u16 = 12;

/// Styles used for rendering items in a pane
/// Includes styles for regular items, directories and selected items
pub(super) struct PaneStyles {
    item: Style,
    dir: Style,
    selection: Style,
    symlink_dir: Color,
    symlink_file: Color,
    symlink_target: Color,
    executable_fg: Color,
}

impl PaneStyles {
    pub(super) fn new(theme: &Theme) -> Self {
        let sym = theme.symlink_theme();
        Self {
            item: theme.entry_style(),
            dir: theme.directory_style(),
            selection: theme.selection_style(),
            symlink_dir: sym.directory(),
            symlink_file: sym.file(),
            symlink_target: sym.target(),
            executable_fg: theme.exe_color(),
        }
    }

    pub(super) fn with_entry(mut self, item: Style) -> Self {
        self.item = item;
        self
    }

    pub(super) fn with_selection(mut self, selection: Style) -> Self {
        self.selection = selection;
        self
    }

    fn get_style(&self, is_dir: bool, is_selected: bool) -> Style {
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
pub(super) struct PaneContext<'a> {
    area: Rect,
    block: Block<'a>,
    border_type: BorderType,
    theme: &'a Theme,
    accent_style: Style,
    styles: PaneStyles,
    highlight_symbol: &'a str,
    padding_str: &'static str,
    show_icons: bool,
    show_marker: bool,
}

impl<'a> PaneContext<'a> {
    pub(super) fn new(
        area: Rect,
        block: Block<'a>,
        border_type: BorderType,
        theme: &'a Theme,
        display: &'a Display,
        styles: PaneStyles,
        highlight_symbol: &'a str,
    ) -> Self {
        Self {
            area,
            block,
            border_type,
            theme,
            accent_style: theme.accent_style(),
            padding_str: display.padding_str(),
            show_icons: display.icons(),
            show_marker: display.dir_marker(),
            styles,
            highlight_symbol,
        }
    }
}

/// Marker and clipboard data for use in pane drawing functions
pub(super) struct PaneMarkers<'a> {
    markers: Option<HashSet<OsString>>,
    clipboard: Option<HashSet<OsString>>,
    marker_icon: &'a str,
    marker_style: Style,
    clipboard_style: Style,
}

/// Options for preview pane rendering
struct PreviewOptions {
    use_underline: bool,
    underline_match_text: bool,
    underline_style: Style,
}

#[derive(Clone, Copy)]
struct RightCol<'a> {
    text: Option<&'a str>,
    width: u16,
}

impl<'a> RightCol<'a> {
    #[inline]
    fn none() -> Self {
        Self {
            text: None,
            width: 0,
        }
    }

    #[inline]
    fn reserve(self) -> usize {
        if self.text.is_some() {
            self.width as usize + 1
        } else {
            0
        }
    }
}

/// Draws the main file list pane in the UI
///
/// Highlights selection, markers and directories and handles styling for items.
pub(super) fn draw_main(
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

    let sort_column = app.nav().sort_column();
    let inner_w = pane_inner_width(&context);
    let (show_col, right_w) = right_col_config(inner_w, sort_column.as_deref());

    let mut items = Vec::with_capacity(shown_len);
    for (vis_idx, &abs_idx) in app.nav().shown_indices().iter().enumerate() {
        let entry = &app.nav().entries()[abs_idx];
        let is_selected = Some(vis_idx) == selected_idx;
        let entry_style = context.styles.get_style(entry.is_dir(), is_selected);
        let right = right_col_for(show_col, right_w, sort_column.as_deref(), abs_idx);

        items.push(make_entry_row(
            entry,
            is_selected,
            entry_style,
            &context,
            &markers,
            None,
            right,
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
pub(super) fn draw_preview(
    frame: &mut Frame,
    app: &AppState,
    context: PaneContext,
    markers: &PaneMarkers,
) {
    let preview = app.preview().data();
    let selected_idx = Some(app.preview().selected_idx());

    let opts = PreviewOptions {
        use_underline: app.config().display().preview_underline(),
        underline_match_text: app.config().display().preview_underline_color(),
        underline_style: app.config().theme().underline_style(),
    };

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
                Paragraph::new(text.clone()).block(
                    context
                        .block
                        .border_style(context.accent_style)
                        .border_type(context.border_type),
                ),
                context.area,
            );
        }

        PreviewData::Directory {
            entries,
            sort_column,
        } => {
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

            let inner_w = pane_inner_width(&context);
            let (show_col, right_w) = right_col_config(inner_w, sort_column.as_deref());

            let mut items = Vec::with_capacity(entries.len());
            for (idx, entry) in entries.iter().enumerate() {
                let is_selected = Some(idx) == selected_idx;
                let style = context.styles.get_style(entry.is_dir(), is_selected);
                let right = right_col_for(show_col, right_w, sort_column.as_deref(), idx);

                items.push(make_entry_row(
                    entry,
                    is_selected,
                    style,
                    &context,
                    markers,
                    Some(&opts),
                    right,
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
pub(super) fn draw_parent(
    frame: &mut Frame,
    app: &AppState,
    context: PaneContext,
    markers: &PaneMarkers,
) {
    let entries = app.parent().entries();
    let selected_idx = app.parent().selected_idx();
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

    let sort_column = app.parent().sort_column();
    let inner_w = pane_inner_width(&context);
    let (show_col, right_w) = right_col_config(inner_w, sort_column.as_deref());

    let mut items = Vec::with_capacity(entries.len());
    for (idx, entry) in entries.iter().enumerate() {
        let is_selected = Some(idx) == selected_idx;
        let style = context.styles.get_style(entry.is_dir(), is_selected);
        let right = right_col_for(show_col, right_w, sort_column.as_deref(), idx);

        items.push(make_entry_row(
            entry,
            is_selected,
            style,
            &context,
            markers,
            None,
            right,
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

/// Helper: Create a PaneMarkers struct for use in pane drawing functions.
/// Builds marker and clipboard sets for a specific directory.
pub(super) fn make_pane_markers<'a>(
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

/// Helper: Build marker and clipboard sets for a specific preview directory.
/// Used to decorate the preview pane with marker/copy icons.
fn pane_marker_sets(
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
    right: RightCol<'a>,
) -> ListItem<'a> {
    let mut used_w: usize = 0;

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
        let has_distinct_bg = icon_style.bg.is_some() && icon_style.bg != Some(Color::Reset);

        if !has_distinct_bg {
            icon_style = icon_style.bg(style.bg.unwrap_or_default());
        }
    }

    let mut row_style = style;

    if let Some(override_style) =
        context
            .theme
            .entry_color_override(entry.name_str(), entry.is_dir(), entry.ext())
    {
        row_style = row_style.patch(override_style);

        if is_selected {
            row_style = row_style.bg(style.bg.unwrap_or_default());
        }
    }

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

    let mut spans = Vec::with_capacity(10);

    if (is_marked || is_copied) && !context.padding_str.is_empty() {
        let first_char_len = context
            .padding_str
            .chars()
            .next()
            .map_or(0, |c| c.len_utf8());

        let mut s = String::with_capacity(markers.marker_icon.len() + context.padding_str.len());
        s.push_str(markers.marker_icon);
        s.push_str(&context.padding_str[first_char_len..]);

        used_w += UnicodeWidthStr::width(s.as_str());
        spans.push(Span::styled(s, icon_style));
    } else {
        used_w += UnicodeWidthStr::width(context.padding_str);
        spans.push(Span::raw(context.padding_str));
    }

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
        let (icon, icon_custom_color) = icons::nerd_font_icon(entry, context.theme);

        let mut icon_col = String::with_capacity(icon.len() + 1);
        icon_col.push_str(icon);
        icon_col.push(' ');

        used_w += UnicodeWidthStr::width(icon_col.as_str());

        let mut icon_render_style = row_style.add_modifier(Modifier::BOLD);

        if entry.is_symlink() {
            icon_render_style = icon_render_style.fg(symlink_fg);
        } else if let Some(custom_color) = icon_custom_color {
            icon_render_style = icon_render_style.fg(custom_color);
        }

        spans.push(Span::styled(icon_col, icon_render_style));
    }

    let total_w = context.block.inner(context.area).width as usize;
    let reserve = right.reserve();

    let name_raw = entry.name_str();
    let name_budget = total_w
        .saturating_sub(reserve)
        .saturating_sub(used_w)
        .max(1);
    let name = truncate_owned(name_raw, name_budget);

    used_w += UnicodeWidthStr::width(&*name);

    if entry.is_symlink() {
        spans.push(Span::styled(name, row_style.fg(symlink_fg)));
    } else {
        spans.push(Span::styled(name, row_style));
    }

    if entry.is_dir() && context.show_marker && left_remaining(total_w, used_w, reserve) >= 1 {
        used_w += 1;
        let slash_style = if entry.is_symlink() {
            row_style.fg(symlink_fg)
        } else {
            row_style
        };
        spans.push(Span::styled("/", slash_style));
    }

    if entry.is_symlink() {
        let rem = left_remaining(total_w, used_w, reserve);
        if rem > 0 {
            if let Some(target) = entry.symlink() {
                let target_str = target.to_string_lossy();
                let mut sym_text = String::with_capacity(target_str.len() + 24);
                sym_text.push_str(" -> ");
                sym_text.push_str(&target_str);

                if entry.is_broken_sym() {
                    sym_text.push_str(" [broken]");
                }

                let sym_text = truncate_owned(&sym_text, rem).into_owned();
                used_w += UnicodeWidthStr::width(&*sym_text);

                let target_style = if entry.is_broken_sym() {
                    row_style.fg(symlink_fg)
                } else {
                    row_style.fg(context.styles.symlink_target)
                };

                spans.push(Span::styled(sym_text, target_style));
            } else if entry.is_broken_sym() {
                let s = truncate_owned(" -> [broken]", rem);
                used_w += UnicodeWidthStr::width(&*s);
                spans.push(Span::styled(s, row_style.fg(Color::Red)));
            }
        }
    }

    if let Some(col) = right.text {
        let col_area_w = right.width as usize;
        let right_text = if UnicodeWidthStr::width(col) <= col_area_w {
            build_right_field(col, total_w, used_w, col_area_w)
        } else {
            let col = truncate_owned(col, col_area_w);
            build_right_field(&col, total_w, used_w, col_area_w)
        };

        spans.push(Span::styled(right_text, row_style));
    }

    ListItem::new(Line::from(spans)).style(row_style)
}

#[inline]
fn pane_show_col(inner_w: u16, right_w: u16) -> bool {
    right_w > 0 && inner_w >= right_w + 1 + MIN_LEFT_WIDTH
}

fn truncate_owned(s: &str, max_w: usize) -> Cow<'_, str> {
    if UnicodeWidthStr::width(s) <= max_w {
        return Cow::Borrowed(s);
    }
    if max_w <= 1 {
        return Cow::Borrowed("…");
    }

    let mut w = 0usize;
    let mut last_idx = 0usize;

    for (idx, ch) in s.char_indices() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw >= max_w {
            let mut out = String::with_capacity(last_idx + 3);
            out.push_str(&s[..last_idx]);
            out.push('…');
            return Cow::Owned(out);
        }
        last_idx = idx;
        w += cw;
    }

    Cow::Borrowed(s)
}

#[inline]
fn left_remaining(total_w: usize, used_w: usize, reserve: usize) -> usize {
    total_w.saturating_sub(reserve).saturating_sub(used_w)
}

#[inline]
fn build_right_field(col: &str, total_w: usize, used_w: usize, col_area_w: usize) -> String {
    let pad_spaces = total_w.saturating_sub(used_w + 1 + col_area_w);
    let col_w = UnicodeWidthStr::width(col).min(col_area_w);
    let inner_pad = col_area_w.saturating_sub(col_w);

    let total_spaces = pad_spaces + 1 + inner_pad;
    let mut out = String::with_capacity(total_spaces + col.len());

    out.extend(std::iter::repeat_n(' ', total_spaces));
    out.push_str(col);

    out
}

#[inline]
fn pane_inner_width(context: &PaneContext) -> u16 {
    context.block.inner(context.area).width
}

#[inline]
fn right_col_width(sort_column: Option<&StrBuffer>) -> u16 {
    let Some(col) = sort_column else {
        return 0;
    };

    let max_w = col.iter().map(UnicodeWidthStr::width).max().unwrap_or(0);

    (max_w as u16).min(MAX_RIGHT_COLUMN_WIDTH)
}

#[inline]
fn right_col_config(inner_w: u16, sort_column: Option<&StrBuffer>) -> (bool, u16) {
    let right_w = right_col_width(sort_column);
    let show_col = pane_show_col(inner_w, right_w);
    (show_col, right_w)
}

#[inline]
fn right_col_at(sort_column: Option<&StrBuffer>, idx: usize) -> Option<&str> {
    sort_column.map(|c| c.get(idx)).filter(|s| !s.is_empty())
}

#[inline]
fn right_col_for(
    show_col: bool,
    right_w: u16,
    sort_column: Option<&StrBuffer>,
    idx: usize,
) -> RightCol<'_> {
    if !show_col {
        return RightCol::none();
    }
    RightCol {
        text: right_col_at(sort_column, idx),
        width: right_w,
    }
}
