use crate::app::{AppState, KeypressResult};
use crate::ui;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, time::Duration};

pub fn run_terminal(app: &mut AppState) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = event_loop(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    result
}

fn event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
) -> io::Result<()> {
    loop {
        // App Tick
        // If tick returns true, something changed internally that needs a redraw.
        if app.tick() {
            terminal.draw(|f| ui::render(f, app))?;
        }

        // Event Polling
        if event::poll(Duration::from_millis(10))? {
            match event::read()? {
                // handle keypress
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let result = app.handle_keypress(key);

                    match result {
                        KeypressResult::Quit => break,
                        KeypressResult::OpenedEditor => {
                            // full clear/reset
                            terminal.clear()?;
                        }
                        _ => {}
                    }
                    // Redraw after state change
                    terminal.draw(|f| ui::render(f, app))?;
                }

                // handle resize
                Event::Resize(_, _) => {
                    terminal.draw(|f| ui::render(f, app))?;
                }

                _ => {}
            }
        }
    }
    Ok(())
}
