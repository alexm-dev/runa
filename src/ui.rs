//! Terminal User Interface rendering logic.
//!
//! This module handles all layout, pane arrangement, and rendering for the main file manager UI.
//! It coordinates drawing of the parent, main, and preview panes, as well as input dialogs and the status bar.
//!
//! UI configuration is driven by the applications display/theme settings and adapts to different layouts: unified, split, or normal views.
//!
//! Functions:
//! - render: Main entry point for rendering the entire UI to a frame.
//! - layout_chunks: Utility for calculating pane positions and widths based on config.
//!
//! See submodules [panes] and [widgets] for detailed drawing functions.

pub mod panes;
pub mod widgets;

use self::panes::PaneContext;
use crate::{
    app::{
        AppState,
        actions::{ActionMode, InputMode},
    },
    ui::panes::{PaneStyles, PreviewOptions},
    ui::widgets::{draw_input_dialog, draw_status_line},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Render function which renders the entire terminal UI for runa on each frame.
/// Handles layout, pane rendering, borders, headers and coordinates all widgets.
/// # Params:
/// - frame: the drawing frame from ratatui::frame
/// - app: runa's shared state, mutated as needed to display metrics
pub fn render(frame: &mut Frame, app: &mut AppState) {
    let mut root_area = frame.area();
    {
        let chunks = layout_chunks(root_area, app);
        let mut metrics = crate::app::LayoutMetrics::default();
        let display_cfg = app.config().display();

        let mut current_idx = 0;
        let has_sep = display_cfg.separators() && !display_cfg.is_split();

        // Helper to determine inner space available for text
        let get_inner = |rect: ratatui::layout::Rect| {
            let width = if display_cfg.is_split() || display_cfg.is_unified() {
                rect.width.saturating_sub(2)
            } else {
                rect.width
            };
            let height = rect.height.saturating_sub(2);
            (width as usize, height as usize)
        };

        if display_cfg.parent() && current_idx < chunks.len() {
            metrics.parent_width = get_inner(chunks[current_idx]).0;
            current_idx += if has_sep { 2 } else { 1 };
        }

        if current_idx < chunks.len() {
            metrics.main_width = get_inner(chunks[current_idx]).0;
            current_idx += if has_sep && display_cfg.preview() {
                2
            } else {
                1
            };
        }

        if display_cfg.preview() && current_idx < chunks.len() {
            let (width, height) = get_inner(chunks[current_idx]);
            metrics.preview_width = width;
            metrics.preview_height = height;
        }

        *app.metrics_mut() = metrics;
    }

    let cfg = app.config();
    let display_cfg = cfg.display();
    let theme_cfg = cfg.theme();

    let accent_style = theme_cfg.accent().as_style();
    let selection_style = theme_cfg.selection().as_style();
    let path_str = app.nav().current_dir().to_string_lossy();
    let path_style = theme_cfg.path().as_style();

    let padding_str = display_cfg.padding_str();

    // Root Border / Header Logic
    if display_cfg.is_unified() {
        let mut outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(accent_style);
        if display_cfg.titles() {
            outer_block = outer_block.title(Line::from(vec![Span::styled(
                format!(" {} ", path_str),
                path_style,
            )]));
        }
        frame.render_widget(outer_block, root_area);
        root_area = Block::default().borders(Borders::ALL).inner(root_area);
    } else {
        let header_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(root_area);
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                format!("{} ", path_str),
                path_style,
            )])),
            header_layout[0],
        );
        root_area = header_layout[1];
    }

    // Render Panes
    let chunks = layout_chunks(root_area, app);
    let mut pane_idx = 0;
    let show_separators = display_cfg.separators() && !display_cfg.is_split();

    // PARENT PANE
    if display_cfg.parent() && pane_idx < chunks.len() {
        panes::draw_parent(
            frame,
            PaneContext {
                area: chunks[pane_idx],
                block: widgets::get_pane_block("Parent", app),
                accent_style,
                styles: PaneStyles {
                    item: theme_cfg.parent().effective_style(&theme_cfg.entry()),
                    dir: theme_cfg.directory().as_style(),
                    selection: theme_cfg.parent().selection_style(selection_style),
                },
                highlight_symbol: "",
                entry_padding: display_cfg.entry_padding(),
                padding_str,
            },
            app.parent().entries(),
            app.parent().selected_idx(),
        );
        pane_idx += 1;
        if show_separators && pane_idx < chunks.len() {
            widgets::draw_separator(
                frame,
                Rect {
                    x: chunks[pane_idx].x,
                    y: root_area.y,
                    width: 1,
                    height: root_area.height,
                },
                theme_cfg.separator().as_style(),
            );
            pane_idx += 1;
        }
    }

    // MAIN PANE
    if pane_idx < chunks.len() {
        let symbol = if display_cfg.selection_marker() {
            theme_cfg.selection_icon()
        } else {
            ""
        };

        let pane_style = PaneStyles {
            item: theme_cfg.entry().as_style(),
            dir: theme_cfg.directory().as_style(),
            selection: selection_style,
        };

        panes::draw_main(
            frame,
            app,
            PaneContext {
                area: chunks[pane_idx],
                block: widgets::get_pane_block("Files", app),
                accent_style,
                styles: pane_style,
                highlight_symbol: symbol,
                entry_padding: display_cfg.entry_padding(),
                padding_str,
            },
        );
        pane_idx += 1;
        if show_separators && display_cfg.preview() && pane_idx < chunks.len() {
            widgets::draw_separator(
                frame,
                Rect {
                    x: chunks[pane_idx].x,
                    y: root_area.y,
                    width: 1,
                    height: root_area.height,
                },
                theme_cfg.separator().as_style(),
            );
            pane_idx += 1;
        }
    }

    // PREVIEW PANE
    if display_cfg.preview() && pane_idx < chunks.len() {
        let area = chunks[pane_idx];
        let bg_filler = Block::default().style(theme_cfg.preview().as_style());
        frame.render_widget(bg_filler, area);

        let is_dir = app
            .nav()
            .selected_entry()
            .map(|e| e.is_dir())
            .unwrap_or(false);

        panes::draw_preview(
            frame,
            PaneContext {
                area: chunks[pane_idx],
                block: widgets::get_pane_block("Preview", app),
                accent_style,
                styles: PaneStyles {
                    item: theme_cfg.parent().effective_style(&theme_cfg.entry()),
                    dir: theme_cfg.directory().as_style(),
                    selection: theme_cfg.preview().selection_style(selection_style),
                },
                highlight_symbol: "",
                entry_padding: display_cfg.entry_padding(),
                padding_str,
            },
            app.preview().data(),
            if is_dir {
                Some(app.preview().selected_idx())
            } else {
                None
            },
            PreviewOptions {
                use_underline: display_cfg.preview_underline(),
                underline_match_text: display_cfg.preview_underline_color(),
                underline_style: theme_cfg.underline().as_style(),
            },
        );
    }
    draw_status_line(frame, app);
    draw_input_dialog(frame, app, accent_style);
}

/// Returns the rectangular areas for all active panes, given the current configuration
///
/// The result is used for positioning file navigation, parent and preview panes in the layout.
/// Handles separators and dynamic ratios.
pub fn layout_chunks(size: Rect, app: &AppState) -> Vec<Rect> {
    let cfg = app.config().display();
    let mut constraints = Vec::new();
    let show_sep = cfg.separators() && !cfg.is_split();

    let parent = if cfg.parent() {
        cfg.parent_ratio() as u32
    } else {
        0
    };
    let main = cfg.main_ratio() as u32;
    let preview = if cfg.preview() {
        cfg.preview_ratio() as u32
    } else {
        0
    };

    let enabled = [
        (parent, cfg.parent()),
        (main, true),
        (preview, cfg.preview()),
    ];

    let total: u32 = enabled
        .iter()
        .filter(|e| e.1)
        .map(|e| e.0)
        .sum::<u32>()
        .max(1);

    let mut sum_pct: u16 = 0;
    let pane_count = enabled.iter().filter(|e| e.1).count();
    let mut pane_added = 0;

    for &(val, enabled) in &enabled {
        if enabled {
            pane_added += 1;
            let pct = if pane_added == pane_count {
                100 - sum_pct
            } else {
                let pct = ((val as f32 / total as f32) * 100.0).round() as u16;
                sum_pct += pct;
                pct
            };
            constraints.push(Constraint::Percentage(pct));
            if show_sep && pane_added < pane_count {
                constraints.push(Constraint::Length(1));
            }
        }
    }

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(size)
        .to_vec()
}
