//! main.rs
//! Entry point for runa

pub(crate) mod app;
pub(crate) mod config;
pub(crate) mod core;
pub(crate) mod ui;
pub(crate) mod utils;

use crate::app::tab::RunaRoot;
use crate::config::Config;
use crate::core::terminal;
use crate::utils::cli::{CliAction, handle_args};
use crate::utils::{is_hardened_directory, resolve_initial_dir};

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

    let app = match initial_path {
        Some(path) => app::AppState::from_dir(&config, &path)?,
        None => app::AppState::new(&config)?,
    };
    let mut root = RunaRoot::Single(Box::new(app));
    terminal::run_terminal(&mut root)
}
