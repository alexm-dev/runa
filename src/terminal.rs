//! Terminal rendering and event loop for runa.
//!
//! Handles setup/teardown of raw mode, alternate screen, redraws,
//! and events (keypress, resize) to app logic.

use crate::app::{AppState, KeypressResult};
use crate::ui;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};
use std::{io, time::Duration};

/// Initializes the terminal in raw mode and alternate sceen and runs the main event loop.
///
/// Blocks until quit. Handles all input and UI rendering.
/// Returns a error if terminal setup or teardown fails
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

/// Main event loop of runa: draws UI, polls for events and dispatches them to the app.
/// Returns on quit
fn event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
) -> io::Result<()>
where
    io::Error: From<<B as Backend>::Error>,
{
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
