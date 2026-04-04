//! Command-line argument parsing and help for runa.
//!
//! This module handles all CLI flag parsing used for config initialization and help.
//!
//! When invoked with no args/flags (rn), runa simply launches the TUI

use std::path::PathBuf;

use crate::config::Config;

const CONFIG_HELP: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/cli/config_help.txt"
));
const KEYBINDS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/cli/keybinds.txt"
));

pub(crate) enum CliAction {
    RunApp,
    RunAppAtPath(Vec<PathBuf>),
    Exit,
}

pub(crate) fn handle_args() -> CliAction {
    let mut args: Vec<String> = std::env::args().collect();
    let config_path = Config::default_path();

    if args.len() < 2 {
        return CliAction::RunApp;
    }

    let first_arg = &args[1];
    if first_arg.starts_with("-") {
        if args.len() > 2 {
            eprintln!("[runa] Error: Options (flags) cannot be combined with other arguments.");
            eprintln!("Usage: rn [OPTION] OR rn [PATH]...");
            return CliAction::Exit;
        }

        return match first_arg.as_str() {
            "--version" | "-v" => {
                print_version();
                CliAction::Exit
            }
            "-h" | "--help" => {
                print_help();
                CliAction::Exit
            }
            "--config-help" => {
                print_config_help();
                CliAction::Exit
            }
            "--keybinds" | "--keybind" | "--key" => {
                print_keybinds();
                CliAction::Exit
            }
            "--init" => {
                if let Err(e) = Config::generate_default(&config_path, true) {
                    eprintln!("Error: {}", e);
                }
                CliAction::Exit
            }
            "--init-full" => {
                if let Err(e) = Config::generate_default(&config_path, false) {
                    eprintln!("Error: {}", e);
                }
                CliAction::Exit
            }
            _ => {
                eprintln!("Unknown argument: {}", first_arg);
                eprintln!("Try --help for available options");
                CliAction::Exit
            }
        };
    }

    let mut paths: Vec<PathBuf> = args
        .drain(1..)
        .filter(|a| !a.trim().is_empty())
        .map(PathBuf::from)
        .collect();

    if paths.is_empty() {
        return CliAction::RunApp;
    }

    if paths.iter().any(|p| p.to_string_lossy().starts_with('-')) {
        eprintln!(
            "[runa] Error: Options must be placed at the start and cannot be combined with paths."
        );
        return CliAction::Exit;
    }

    if paths.len() > 9 {
        eprintln!("[runa] Note: runa supports a maximum of 9 tabs. Extra paths will be ignored.");
        paths.truncate(9);
    }

    CliAction::RunAppAtPath(paths)
}

fn print_version() {
    println!("runa {}", env!("CARGO_PKG_VERSION"));
}

fn print_help() {
    println!(
        r#"runa - A fast and lightweight terminal file manager written in Rust

USAGE:
  rn [PATH]...

PATH:
  Directory to open (defaults to current directory)
  Pass multiple paths to open each in a separate tab

OPTIONS:
      --init              Generate a minimal default configuration
      --init-full         Generate the full configuration with all options
      --config-help       Display all the configuration options
      --keybinds          Display all the default keybinds
  -h, --help              Print help information
  -v, --version           Display the current installed version of runa

ENVIRONMENT:
  RUNA_CONFIG             Override the default config path
"#
    );
}

fn print_keybinds() {
    println!("{}", KEYBINDS);
}

fn print_config_help() {
    println!("{}{}", CONFIG_HELP, KEYBINDS);
}
