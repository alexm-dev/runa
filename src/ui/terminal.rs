//! Terminal rendering and event loop for runa.
//!
//! Handles setup/teardown of raw mode, alternate screen, redraws,
//! and events (keypress, resize) to app logic.

use std::{io, time::Duration};

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};

use crate::app::{self, AppContainer, KeypressResult, RunaRoot};
use crate::ui;

/// Initializes the terminal in raw mode and alternate sceen and runs the main event loop.
///
/// Blocks until quit. Handles all input and UI rendering.
/// Returns a error if terminal setup or teardown fails
///
/// Returns an std::io::Error if terminal setup or teardown fails.
pub(crate) fn run_terminal(root: &mut RunaRoot) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = event_loop(&mut terminal, root);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;
    result
}

/// Main event loop of runa: draws UI, polls for events and dispatches them to the app.
/// Returns on quit
fn event_loop<B: Backend>(terminal: &mut Terminal<B>, root: &mut RunaRoot) -> io::Result<()>
where
    io::Error: From<<B as Backend>::Error>,
{
    loop {
        let mut changed = root.update();

        let (container, workers, clipboard) = root.parts();

        changed |= match container {
            AppContainer::Single(app) => app.tick(workers),
            AppContainer::Tabs(tabs) => tabs.current_tab_mut().tick(workers),
        };

        if changed {
            if let AppContainer::Tabs(tabs) = container {
                tabs.sync_tab_line();
            }

            terminal.draw(|f| match container {
                AppContainer::Single(app) => ui::render(f, app, workers, clipboard),
                AppContainer::Tabs(tabs) => {
                    ui::render(f, tabs.current_tab_mut(), workers, clipboard)
                }
            })?;
        }

        // Event Polling
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                // handle keypress
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let result = match container {
                        AppContainer::Single(app) => app.handle_keypress(key, workers, clipboard),
                        AppContainer::Tabs(tabs) => tabs
                            .current_tab_mut()
                            .handle_keypress(key, workers, clipboard),
                    };

                    match result {
                        KeypressResult::Quit => break,
                        KeypressResult::OpenedEditor | KeypressResult::Recovered => {
                            // full clear/reset
                            terminal.clear()?;
                        }
                        KeypressResult::Tab(tab_act) => {
                            if let KeypressResult::Quit =
                                app::handle_tab_action(workers, container, tab_act)
                            {
                                break;
                            }
                        }
                        KeypressResult::Sort(config) => {
                            app::handle_sort_action(container, config);
                        }
                        _ => {}
                    }
                    // Redraw after state change
                    terminal.draw(|f| match container {
                        AppContainer::Single(app) => ui::render(f, app, workers, clipboard),
                        AppContainer::Tabs(tabs) => {
                            ui::render(f, tabs.current_tab_mut(), workers, clipboard)
                        }
                    })?;
                }

                // handle resize
                Event::Resize(_, _) => {
                    terminal.draw(|f| match container {
                        AppContainer::Single(app) => ui::render(f, app, workers, clipboard),
                        AppContainer::Tabs(tabs) => {
                            ui::render(f, tabs.current_tab_mut(), workers, clipboard)
                        }
                    })?;
                }

                _ => {}
            }
        }
    }
    Ok(())
}
