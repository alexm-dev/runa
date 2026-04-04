//! Command-line argument parsing and help for runa.
//!
//! This module handles all CLI flag parsing used for config initialization and help.
//!
//! When invoked with no args/flags (rn), runa simply launches the TUI

use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use crate::config::Config;

const CONFIG_HELP: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/config/runa_full.toml"
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
        return match first_arg.as_str() {
            "--version" | "-v" => {
                if check_no_extra_args(first_arg, &args) {
                    return CliAction::Exit;
                }
                print_version();
                CliAction::Exit
            }
            "-h" | "--help" => {
                if check_no_extra_args(first_arg, &args) {
                    return CliAction::Exit;
                }
                print_help();
                CliAction::Exit
            }
            "--init" => {
                if check_no_extra_args(first_arg, &args) {
                    return CliAction::Exit;
                }
                if let Err(e) = Config::generate_default(&config_path, true) {
                    eprintln!("Error: {}", e);
                }
                CliAction::Exit
            }
            "--init-full" => {
                if check_no_extra_args(first_arg, &args) {
                    return CliAction::Exit;
                }
                if let Err(e) = Config::generate_default(&config_path, false) {
                    eprintln!("Error: {}", e);
                }
                CliAction::Exit
            }
            "--config-help" => {
                let section_arg = args.get(2).map(|s| s.as_str());

                let result = match section_arg {
                    Some(arg) if arg.contains('/') || arg.contains('\\') => {
                        eprintln!("[runa] '{}' looks like a path, not a config section.", arg);
                        Ok(())
                    }
                    Some(section) if args.len() == 3 => print_config_help(Some(section)),
                    None => print_config_help(None),
                    _ => {
                        eprintln!(
                            "[runa] Error: --config-help only accepts one optional SECTION argument."
                        );
                        Ok(())
                    }
                };

                if let Err(e) = result
                    && e.kind() != std::io::ErrorKind::BrokenPipe
                {
                    eprintln!("[runa] Error displaying config help: {}", e);
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
      --config-help       Display all the configuration options. Use '--config-help [SECTION]'
                          to show a specific part (e.g. 'theme' or 'keys')
  -h, --help              Print help information
  -v, --version           Display the current installed version of runa

ENVIRONMENT:
  RUNA_CONFIG             Override the default config path
"#
    );
}

fn print_config_help(target_section: Option<&str>) -> io::Result<()> {
    let config = CONFIG_HELP;
    let indent = "    ";

    let stdout = io::stdout();
    let mut handle = BufWriter::new(stdout.lock());

    let target = target_section.map(|s| s.trim().to_ascii_lowercase());
    let mut inside = target.is_none();
    let mut found = target.is_none();
    let mut head_buffer: Vec<&str> = Vec::new();
    let mut body_buffer: Vec<&str> = Vec::new();

    writeln!(handle)?;

    for line in config.lines() {
        let trimmed = line.trim();
        let is_active_header = trimmed.starts_with('[');

        if is_active_header {
            let header_part = trimmed.split('#').next().unwrap_or("").trim();
            let hdr_name = header_part
                .trim_matches(|c| c == '[' || c == ']' || c == ' ')
                .to_ascii_lowercase();

            if let Some(ref t) = target {
                if &hdr_name == t {
                    for h in &head_buffer {
                        writeln!(handle, "{}{}", indent, h)?;
                    }
                    writeln!(handle, "{}{}", indent, line)?;
                    inside = true;
                    found = true;
                    head_buffer.clear();
                    continue;
                } else if inside {
                    break;
                }
            }
        }

        if inside {
            let is_setting = trimmed.contains('=') && !trimmed.starts_with('#');
            if is_setting || target.is_none() {
                for b in &body_buffer {
                    writeln!(handle, "{}{}", indent, b)?;
                }
                body_buffer.clear();
                writeln!(handle, "{}{}", indent, line)?;
            } else {
                body_buffer.push(line);
            }
        } else {
            if trimmed.is_empty() {
                head_buffer.clear();
            } else if trimmed.starts_with('#') {
                head_buffer.push(line);
            } else {
                head_buffer.clear();
            }
        }
    }

    if inside {
        for b in &body_buffer {
            writeln!(handle, "{}{}", indent, b)?;
        }
    }

    if !found {
        writeln!(
            handle,
            "[runa] Section '{}' not found.",
            target.unwrap_or_default()
        )?;
        writeln!(handle)?;
        writeln!(
            handle,
            "USAGE:
rn --config-help           (Show all options)
rn --config-help [SECTION] (Show specific section, e.g., 'theme')

TIP:
   Try running without arguments to see the full list of available sections.",
        )?;
    }

    writeln!(handle)?;
    handle.flush()?;

    Ok(())
}

fn check_no_extra_args(flag: &str, args: &[String]) -> bool {
    if args.len() > 2 {
        eprintln!("[runa] Error: {} does not take arguments.", flag);
        true
    } else {
        false
    }
}
