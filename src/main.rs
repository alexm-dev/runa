mod app;
mod config;
mod file_manager;
mod formatter;
mod keymap;
mod terminal;
mod utils;

use config::Config;

fn handle_default_confg() -> std::io::Result<bool> {
    let args: Vec<String> = std::env::args().collect();
    let config_path = Config::default_path();

    if args.len() > 1 && args[1] == "--init" {
        match Config::generate_default(&config_path) {
            Ok(_) => (),
            Err(e) => eprintln!("{}", e),
        }
        return Ok(true);
    }
    Ok(false)
}

fn main() -> std::io::Result<()> {
    if handle_default_confg()? {
        return Ok(());
    }

    let config_path = Config::default_path();
    let config = Config::load(&config_path);
    let mut app = app::AppState::new(&config)?;
    terminal::run_terminal(&mut app)
}
