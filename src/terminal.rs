use crate::app::{AppState, KeypressResult};
use crate::utils::parse_color;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
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

fn event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let size = f.area();
            let cfg = app.config();
            let mut block = Block::default();

            if cfg.display().borders() {
                block = block.borders(Borders::ALL);
            }

            let accent_color = parse_color(cfg.theme().accent_color());

            let items: Vec<ListItem> = app
                .visible_entries()
                .iter()
                .map(|e| {
                    let dir_name = if e.is_dir() && app.config().display().show_dir_marker() {
                        format!("{}{}", e.name().to_string_lossy(), "/")
                    } else {
                        e.name().to_string_lossy().into_owned()
                    };
                    ListItem::new(dir_name)
                })
                .collect();

            let mut state = ListState::default();
            if app.has_visible_entries() {
                state.select(app.visible_selected());
            }

            let list = List::new(items)
                .block(block)
                .highlight_style(
                    Style::default()
                        .fg(accent_color)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            f.render_stateful_widget(list, size, &mut state);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind != KeyEventKind::Press {
                    continue;
                }
                let key_str = keycode_to_str(&key_event.code);
                if !key_str.is_empty() {
                    let result = app.handle_keypress(key_str);
                    if let KeypressResult::OpenedEditor = result {
                        terminal.clear()?;
                    }
                    if let KeypressResult::Quit = result {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
