//! Helpers for runa
//!
//! Holds the min, default and the max find result numbers which is uses throughout runa.

use crate::config::Editor;
use ratatui::style::Color;
use std::io;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};

/// The minimum results which is set to if the maximum is overset in the runa.toml.
pub const MIN_FIND_RESULTS: usize = 15;
/// The default find results. Can be overwritten in the runa.toml.
pub const DEFAULT_FIND_RESULTS: usize = 2000;
/// The maximum find result limit which is possible.
/// Can be set higher, but better to set it to a big limit instead of usize::MAX
pub const MAX_FIND_RESULTS_LIMIT: usize = 1000000;

/// Parses a string (color name or hex) into a ratatui::style::color
///
/// Supports standard names (red, green, etc.) as well as hex values (#RRGGBB or #RGB)
pub fn parse_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "default" | "reset" => Color::Reset,
        "yellow" => Color::Yellow,
        "red" => Color::Red,
        "blue" => Color::Blue,
        "green" => Color::Green,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "black" => Color::Black,
        "gray" => Color::Gray,
        "darkgray" => Color::DarkGray,
        _ => {
            if let Some(color) = s.strip_prefix('#') {
                match color.len() {
                    6 => {
                        if let Ok(rgb) = u32::from_str_radix(color, 16) {
                            return Color::Rgb(
                                ((rgb >> 16) & 0xFF) as u8,
                                ((rgb >> 8) & 0xFF) as u8,
                                (rgb & 0xFF) as u8,
                            );
                        }
                    }
                    3 => {
                        let expanded = color
                            .chars()
                            .map(|c| format!("{}{}", c, c))
                            .collect::<String>();
                        if let Ok(rgb) = u32::from_str_radix(&expanded, 16) {
                            return Color::Rgb(
                                ((rgb >> 16) & 0xFF) as u8,
                                ((rgb >> 8) & 0xFF) as u8,
                                (rgb & 0xFF) as u8,
                            );
                        }
                    }
                    _ => {}
                }
            }
            // fallback
            Color::Reset
        }
    }
}

/// Opens a specified path/file in the configured editor ("nvim" or "vim" etc.).
///
/// Temporary disables raw mode and exits alternate sceen while the editor runs.
/// On return, restores raw mode and alternate sceen.
pub fn open_in_editor(editor: &Editor, file_path: &std::path::Path) -> std::io::Result<()> {
    use crossterm::{
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    };

    let mut stdout = io::stdout();
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen)?;

    let status = std::process::Command::new(editor.cmd())
        .arg(file_path)
        .status();

    execute!(io::stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    status.map(|_| ())
}

/// Finds the next available filename by appending _1, _2, etc. if the target exists
///
/// Example: "notes.txt" -> "notes_1.txt"
pub fn get_unused_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let name = path.file_name().unwrap_or_default();

    let stem = Path::new(name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();

    let ext = Path::new(name)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    let mut counter = 1;
    loop {
        let new_name = format!("{}_{}{}", stem, counter, ext);
        let target = parent.join(new_name);
        if !target.exists() {
            return target;
        }
        counter += 1;
    }
}

/// Util function to shorten home directory to ~.
/// Is used by the path_str in the ui.rs render function.
pub fn shorten_home_path<P: AsRef<Path>>(path: P) -> String {
    let path = path.as_ref();
    if let Some(home_dir) = dirs::home_dir()
        && let Ok(stripped) = path.strip_prefix(&home_dir)
    {
        if stripped.as_os_str().is_empty() {
            return "~".to_string();
        } else {
            let mut short = stripped.display().to_string();
            if short.starts_with(MAIN_SEPARATOR) {
                short.remove(0);
            }
            return format!("~{}{}", MAIN_SEPARATOR, short);
        }
    }
    path.display().to_string()
}

/// Safely clamp the find result numbers.
///
/// If the clamped value does not match the set [MAX_FIND_RESULTS_LIMIT] then its invalid and its
/// set to the [MIN_FIND_RESULTS] instead.
pub fn clamp_find_results(value: usize) -> usize {
    let clamped = value.clamp(MIN_FIND_RESULTS, MAX_FIND_RESULTS_LIMIT);
    if clamped != value {
        eprintln!(
            "[Warning] max_find_results={} out of range ({}..={}), clamped to {}",
            value, MIN_FIND_RESULTS, MAX_FIND_RESULTS_LIMIT, clamped
        );
    }
    clamped
}
