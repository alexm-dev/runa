//! Terminal rendering and event loop for runa.
//!
//! Handles setup/teardown of raw mode, alternate screen, redraws,
//! and events (keypress, resize) to app logic.

use crate::app::{AppContainer, KeypressResult, RunaRoot, handle_tab_action};
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
pub(crate) fn run_terminal(root: &mut RunaRoot) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = event_loop(&mut terminal, root);

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
        let mut changed = root.update();

        changed |= match &mut root.container {
            AppContainer::Single(app) => app.tick(&root.workers),
            AppContainer::Tabs(tabs) => tabs.current_tab_mut().tick(&root.workers),
        };

        if changed {
            terminal.draw(|f| match &mut root.container {
                AppContainer::Single(app) => ui::render(f, app, &root.workers, &mut root.clipboard),
                AppContainer::Tabs(tabs) => ui::render(
                    f,
                    tabs.current_tab_mut(),
                    &root.workers,
                    &mut root.clipboard,
                ),
            })?;
        }

        // Event Polling
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                // handle keypress
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let result = match &mut root.container {
                        AppContainer::Single(app) => {
                            app.handle_keypress(key, &root.workers, &mut root.clipboard)
                        }
                        AppContainer::Tabs(tabs) => tabs.current_tab_mut().handle_keypress(
                            key,
                            &root.workers,
                            &mut root.clipboard,
                        ),
                    };

                    match result {
                        KeypressResult::Quit => break,
                        KeypressResult::OpenedEditor | KeypressResult::Recovered => {
                            // full clear/reset
                            terminal.clear()?;
                        }
                        KeypressResult::Tab(tab_act) => {
                            if let KeypressResult::Quit =
                                handle_tab_action(&root.workers, &mut root.container, tab_act)
                            {
                                break;
                            }
                        }
                        _ => {}
                    }
                    // Redraw after state change
                    terminal.draw(|f| match &mut root.container {
                        AppContainer::Single(app) => {
                            ui::render(f, app, &root.workers, &mut root.clipboard)
                        }
                        AppContainer::Tabs(tabs) => ui::render(
                            f,
                            tabs.current_tab_mut(),
                            &root.workers,
                            &mut root.clipboard,
                        ),
                    })?;
                }

                // handle resize
                Event::Resize(_, _) => {
                    terminal.draw(|f| match &mut root.container {
                        AppContainer::Single(app) => {
                            ui::render(f, app, &root.workers, &mut root.clipboard)
                        }
                        AppContainer::Tabs(tabs) => ui::render(
                            f,
                            tabs.current_tab_mut(),
                            &root.workers,
                            &mut root.clipboard,
                        ),
                    })?;
                }

                _ => {}
            }
        }
    }
    Ok(())
}
