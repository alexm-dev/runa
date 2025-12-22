use crate::app::{AppState, PreviewData};
use crate::file_manager::FileEntry;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

pub struct PaneStyles {
    pub item: Style,
    pub dir: Style,
    pub selection: Style,
}

impl PaneStyles {
    pub fn get_style(&self, is_dir: bool, is_selected: bool) -> Style {
        let mut style = if is_dir {
            if self.dir.fg != Some(Color::Reset) {
                self.dir
            } else {
                self.item
            }
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
}

pub struct PreviewOptions {
    pub use_underline: bool,
    pub underline_match_text: bool,
    pub underline_style: Style,
}

pub fn draw_main(frame: &mut Frame, app: &AppState, context: PaneContext) {
    let show_marker = app.config().display().dir_marker();
    let selected_idx = app.visible_selected();

    let items: Vec<ListItem> = app
        .visible_entries()
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let is_selected = Some(i) == selected_idx;
            let text = if e.is_dir() && show_marker {
                e.display_name()
            } else {
                e.name_str()
            };
            let style = context.styles.get_style(e.is_dir(), is_selected);

            ListItem::new(text).style(style)
        })
        .collect();

    let mut state = ListState::default();
    if app.has_visible_entries() {
        state.select(selected_idx);
    }

    frame.render_stateful_widget(
        List::new(items)
            .block(context.block.border_style(context.accent_style))
            .highlight_style(Style::default())
            .highlight_symbol(context.highlight_symbol)
            .scroll_padding(app.config().display().scroll_padding()),
        context.area,
        &mut state,
    );
}

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
            let text = lines.join("\n");

            frame.render_widget(
                Paragraph::new(text).block(context.block.border_style(context.accent_style)),
                context.area,
            );
        }

        PreviewData::Directory(entries) => {
            if entries.is_empty() {
                frame.render_widget(Paragraph::new("").block(context.block), context.area);
                return;
            }

            let items: Vec<ListItem> = entries
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let is_selected = Some(i) == selected_idx;
                    let mut style = context.styles.get_style(e.is_dir(), false);

                    if is_selected && opts.use_underline {
                        style = style.add_modifier(Modifier::UNDERLINED);

                        if let Some(bg_color) = opts.underline_style.bg
                            && bg_color != Color::Reset
                        {
                            style = style.bg(bg_color);
                        }
                        if let Some(color) = opts.underline_style.fg {
                            style = style.underline_color(color);
                            if opts.underline_match_text {
                                style = style.fg(color);
                            }
                        }
                    }

                    ListItem::new(e.display_name()).style(style)
                })
                .collect();

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

pub fn draw_origin(
    frame: &mut Frame,
    context: PaneContext,
    entries: &[FileEntry],
    selected_idx: Option<usize>,
) {
    if entries.is_empty() {
        frame.render_widget(Paragraph::new("").block(context.block), context.area);
        return;
    }

    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let is_selected = Some(i) == selected_idx;
            // Use metadata from FileEntry to apply correct coloring
            let style = context.styles.get_style(e.is_dir(), is_selected);
            ListItem::new(e.display_name()).style(style)
        })
        .collect();

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
