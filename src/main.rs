//! main.rs
//! Entry point for runa

pub(crate) mod app;
pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod core;
pub(crate) mod ui;
pub(crate) mod utils;

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use crate::cli::{CliAction, handle_args};
use crate::config::Config;
use crate::core::workers::Workers;
use crate::utils::path::{resolve_initial_dir, validate_path};

#[inline(never)]
fn install_panic_hook() {
    std::panic::set_hook(Box::new(handle_panic));
}

#[inline(never)]
fn handle_panic(info: &std::panic::PanicHookInfo<'_>) {
    let _ = crossterm::terminal::disable_raw_mode();
    let mut stdout = io::stdout();
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
}

#[inline(never)]
fn load_config_or_default() -> Config {
    Config::load().unwrap_or_else(|e| {
        eprintln!("[runa] Config error: {}", e);
        Config::default()
    })
}

#[inline(never)]
fn exit_with_startup_error(error: io::Error) -> ! {
    eprintln!("[runa] Error: {}", error);
    std::process::exit(1);
}

fn startup_container(
    config: Arc<Config>,
    workers: &Workers,
    cli_paths: Option<Vec<PathBuf>>,
) -> io::Result<app::AppContainer> {
    let is_cli_request = cli_paths.is_some();
    let paths: Vec<PathBuf> = match cli_paths {
        Some(paths) => paths,
        None => config.general().startup_tabs().to_vec(),
    };

    if paths.is_empty() {
        let mut app = app::AppState::new(Arc::clone(&config))?;
        app.initialize(workers, None);
        return Ok(app::AppContainer::Single(Box::new(app)));
    }

    let mut tabs = Vec::with_capacity(paths.len());
    for path in paths {
        if tabs.len() >= 9 {
            break;
        }

        let path_str = path.to_string_lossy();
        if path_str == "." || path_str == "cwd" {
            if let Ok(mut state) = app::AppState::new(Arc::clone(&config)) {
                state.initialize(workers, None);
                tabs.push(state);
            }
        } else {
            let target = resolve_initial_dir(&path);

            if let Err(e) = validate_path(&target) {
                return Err(io::Error::new(e.kind(), format!("{}: '{}'", e, path_str)));
            }

            let mut state = app::AppState::from_dir(Arc::clone(&config), &target)?;
            state.initialize(workers, None);
            tabs.push(state);
        }
    }

    match tabs.len() {
        0 => {
            if is_cli_request {
                return Err(io::Error::other("The provided paths could not be opened"));
            }
            let mut app = app::AppState::new(Arc::clone(&config))?;
            app.initialize(workers, None);
            Ok(app::AppContainer::Single(Box::new(app)))
        }
        1 => {
            let state = tabs.into_iter().next().ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "Failed to initialize the path")
            })?;
            Ok(app::AppContainer::Single(Box::new(state)))
        }
        _ => Ok(app::AppContainer::create_tabs(tabs)),
    }
}

fn main() -> io::Result<()> {
    install_panic_hook();

    let action = handle_args();

    if let CliAction::Exit = action {
        return Ok(());
    }

    let config = load_config_or_default();

    let cli_paths = match action {
        CliAction::RunApp => None,
        CliAction::RunAppAtPath(paths) => Some(paths),
        _ => unreachable!(),
    };

    let workers = Workers::spawn();

    let container = match startup_container(Arc::new(config), &workers, cli_paths) {
        Ok(cont) => cont,
        Err(e) => exit_with_startup_error(e),
    };

    let mut runa = app::RunaRoot::new(container, workers);

    ui::run_terminal(&mut runa)
}
