use crate::app::{AppState, KeypressResult};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::{io, time::Duration};

pub fn run_terminal(app: &mut AppState) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(&mut stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn keycode_to_str(key: &KeyCode) -> &'static str {
    match key {
        KeyCode::Char('j') => "j",
        KeyCode::Char('k') => "k",
        KeyCode::Char('h') => "h",
        KeyCode::Char('l') => "l",
        KeyCode::Char('q') => "q",
        KeyCode::Enter | KeyCode::Char('\n') | KeyCode::Char('\r') => "Enter",
        KeyCode::Left => "Left Arrow",
        KeyCode::Right => "Right Arrow",
        KeyCode::Down => "Down Arrow",
        KeyCode::Up => "Up Arrow",
        KeyCode::Esc => "Esc",
        KeyCode::Backspace => "Backspace",
        _ => "",
    }
}

fn layout_chunks(size: Rect, app: &AppState) -> Vec<Rect> {
    let cfg = app.config().display();
    let mut constraints = Vec::new();
    let show_sep = cfg.separators() && !cfg.is_split();

    if cfg.origin() {
        constraints.push(Constraint::Percentage(cfg.origin_ratio()));
        if show_sep {
            constraints.push(Constraint::Length(1));
        }
    }

    constraints.push(Constraint::Percentage(cfg.main_ratio()));

    if cfg.preview() {
        if show_sep {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Percentage(cfg.preview_ratio()));
    }

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(size)
        .to_vec()
}

pub fn draw_separator(frame: &mut Frame, area: Rect, style: Style) {
    let separator = Block::default().borders(Borders::LEFT).border_style(style);
    frame.render_widget(separator, area);
}

pub struct PaneContext<'a> {
    pub area: Rect,
    pub block: Block<'a>,
    pub accent_style: Style,
    pub entry_style: Style,
    pub selection_style: Style,
    pub highlight_symbol: &'a str,
}

fn draw_main_pane(frame: &mut Frame, app: &AppState, context: PaneContext) {
    let show_marker = app.config().display().dir_marker();
    let items: Vec<ListItem> = app
        .visible_entries()
        .iter()
        .map(|e| {
            let text = if e.is_dir() && show_marker {
                e.display_name()
            } else {
                e.name_str()
            };
            ListItem::new(text).style(context.entry_style)
        })
        .collect();

    let mut state = ListState::default();
    if app.has_visible_entries() {
        state.select(app.visible_selected());
    }

    let padding = app.config().display().scroll_padding();

    frame.render_stateful_widget(
        List::new(items)
            .block(context.block.border_style(context.accent_style))
            .highlight_style(context.selection_style.add_modifier(Modifier::BOLD))
            .highlight_symbol(context.highlight_symbol)
            .scroll_padding(padding),
        context.area,
        &mut state,
    );
}

fn draw_preview_pane(
    frame: &mut Frame,
    area: Rect,
    lines: &[String],
    block: Block,
    style: Style,
    highlight_style: Style,
    selected_idx: Option<usize>,
) {
    if lines.is_empty() {
        frame.render_widget(Paragraph::new("").block(block), area);
        return;
    }

    let items: Vec<ListItem> = lines
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut line_style = style;
            if Some(i) == selected_idx {
                line_style = highlight_style.add_modifier(Modifier::BOLD);
            }
            ListItem::new(s.as_str()).style(line_style)
        })
        .collect();

    let mut state = ListState::default();
    if let Some(idx) = selected_idx {
        state.select(Some(idx.min(lines.len().saturating_sub(1))));
    }

    frame.render_stateful_widget(
        List::new(items)
            .block(block)
            .highlight_style(highlight_style),
        area,
        &mut state,
    );
}

fn draw_origin_pane(
    frame: &mut Frame,
    area: Rect,
    lines: &[String],
    block: Block,
    style: Style,
    highlight_style: Style,
    selected_idx: Option<usize>,
) {
    if lines.is_empty() {
        frame.render_widget(Paragraph::new("").block(block), area);
        return;
    }

    let items: Vec<ListItem> = lines
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut line_style = style;
            if Some(i) == selected_idx {
                line_style = highlight_style.add_modifier(Modifier::BOLD);
            }
            ListItem::new(s.as_str()).style(line_style)
        })
        .collect();

    let mut state = ListState::default();

    // This ensures Ratatui scrolls the Parent pane to the correct spot
    state.select(selected_idx.map(|idx| idx.min(lines.len().saturating_sub(1))));

    frame.render_stateful_widget(
        List::new(items)
            .block(block)
            .highlight_style(highlight_style),
        area,
        &mut state,
    );
}

fn render_ui(frame: &mut Frame, app: &AppState) {
    let mut root_area = frame.area();
    let cfg = app.config();
    let display_cfg = cfg.display();
    let theme_cfg = cfg.theme();

    // 0. PRE-CALCULATE COLORS AND SHARED DATA
    // let bg_color = parse_color(theme_cfg.background());
    let accent_style = theme_cfg.accent();
    let entry_style = theme_cfg.entry();
    let selection_style = theme_cfg.selection();
    let origin_style = theme_cfg.origin();
    let preview_style = theme_cfg.preview();
    let separator_style = theme_cfg.separator();

    let show_separators = display_cfg.separators() && !display_cfg.is_split();
    let path_str = app.current_dir().to_string_lossy();

    // 1. HANDLE OUTER BORDER AND TOP PATH
    if display_cfg.is_unified() {
        let mut outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(accent_style);
        if display_cfg.titles() {
            outer_block = outer_block.title(format!(" {} ", path_str));
        }
        frame.render_widget(outer_block, root_area);
        root_area = Block::default().borders(Borders::ALL).inner(root_area);
    } else {
        let header_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(root_area);
        let header = Paragraph::new(format!("   {} ", path_str)).style(accent_style);
        frame.render_widget(header, header_layout[0]);
        root_area = header_layout[1];
    }

    // 2. DEFINE PANE BLOCK GENERATOR
    let use_individual_borders = display_cfg.is_split();
    let get_pane_block = |title: &str| {
        let mut b = Block::default();
        if use_individual_borders {
            b = b.borders(Borders::ALL).border_style(accent_style);
            if display_cfg.titles() {
                b = b.title(format!(" {} ", title));
            }
        }
        b
    };

    // 3. RENDER PANES
    let chunks = layout_chunks(root_area, app);
    let mut pane_idx = 0;

    // ORIGIN PANE
    if display_cfg.origin() && pane_idx < chunks.len() {
        draw_origin_pane(
            frame,
            chunks[pane_idx],
            &app.parent_content,
            get_pane_block("Parent"),
            origin_style,
            selection_style,
            app.parent_selected,
        );
        pane_idx += 1;

        if show_separators && pane_idx < chunks.len() {
            let sep_rect = Rect {
                x: chunks[pane_idx].x,
                y: root_area.y,
                width: 1,
                height: root_area.height,
            };
            draw_separator(frame, sep_rect, separator_style);
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

        let main_ctx = PaneContext {
            area: chunks[pane_idx],
            block: get_pane_block("Files"),
            accent_style,
            entry_style,
            selection_style,
            highlight_symbol: symbol,
        };

        draw_main_pane(frame, app, main_ctx);
        pane_idx += 1;

        if show_separators && display_cfg.preview() && pane_idx < chunks.len() {
            let sep_rect = Rect {
                x: chunks[pane_idx].x,
                y: root_area.y,
                width: 1,
                height: root_area.height,
            };
            draw_separator(frame, sep_rect, separator_style);
            pane_idx += 1;
        }
    }

    // PREVIEW PANE
    if display_cfg.preview() && pane_idx < chunks.len() {
        let is_dir = app
            .visible_entries()
            .get(app.visible_selected().unwrap_or(0))
            .map(|e| e.is_dir())
            .unwrap_or(false);

        draw_preview_pane(
            frame,
            chunks[pane_idx],
            &app.preview_content,
            get_pane_block("Preview"),
            preview_style,
            selection_style,
            if is_dir {
                Some(app.preview_selected)
            } else {
                None
            },
        );
    }
}

fn event_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut AppState) -> io::Result<()> {
    loop {
        if app.tick() {
            terminal.draw(|frame| render_ui(frame, app))?;
        }

        if event::poll(Duration::from_millis(5))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    let key_str = keycode_to_str(&key_event.code);
                    let result = app.handle_keypress(key_str);

                    terminal.draw(|frame| render_ui(frame, app))?;

                    if let KeypressResult::Quit = result {
                        break;
                    }
                    if let KeypressResult::OpenedEditor = result {
                        terminal.clear()?;
                    }
                }
                Event::Resize(_, _) => {
                    terminal.draw(|frame| render_ui(frame, app))?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}
