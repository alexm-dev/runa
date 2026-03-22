//! main.rs
//! Entry point for runa

pub(crate) mod app;
pub(crate) mod config;
pub(crate) mod core;
pub(crate) mod ui;
pub(crate) mod utils;

use crate::config::Config;
use crate::core::terminal;
use crate::core::worker::Workers;
use crate::utils::cli::{CliAction, handle_args};
use crate::utils::{is_hardened_directory, resolve_initial_dir};

fn startup_container<'a>(
    config: &'a Config,
    workers: &Workers,
) -> std::io::Result<app::AppContainer<'a>> {
    let startup_tabs = config.general().startup_tabs();

    if startup_tabs.is_empty() {
        let mut app = app::AppState::new(config)?;
        app.initialize(workers, None);
        return Ok(app::AppContainer::Single(Box::new(app)));
    }

    let mut tabs = Vec::with_capacity(startup_tabs.len());

    for path in startup_tabs {
        let path_str = path.to_string_lossy();
        if path_str == "." || path_str == "cwd" {
            if let Ok(mut state) = app::AppState::new(config) {
                state.initialize(workers, None);
                tabs.push(state);
            }
            continue;
        }

        if is_hardened_directory(path)
            && let Ok(mut state) = app::AppState::from_dir(config, path)
        {
            state.initialize(workers, None);
            tabs.push(state);
        }
    }

    match tabs.len() {
        0 => {
            let mut app = app::AppState::new(config)?;
            app.initialize(workers, None);
            Ok(app::AppContainer::Single(Box::new(app)))
        }
        1 => Ok(app::AppContainer::Single(Box::new(tabs.pop().unwrap()))),
        _ => Ok(app::AppContainer::Tabs(app::tab::TabManager::from_vec(
            tabs,
        ))),
    }
}

fn main() -> std::io::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = crossterm::execute!(
            stdout,
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        );

        eprintln!("\n[runa] Error occurred: {}", info);

        #[cfg(debug_assertions)]
        {
            let bt = std::backtrace::Backtrace::force_capture();
            eprintln!("\nStack Backtrace:\n{}", bt);
        }
    }));

    let action = handle_args();

    if let CliAction::Exit = action {
        return Ok(());
    }

    let config = Config::load();

    let initial_path = match action {
        CliAction::RunApp => None,
        CliAction::RunAppAtPath(path_arg) => {
            let target = resolve_initial_dir(&path_arg);

            if !is_hardened_directory(&target) {
                eprintln!("\n[runa] Error: Path '{}' cannot be opened.", path_arg);
                std::process::exit(1);
            }
            Some(target)
        }
        _ => unreachable!(),
    };

    let workers = Workers::spawn();

    let container = match initial_path {
        Some(path) => {
            let mut app = app::AppState::from_dir(&config, &path)?;
            app.initialize(&workers, None);
            app::AppContainer::Single(Box::new(app))
        }
        None => startup_container(&config, &workers)?,
    };

    let mut runa = app::RunaRoot {
        container,
        clipboard: app::Clipboard::default(),
        workers,
    };

    terminal::run_terminal(&mut runa)
}
