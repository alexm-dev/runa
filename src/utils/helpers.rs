//! Helpers for runa.
//!
//! This module defines constants for the minimum, default, and maximum find result limits used throughout runa.
//! It also provides utility functions:
//! - Color parsing from strings or hex codes
//! - Opening paths/files in the user's chosen editor
//! - Generating unused filenames to prevent accidental overwrite
//! - Displaying home directories as "~" in file paths
//! - Clamping the find result count to safe values
//!
//! These helpers are used throughout runa.

use crate::config::Editor;
use ratatui::style::Color;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::sync::OnceLock;
use std::{fs, io};

/// The minimum results which is set to if the maximum is overset in the runa.toml.
pub const MIN_FIND_RESULTS: usize = 15;
/// The default find results. Can be overwritten in the runa.toml.
pub const DEFAULT_FIND_RESULTS: usize = 2000;
/// The maximum find result limit which is possible.
/// Can be set higher, but better to set it to a big limit instead of usize::MAX
pub const MAX_FIND_RESULTS_LIMIT: usize = 1000000;

/// Shared cache for the home_dir dirs call
static HOME_DIR_CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Thread safe for getting home_dir once.
pub fn get_home() -> Option<&'static PathBuf> {
    HOME_DIR_CACHE.get_or_init(dirs::home_dir).as_ref()
}

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

    let home_dir = get_home();

    if let Some(home) = home_dir
        && let Ok(stripped) = path.strip_prefix(home)
    {
        if stripped.as_os_str().is_empty() {
            return "~".to_string();
        } else {
            return format!("~{}{}", MAIN_SEPARATOR, stripped.display());
        }
    }
    path.display().to_string()
}

pub fn expand_home_path(input: &str) -> String {
    expand_home_path_buf(input).to_string_lossy().to_string()
}

pub fn expand_home_path_buf(input: &str) -> PathBuf {
    let home = get_home();

    if let Some(home) = home {
        if input == "~" {
            return home.clone();
        }

        if let Some(rest) = input.strip_prefix("~/") {
            return home.join(rest);
        }

        #[cfg(windows)]
        if let Some(rest) = input.strip_prefix(r"~\") {
            return home.join(rest);
        }
    }

    #[cfg(windows)]
    {
        if input.len() == 2 && input.ends_with(':') {
            let first_char = input.chars().next().unwrap();
            if first_char.is_ascii_alphabetic() {
                return PathBuf::from(format!(r"{}\", input));
            }
        }
    }

    PathBuf::from(input)
}

pub fn is_hardened_directory(path: &Path) -> bool {
    if !path.exists() || !path.is_dir() {
        return false;
    }

    if std::fs::read_dir(path).is_err() {
        return false;
    }

    if path.components().count() > 255 {
        return false;
    }

    true
}

pub fn clean_display_path(path: &str) -> &str {
    path.strip_prefix(r"\\?\").unwrap_or(path)
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

/// Recursively copies files and directories from `src` to `dest`.
///
/// If `src` is a directory, it creates the directory at `dest` and copies all its contents recursively.
pub fn copy_recursive(src: &Path, dest: &Path) -> io::Result<()> {
    if dest.starts_with(src) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cannot copy a directory into itself",
        ));
    }

    if src.is_dir() {
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &dest.join(entry.file_name()))?;
        }
    } else {
        fs::copy(src, dest)?;
    }
    Ok(())
}

pub fn readable_path(path: &Path) -> String {
    #[cfg(windows)]
    {
        let display = path.display().to_string();
        display
            .strip_prefix(r"\\?\")
            .unwrap_or(&display)
            .to_string()
    }
    #[cfg(not(windows))]
    {
        path.display().to_string()
    }
}

/// Helpers to convert Option<&PathBuf> to Option<&Path>
pub fn as_path_op(opt: Option<&PathBuf>) -> Option<&Path> {
    opt.map(|pathb| pathb.as_path())
}

/// Helper utils integration tests
#[cfg(test)]
mod tests {
    use super::*;

    use std::error;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_path_collision_increments() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let path = dir.path().join("data.csv");

        assert_eq!(get_unused_path(&path.clone()), path);

        File::create(&path)?;
        assert_eq!(
            get_unused_path(&path.clone()),
            dir.path().join("data_1.csv")
        );

        File::create(dir.path().join("data_1.csv"))?;
        assert_eq!(get_unused_path(&path), dir.path().join("data_2.csv"));
        Ok(())
    }

    #[test]
    fn test_hidden_file_collision() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let path = dir.path().join(".config");

        File::create(&path)?;
        // Result: .config_1
        assert_eq!(get_unused_path(&path), dir.path().join(".config_1"));
        Ok(())
    }

    #[test]
    fn test_get_unused_path_basic() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");

        let path1 = get_unused_path(&file_path);
        assert_eq!(path1, file_path);

        File::create(&file_path)?;
        let path2 = get_unused_path(&file_path);
        let path2_fname = path2
            .file_name()
            .ok_or("Failed to get file name from path2")?
            .to_str()
            .ok_or("File name not valid UTF-8")?;
        assert_eq!(path2_fname, "test_1.txt");

        File::create(&path2)?;
        let path3 = get_unused_path(&file_path);
        let path3_fname = path3
            .file_name()
            .ok_or("Failed to get file name from path3")?
            .to_str()
            .ok_or("File name not valid UTF-8")?;
        assert_eq!(path3_fname, "test_2.txt");
        Ok(())
    }

    #[test]
    fn test_get_unused_path_no_extension() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let folder_path = dir.path().join("my_folder");

        File::create(&folder_path)?;
        let path = get_unused_path(&folder_path);

        // Should handle files/folders without extensions correctly
        let fname = path
            .file_name()
            .ok_or("No file name in path")?
            .to_str()
            .ok_or("File name not valid UTF-8")?;
        assert_eq!(fname, "my_folder_1");
        Ok(())
    }

    #[test]
    fn test_get_unused_path_hidden_file() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let dot_file = dir.path().join(".gitignore");

        File::create(&dot_file)?;
        let path = get_unused_path(&dot_file);

        let fname = path
            .file_name()
            .ok_or("No file name in path")?
            .to_str()
            .ok_or("File name not valid UTF-8")?;
        assert_eq!(fname, ".gitignore_1");
        Ok(())
    }

    #[test]
    fn test_get_unused_path_complex_extension() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let tar_gz = dir.path().join("archive.tar.gz");

        File::create(&tar_gz)?;
        let path = get_unused_path(&tar_gz);

        let name = path
            .file_name()
            .ok_or("No file name in path")?
            .to_str()
            .ok_or("File name not valid UTF-8")?;
        assert!(name.contains("_1"), "Suffix missing: got {:?}", name);
        Ok(())
    }
}
