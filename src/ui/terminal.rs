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

pub(crate) const POLL_INTERVAL: Duration = Duration::from_millis(16);

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
fn event_loop<B: Backend + io::Write>(
    terminal: &mut Terminal<B>,
    root: &mut RunaRoot,
) -> io::Result<()>
where
    io::Error: From<<B as Backend>::Error>,
{
    loop {
        let mut changed = root.update();

        changed |= root.container.current_mut().tick(&root.workers);

        if changed {
            root.sync_watch();

            if let AppContainer::Tabs(tabs) = &mut root.container {
                tabs.sync_tab_line();
            }

            draw(terminal, root)?;
        }

        // Event Polling
        if event::poll(POLL_INTERVAL)? {
            match event::read()? {
                // handle keypress
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let result = root.container.current_mut().handle_keypress(
                        key,
                        &root.workers,
                        &mut root.clipboard,
                    );

                    match result {
                        KeypressResult::Quit => break,
                        KeypressResult::OpenedEditor | KeypressResult::Recovered => {
                            execute!(
                                terminal.backend_mut(),
                                LeaveAlternateScreen,
                                EnterAlternateScreen,
                                Hide,
                            )?;
                            terminal.clear()?;
                        }
                        KeypressResult::UiReload if root.reload_ui(terminal.backend_mut())? => {
                            terminal.clear()?;
                        }
                        KeypressResult::Tab(tab_act) => {
                            if let KeypressResult::Quit =
                                app::handle_tab_action(&root.workers, &mut root.container, tab_act)
                            {
                                break;
                            }
                        }
                        KeypressResult::Sort(config) => {
                            app::handle_sort_action(&mut root.container, config);
                        }
                        _ => {}
                    }
                    // Redraw after state change
                    draw(terminal, root)?;
                }

                // handle resize
                Event::Resize(_, _) => draw(terminal, root)?,

                _ => {}
            }
        }
    }
    Ok(())
}

/// Draws the active app state to the terminal.
fn draw<B>(terminal: &mut Terminal<B>, root: &mut RunaRoot) -> io::Result<()>
where
    B: Backend,
    io::Error: From<<B as Backend>::Error>,
{
    terminal.draw(|f| {
        ui::render(
            f,
            root.container.current_mut(),
            &root.workers,
            &mut root.clipboard,
        );
    })?;
    Ok(())
}
