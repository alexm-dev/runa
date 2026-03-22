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
    cli_paths: Option<Vec<String>>,
) -> std::io::Result<app::AppContainer<'a>> {
    let raw_paths: Vec<String> = match cli_paths {
        Some(paths) => paths,
        None => config
            .general()
            .startup_tabs()
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
    };

    if raw_paths.is_empty() {
        let mut app = app::AppState::new(config)?;
        app.initialize(workers, None);
        return Ok(app::AppContainer::Single(Box::new(app)));
    }

    let mut tabs = Vec::with_capacity(raw_paths.len());
    for path_str in raw_paths {
        if tabs.len() >= 9 {
            break;
        }

        if path_str == "." || path_str == "cwd" {
            if let Ok(mut state) = app::AppState::new(config) {
                state.initialize(workers, None);
                tabs.push(state);
            }
        } else {
            let target = resolve_initial_dir(&path_str);
            if is_hardened_directory(&target)
                && let Ok(mut state) = app::AppState::from_dir(config, &target)
            {
                state.initialize(workers, None);
                tabs.push(state);
            }
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

    let cli_paths = match action {
        CliAction::RunApp => None,
        CliAction::RunAppAtPath(paths) => Some(paths),
        _ => unreachable!(),
    };

    let workers = Workers::spawn();

    let container = startup_container(&config, &workers, cli_paths)?;

    let mut runa = app::RunaRoot {
        container,
        clipboard: app::Clipboard::default(),
        workers,
    };

    terminal::run_terminal(&mut runa)
}
