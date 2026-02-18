//! Terminal rendering and event loop for runa.
//!
//! Handles setup/teardown of raw mode, alternate screen, redraws,
//! and events (keypress, resize) to app logic.

use crate::app::{KeypressResult, handle_tab_action, tab::RunaRoot};
use crate::ui;
use crossterm::{
    cursor::{Hide, Show},
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
///
/// Returns an std::io::Error if terminal setup or teardown fails.
pub(crate) fn run_terminal(app: &mut RunaRoot) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = event_loop(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;
    result
}

/// Main event loop of runa: draws UI, polls for events and dispatches them to the app.
/// Returns on quit
fn event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    root: &mut RunaRoot,
) -> io::Result<()>
where
    io::Error: From<<B as Backend>::Error>,
{
    loop {
        // App Tick
        // If tick returns true, something changed internally that needs a redraw.
        let changed = match root {
            RunaRoot::Single(app) => app.tick(),
            RunaRoot::Tabs(tabs) => tabs.current_tab_mut().tick(),
        };

        if changed {
            terminal.draw(|f| match root {
                RunaRoot::Single(app) => ui::render(f, app),
                RunaRoot::Tabs(tabs) => ui::render(f, tabs.current_tab_mut()),
            })?;
        }

        // Event Polling
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                // handle keypress
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let result = match root {
                        RunaRoot::Single(app) => app.handle_keypress(key),
                        RunaRoot::Tabs(tabs) => tabs.current_tab_mut().handle_keypress(key),
                    };

                    match result {
                        KeypressResult::Quit => break,
                        KeypressResult::OpenedEditor | KeypressResult::Recovered => {
                            // full clear/reset
                            terminal.clear()?;
                        }
                        KeypressResult::Tab(tab_act) => {
                            if let KeypressResult::Quit = handle_tab_action(root, tab_act) {
                                break;
                            }
                        }
                        _ => {}
                    }
                    // Redraw after state change
                    terminal.draw(|f| match root {
                        RunaRoot::Single(app) => ui::render(f, app),
                        RunaRoot::Tabs(tabs) => ui::render(f, tabs.current_tab_mut()),
                    })?;
                }

                // handle resize
                Event::Resize(_, _) => {
                    terminal.draw(|f| match root {
                        RunaRoot::Single(app) => ui::render(f, app),
                        RunaRoot::Tabs(tabs) => ui::render(f, tabs.current_tab_mut()),
                    })?;
                }

                _ => {}
            }
        }
    }
    Ok(())
}
