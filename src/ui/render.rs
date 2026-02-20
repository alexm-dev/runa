//! UI renderer implementation.
//!
//! Contains the top-level `render` entry point used by the terminal loop and the
//! layout helpers that split the screen into parent/main/preview chunks.
//!
//! This module should stay mostly “pure rendering”: it reads state + config and
//! produces widgets, without owning runa core logic.

use crate::{
    app::{
        AppState, Clipboard, LayoutMetrics, PreviewData,
        actions::{ActionMode, InputMode},
    },
    ui::{
        overlays::Overlay,
        panes::{self, PaneContext, PaneStyles, PreviewOptions},
        widgets,
    },
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Render function which renders the entire terminal UI for runa on each frame.
/// Handles layout, pane rendering, borders, headers and coordinates all widgets.
pub(crate) fn render(frame: &mut Frame, app: &mut AppState, clipboard: &mut Clipboard) {
    let mut root_area = frame.area();
    let metrics = calculate_layout_metrics(frame.area(), app);
    app.update_layout_metrics(metrics);

    let cfg = app.config();
    let display_cfg = cfg.display();
    let theme_cfg = cfg.theme();

    let accent_style = theme_cfg.accent_style();
    let selection_style = theme_cfg.selection_style();

    let symlink_theme = theme_cfg.symlink_theme();

    let padding_str = display_cfg.padding_str();
    let border_type = display_cfg.border_shape().as_border_type();

    let markers = app.nav().markers();
    let marker_theme = theme_cfg.marker();
    let marker_icon = marker_theme.icon();
    let marker_style = marker_theme.style_or_theme();

    let clipboard_style = marker_theme.clipboard_style_or_theme();

    root_area = render_root_and_header(frame, app, root_area);

    // Render Panes
    let chunks = layout_chunks(root_area, app);
    let mut pane_idx = 0;
    let show_separators = display_cfg.separators() && !display_cfg.is_split();

    // PARENT PANE
    if display_cfg.parent() && pane_idx < chunks.len() {
        let parent_dir = app
            .parent()
            .last_path()
            .or_else(|| app.nav().current_dir().parent());
        let parent_markers = panes::make_pane_markers(
            markers,
            clipboard.entries.as_ref(),
            parent_dir,
            marker_icon,
            marker_style,
            clipboard_style,
        );

        let parent_pane_style = PaneStyles {
            item: theme_cfg.parent_item_style(),
            dir: theme_cfg.directory_style(),
            selection: theme_cfg.parent_selection_style(),
            symlink_file: symlink_theme.file(),
            symlink_dir: symlink_theme.directory(),
            symlink_target: symlink_theme.target(),
            executable_fg: theme_cfg.exe_color(),
        };

        panes::draw_parent(
            frame,
            PaneContext {
                area: chunks[pane_idx],
                block: widgets::get_pane_block("Parent", app),
                border_type,
                accent_style,
                styles: parent_pane_style,
                highlight_symbol: "",
                padding_str,
                show_icons: display_cfg.icons(),
                show_marker: display_cfg.dir_marker(),
            },
            app.parent().entries(),
            app.parent().selected_idx(),
            &parent_markers,
        );
        pane_idx += 1;
        if show_separators && pane_idx < chunks.len() {
            render_separator(
                frame,
                chunks[pane_idx].x,
                root_area,
                theme_cfg.separator_style(),
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
            item: theme_cfg.entry_style(),
            dir: theme_cfg.directory_style(),
            selection: selection_style,
            symlink_file: symlink_theme.file(),
            symlink_dir: symlink_theme.directory(),
            symlink_target: symlink_theme.target(),
            executable_fg: theme_cfg.exe_color(),
        };

        panes::draw_main(
            frame,
            app,
            PaneContext {
                area: chunks[pane_idx],
                block: widgets::get_pane_block("Files", app),
                border_type,
                accent_style,
                styles: pane_style,
                highlight_symbol: symbol,
                padding_str,
                show_icons: display_cfg.icons(),
                show_marker: display_cfg.dir_marker(),
            },
            clipboard,
        );
        pane_idx += 1;
        if show_separators && display_cfg.preview() && pane_idx < chunks.len() {
            render_separator(
                frame,
                chunks[pane_idx].x,
                root_area,
                theme_cfg.separator_style(),
            );
            pane_idx += 1;
        }
    }

    // PREVIEW PANE
    if display_cfg.preview() && pane_idx < chunks.len() {
        let area = chunks[pane_idx];
        let bg_filler = Block::default().style(theme_cfg.preview().effective_style_or_theme());
        frame.render_widget(bg_filler, area);

        let preview_dir = app.preview().current_path();
        let preview_markers = panes::make_pane_markers(
            markers,
            clipboard.entries.as_ref(),
            preview_dir,
            marker_icon,
            marker_style,
            clipboard_style,
        );

        let preview_pane_styles = PaneStyles {
            item: theme_cfg.preview_item_style(),
            dir: theme_cfg.directory_style(),
            selection: theme_cfg.preview_selection_style(),
            symlink_file: symlink_theme.file(),
            symlink_dir: symlink_theme.directory(),
            symlink_target: symlink_theme.target(),
            executable_fg: theme_cfg.exe_color(),
        };

        let (preview_data, selected_idx) = {
            let preview = app.preview();
            match preview.data() {
                PreviewData::Directory(_) => (preview.data(), Some(preview.selected_idx())),
                _ => (preview.data(), None),
            }
        };

        panes::draw_preview(
            frame,
            PaneContext {
                area: chunks[pane_idx],
                block: widgets::get_pane_block("Preview", app),
                border_type,
                accent_style,
                styles: preview_pane_styles,
                highlight_symbol: "",
                padding_str,
                show_icons: display_cfg.icons(),
                show_marker: display_cfg.dir_marker(),
            },
            preview_data,
            selected_idx,
            PreviewOptions {
                use_underline: display_cfg.preview_underline(),
                underline_match_text: display_cfg.preview_underline_color(),
                underline_style: theme_cfg.underline_style(),
            },
            &preview_markers,
        );
    }

    widgets::draw_status_bar(frame, app, widgets::StatusPosition::Header, clipboard);
    widgets::draw_status_bar(frame, app, widgets::StatusPosition::Footer, clipboard);
    render_overlays(frame, app, accent_style);
}

/// Returns the rectangular areas for all active panes, given the current configuration
///
/// The result is used for positioning file navigation, parent and preview panes in the layout.
/// Handles separators and dynamic ratios.
pub(crate) fn layout_chunks(size: Rect, app: &AppState) -> Vec<Rect> {
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

/// Renders the root block and header (if applicable) around the main content area.
fn render_root_and_header(frame: &mut Frame, app: &AppState, area: Rect) -> Rect {
    let cfg = app.config();
    let display_cfg = cfg.display();
    let theme_cfg = cfg.theme();
    let path_str = app.nav().display_path();
    let border_type = display_cfg.border_shape().as_border_type();

    if display_cfg.is_unified() {
        let mut outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme_cfg.accent_style())
            .border_type(border_type);

        if display_cfg.titles() {
            outer_block = outer_block.title(Line::from(vec![
                Span::raw(" "),
                Span::styled(path_str, theme_cfg.path_style()),
                Span::raw(" "),
            ]));
        }

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);
        inner
    } else {
        let header_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(path_str, theme_cfg.path_style()),
                Span::raw(" "),
            ])),
            header_layout[0],
        );
        header_layout[1]
    }
}

/// Renders a vertical separator line at the specified x-coordinate within the root area.
fn render_separator(frame: &mut Frame, x: u16, root_area: Rect, style: Style) {
    widgets::draw_separator(
        frame,
        Rect {
            x,
            y: root_area.y,
            width: 1,
            height: root_area.height,
        },
        style,
    );
}

/// Renders any active overlays such as input dialogs or message boxes.
///
/// Calls appropriate widget drawing functions based on the current overlays.
fn render_overlays(frame: &mut Frame, app: &AppState, accent_style: Style) {
    if let ActionMode::Input { mode, .. } = app.actions().mode() {
        if *mode != InputMode::Find {
            widgets::draw_input_dialog(frame, app, accent_style);
        } else {
            widgets::draw_find_dialog(frame, app, accent_style);
        }
    }

    for overlay in app.overlays().iter() {
        match overlay {
            Overlay::ShowInfo { info } => {
                widgets::draw_show_info_dialog(frame, app, accent_style, info);
            }
            Overlay::Message { text } => {
                widgets::draw_message_overlay(frame, app, accent_style, text);
            }
            Overlay::PrefixHelp => {
                widgets::draw_prefix_help_overlay(frame, app, accent_style);
            }
            Overlay::KeybindHelp => {
                widgets::draw_keybind_help(frame, app, accent_style);
            }
        }
    }
}

/// Helper function to calculate and return layout metrics
///
/// Used to store pane widths and heights in the AppState for later use.
fn calculate_layout_metrics(area: Rect, app: &AppState) -> LayoutMetrics {
    let chunks = layout_chunks(area, app);
    let mut metrics = LayoutMetrics::default();
    let display_cfg = app.config().display();

    let mut idx = 0;
    let has_sep = display_cfg.separators() && !display_cfg.is_split();

    // Helper to get inner width and height of a Rect, accounting for borders
    let get_inner = |rect: Rect| {
        let width = if display_cfg.is_split() || display_cfg.is_unified() {
            rect.width.saturating_sub(2)
        } else {
            rect.width
        };

        let height = rect.height.saturating_sub(2);
        (width as usize, height as usize)
    };

    if display_cfg.parent() && idx < chunks.len() {
        metrics.parent_width = get_inner(chunks[idx]).0;
        idx += if has_sep { 2 } else { 1 };
    }

    if idx < chunks.len() {
        metrics.main_width = get_inner(chunks[idx]).0;
        idx += if has_sep && display_cfg.preview() {
            2
        } else {
            1
        };
    }

    if display_cfg.preview() && idx < chunks.len() {
        let (width, height) = get_inner(chunks[idx]);
        metrics.preview_width = width;
        metrics.preview_height = height;
    }
    metrics
}

/// render integration tests
#[cfg(test)]
mod tests {
    use super::*;

    use crate::Config;
    use crate::config::load::RawConfig;
    use std::error;

    #[test]
    fn layout_chunks_with_config() -> Result<(), Box<dyn error::Error>> {
        let size = Rect::new(0, 0, 100, 10);

        let toml_content = r#"
            [display]
            parent = true
            preview = true
            separators = false

            [display.layout]
            parent = 50
            main = 50
            preview = 50
        "#;

        let raw: RawConfig = toml::from_str(toml_content)?;
        let config = Config::from(raw);

        let app = AppState::new(&config).expect("Failed to create AppState");

        let chunks = layout_chunks(size, &app);

        assert_eq!(chunks.len(), 3);
        let total_width: u16 = chunks.iter().map(|c| c.width).sum();

        assert!(total_width <= 100);
        assert!(chunks[0].width >= 33 && chunks[0].width <= 34);
        Ok(())
    }
}
