//! main.rs
//! Entry point for runa

pub mod app;
pub mod config;
pub mod core;
pub mod ui;
pub mod utils;

use crate::config::Config;
use crate::core::terminal;
use crate::utils::cli::{CliAction, handle_args};
use crate::utils::helpers::{expand_home_path_buf, is_hardened_directory};

fn main() -> std::io::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = crossterm::execute!(
            stdout,
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        );

        eprintln!("\n[runa] Panic occurred: {}", info);

        #[cfg(debug_assertions)]
        {
            let bt = std::backtrace::Backtrace::force_capture();
            eprintln!("\nStack Backtrace:\n{}", bt);
        }

        #[cfg(not(debug_assertions))]
        eprintln!("Run with RUST_BACKTRACE=1 for more details.");
    }));

    let action = handle_args();

    if let CliAction::Exit = action {
        return Ok(());
    }

    let config = Config::load();

    let initial_path = match action {
        CliAction::RunApp => None,
        CliAction::RunAppAtPath(path_arg) => {
            let expanded = expand_home_path_buf(&path_arg);

            if !is_hardened_directory(&expanded) {
                eprintln!("\n[runa] Error: Path '{}' cannot be opened.", path_arg);
                std::process::exit(1);
            }
            Some(expanded)
        }
        _ => unreachable!(),
    };

    let mut app = match initial_path {
        Some(path) => app::AppState::from_dir(&config, &path)?,
        None => app::AppState::new(&config)?,
    };

    terminal::run_terminal(&mut app)
}
