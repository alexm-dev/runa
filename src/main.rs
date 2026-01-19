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
    match handle_args() {
        CliAction::Exit => Ok(()),
        CliAction::RunApp => {
            let config = Config::load();
            let mut app = app::AppState::new(&config)?;
            terminal::run_terminal(&mut app)
        }
        CliAction::RunAppAtPath(path_arg) => {
            let config = Config::load();
            let expanded = expand_home_path_buf(&path_arg);
            if !is_hardened_directory(&expanded) {
                eprintln!(
                    "Error: Path '{}' cannot be opened (does not exist, is not a directory, or permission denied).",
                    path_arg
                );
                std::process::exit(1);
            }
            let mut app = app::AppState::new_with_path(&config, &expanded)?;
            terminal::run_terminal(&mut app)
        }
    }
}
