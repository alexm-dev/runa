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

fn main() -> std::io::Result<()> {
    match handle_args() {
        CliAction::Exit => return Ok(()),
        CliAction::RunApp => (),
    }

    let config = Config::load();
    let mut app = app::AppState::new(&config)?;
    terminal::run_terminal(&mut app)
}
