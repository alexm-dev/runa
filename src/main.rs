mod app;
mod config;
mod file_manager;
mod formatter;
mod terminal;

use config::Config;

fn main() -> std::io::Result<()> {
    let config = Config::load("runner.toml");
    let mut app = app::AppState::new(&config)?;
    terminal::run_terminal(&mut app)
}
