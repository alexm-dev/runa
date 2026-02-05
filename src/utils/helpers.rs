//! Helpers for runa.
//!
//! This module defines constants for the minimum, default, and maximum find result limits used throughout runa.
//! It also provides utility functions:
//! - Home directory cache and getter to ensure only a single call to home_dir is made.
//! - Color parsing from strings or hex codes
//! - Opening paths/files in the user's chosen editor
//! - Generating unused filenames to prevent accidental overwrite
//! - Displaying home directories as "~" in file paths
//! - Clamping the find result count to safe values
//!
//! These helpers are used throughout runa.

use crate::config::Editor;
use ratatui::style::Color;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::sync::OnceLock;
use std::{fs, io};

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

/// The minimum results which is set to if the maximum is overset in the runa.toml.
pub(crate) const MIN_FIND_RESULTS: usize = 15;
/// The default find results. Can be overwritten in the runa.toml.
pub(crate) const DEFAULT_FIND_RESULTS: usize = 2000;
/// The maximum find result limit which is possible.
/// Can be set higher, but better to set it to a big limit instead of usize::MAX
pub(crate) const MAX_FIND_RESULTS_LIMIT: usize = 1000000;

/// Deny previews of certain file extensions
const DENY: &[&str] = &["a", "lib", "ilk", "h5", "zip", "gz", "tar"];

/// Shared cache for the home_dir dirs call
static HOME_DIR_CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Thread safe for getting home_dir once.
#[inline]
pub(crate) fn get_home() -> Option<&'static PathBuf> {
    HOME_DIR_CACHE.get_or_init(dirs::home_dir).as_ref()
}

/// Parses a string (color name or hex) into a ratatui::style::color
///
/// Supports standard names (red, green, etc.) as well as hex values (#RRGGBB or #RGB)
pub(crate) fn parse_color(s: &str) -> Color {
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
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
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
pub(crate) fn open_in_editor(editor: &Editor, file_path: &std::path::Path) -> std::io::Result<()> {
    let mut stdout = io::stdout();
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen)?;

    let status = std::process::Command::new(editor.cmd())
        .arg(file_path)
        .status();

    execute!(io::stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(io::Error::other(format!(
            "Editor exited with status: {}",
            s
        ))),
        Err(e) => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Command '{}' not found: {}", editor.cmd(), e),
        )),
    }
}

/// Finds the next available filename by appending _1, _2, etc. if the target exists
///
/// Example: "notes.txt" -> "notes_1.txt"
pub(crate) fn get_unused_path(path: &Path) -> PathBuf {
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

/// Helper to clean path strings by removing applied prefixes applied by caonicalized paths.
/// Mostly useful for windows.
/// On unix, it justs passes the path string back.
pub(crate) fn clean_display_path(path: &str) -> &str {
    #[cfg(windows)]
    {
        if let Some(stripped) = path
            .strip_prefix(r"\\?\")
            .or_else(|| path.strip_prefix("//?/"))
            .or_else(|| path.strip_prefix(r"\??\"))
        {
            return stripped;
        }
    }

    path
}

/// Util function to shorten home directory to ~.
/// Is used by the path_str in the ui.rs render function.
pub(crate) fn shorten_home_path<P: AsRef<Path>>(path: P) -> String {
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

/// Normalize a relative path to use forward slashes for consistency across platforms.
pub(crate) fn normalize_relative_path(path: &Path) -> Cow<'_, str> {
    let rel = path.to_string_lossy();

    #[cfg(windows)]
    {
        if rel.contains('\\') {
            Cow::Owned(rel.replace('\\', "/"))
        } else {
            rel
        }
    }

    #[cfg(not(windows))]
    {
        rel
    }
}

/// Normalize separators in a given string to use forward slashes.
pub(crate) fn normalize_separators<'a>(separator: &'a str) -> Cow<'a, str> {
    if separator.contains('\\') {
        Cow::Owned(separator.replace('\\', "/"))
    } else {
        Cow::Borrowed(separator)
    }
}

/// Flatten separators by removing all '/' and '\' characters from the string.
/// This is used to create a simplified version of the path for fuzzy matching.
///
/// # Examples
/// let flat = flatten_separators("src/core/proc.rs");
/// flat = "srccoreprocrs";
pub(crate) fn flatten_separators(separator: &str) -> String {
    let mut buf = String::with_capacity(separator.len());
    for char in separator.chars() {
        if char != '/' && char != '\\' {
            buf.push(char);
        }
    }
    buf
}

/// Expands the home path (~ to `home/<user>/`) and returns the string of the path.
pub(crate) fn expand_home_path(input: &str) -> String {
    expand_home_path_buf(input).to_string_lossy().to_string()
}

/// Expands the home path and returns the PathBuf of the home path.
pub(crate) fn expand_home_path_buf(input: &str) -> PathBuf {
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

/// Hardened directory check to make sure passed path is actually a directory and not
/// innaccessible.
pub(crate) fn is_hardened_directory(path: &Path) -> bool {
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

/// Helper to resolve the initial loaded for path string.
/// Checks if the path arg is a file and then loads the parent directory of that file.
/// Used by cli path args.
/// # Example
/// `rn ~/test.txt`
/// -> runa started at `~`
pub(crate) fn resolve_initial_dir(path_arg: &str) -> PathBuf {
    let expaned = expand_home_path_buf(path_arg);
    if expaned.is_file() {
        expaned.parent().map(|p| p.to_path_buf()).unwrap_or(expaned)
    } else {
        expaned
    }
}

/// Safely clamp the find result numbers.
///
/// If the clamped value does not match the set [MAX_FIND_RESULTS_LIMIT] then its invalid and its
/// set to the [MIN_FIND_RESULTS] instead.
pub(crate) fn clamp_find_results(value: usize) -> usize {
    let clamped = value.clamp(MIN_FIND_RESULTS, MAX_FIND_RESULTS_LIMIT);
    if clamped != value {
        eprintln!(
            "[Warning] max_find_results={} out of range ({}..={}), clamped to {}",
            value, MIN_FIND_RESULTS, MAX_FIND_RESULTS_LIMIT, clamped
        );
    }
    clamped
}

/// Recursively copies files and directories from `src` to `dest`, with safety checks.
///
/// Safety checks prevent copying a directory into its own subdirectory,
/// Returns an Error if such an operation is attempted.
pub(crate) fn copy_recursive(src: &Path, dest: &Path) -> io::Result<()> {
    let src_canon = src.canonicalize()?;
    let dest_parent = dest
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Destination has no parent"))?;
    let dest_parent_canon = dest_parent.canonicalize().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination parent does not exist",
        )
    })?;
    let file_name = dest.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "Destination has no file name")
    })?;
    let dest_canon = dest_parent_canon.join(file_name);

    if dest_canon.starts_with(&src_canon) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cannot copy a directory into its own subdirectory",
        ));
    }

    copy_recursive_inner(src, dest)
}

/// Internal helper function to perform the actual recursive copy.
/// Handles directories, files, and symbolic links appropriately.
/// This function is called by [copy_recursive] after performing safety checks.
fn copy_recursive_inner(src: &Path, dest: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(src)?;

    if meta.is_dir() {
        let entries = fs::read_dir(src)?;
        fs::create_dir_all(dest)?;

        for entry in entries {
            let entry = entry?;
            copy_recursive_inner(&entry.path(), &dest.join(entry.file_name()))?;
        }
    } else if meta.file_type().is_symlink() {
        let target = fs::read_link(src)?;
        #[cfg(windows)]
        {
            let is_dir_target = fs::metadata(src).map(|m| m.is_dir()).unwrap_or(false);

            if is_dir_target {
                std::os::windows::fs::symlink_dir(&target, dest)?;
            } else {
                std::os::windows::fs::symlink_file(&target, dest)?;
            }
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, dest)?;
        }
    } else {
        fs::copy(src, dest)?;
    }

    Ok(())
}

/// Calls f closure with a lowercase (ASCII) version of a entry name if any are uppercase ASCII.
/// Using a stack buffer for names <= 64 bytes or with unchanged name otherwise.
/// Falls through for Unicode and long strings with no heap allocations.
#[inline(always)]
pub(crate) fn with_lowered_stack<R>(name: &str, f: impl FnOnce(&str) -> R) -> R {
    const BUFFER_SIZE: usize = 64;
    if name.len() <= BUFFER_SIZE {
        let mut buf = [0u8; BUFFER_SIZE];
        let bytes = name.as_bytes();
        let mut needs_lowering = false;

        for i in 0..bytes.len() {
            let b = bytes[i];
            if b.is_ascii_uppercase() {
                needs_lowering = true;
                buf[i] = b.to_ascii_lowercase();
            } else {
                buf[i] = b;
            }
        }

        if needs_lowering && let Ok(lowered) = std::str::from_utf8(&buf[..bytes.len()]) {
            return f(lowered);
        }
    }
    f(name)
}

/// Check for file extension to deny file previews
pub(crate) fn is_preview_deny(path: &Path) -> bool {
    match path.extension() {
        Some(ext) => DENY.iter().any(|&s| ext == OsStr::new(s)),
        None => false,
    }
}

/// Helper utils integration tests
#[cfg(test)]
mod tests {
    use super::*;

    use std::error;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn path_collision_increments() -> Result<(), Box<dyn error::Error>> {
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
    fn hidden_file_collision() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let path = dir.path().join(".config");

        File::create(&path)?;
        // Result: .config_1
        assert_eq!(get_unused_path(&path), dir.path().join(".config_1"));
        Ok(())
    }

    #[test]
    fn get_unused_path_basic() -> Result<(), Box<dyn error::Error>> {
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
    fn get_unused_path_no_extension() -> Result<(), Box<dyn error::Error>> {
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
    fn get_unused_path_hidden_file() -> Result<(), Box<dyn error::Error>> {
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
    fn get_unused_path_complex_extension() -> Result<(), Box<dyn error::Error>> {
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

    #[test]
    fn home_expansion() -> Result<(), Box<dyn error::Error>> {
        let path = expand_home_path_buf("~");
        assert!(path.is_absolute());

        let path = expand_home_path_buf("~/downloads");
        assert!(path.ends_with("downloads"));
        assert!(path.is_absolute());

        Ok(())
    }

    #[test]
    #[cfg(windows)]
    fn windows_drive_normalization() -> Result<(), Box<dyn error::Error>> {
        // Test C: -> C:\
        let path = expand_home_path_buf("C:");
        assert_eq!(path.to_str().ok_or("UTF8 error")?, r"C:\");

        // Test lowercase d: -> d:\
        let path = expand_home_path_buf("d:");
        assert_eq!(path.to_str().ok_or("UTF8 error")?, r"d:\");

        let path = expand_home_path_buf(r"C:\Users");
        assert_eq!(path.to_str().ok_or("UTF8 error")?, r"C:\Users");

        let path = expand_home_path_buf(r"~\Documents");
        assert!(path.is_absolute());
        assert!(path.ends_with("Documents"));

        Ok(())
    }

    #[test]
    fn standard_paths() -> Result<(), Box<dyn error::Error>> {
        let path = expand_home_path_buf("projects/runa");
        assert_eq!(path, Path::new("projects/runa"));

        let sandbox = tempdir()?;
        let absolute_input = sandbox.path().join("my_app").join("config.toml");

        let input_str = absolute_input.to_string_lossy();
        let result = expand_home_path_buf(&input_str);

        assert_eq!(result, absolute_input);
        assert!(result.is_absolute());

        Ok(())
    }

    #[test]
    fn copy_recursive_basic_file() -> Result<(), Box<dyn error::Error>> {
        let src_dir = tempdir()?;
        let dest_dir = tempdir()?;

        let file_path = src_dir.path().join("test.txt");
        fs::write(&file_path, "hello runa")?;

        let dest_path = dest_dir.path().join("test_copied.txt");
        copy_recursive(&file_path, &dest_path)?;

        assert!(dest_path.exists());
        assert_eq!(fs::read_to_string(dest_path)?, "hello runa");
        Ok(())
    }

    #[test]
    fn copy_recursive_directory_structure() -> Result<(), Box<dyn error::Error>> {
        let src_dir = tempdir()?;
        let dest_base = tempdir()?;
        let dest_path = dest_base.path().join("backup");

        let subdir = src_dir.path().join("subdir");
        fs::create_dir(&subdir)?;
        fs::write(subdir.join("inner.txt"), "nested data")?;

        copy_recursive(src_dir.path(), &dest_path)?;

        assert!(dest_path.join("subdir").is_dir());
        assert_eq!(
            fs::read_to_string(dest_path.join("subdir").join("inner.txt"))?,
            "nested data"
        );
        Ok(())
    }

    #[test]
    fn copy_recursive_prevention_subdir() -> Result<(), Box<dyn error::Error>> {
        let src_dir = tempdir()?;
        let src_path = src_dir.path();

        let dest_path = src_path.join("backup");

        let result = copy_recursive(src_path, &dest_path);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("subdirectory"));

        Ok(())
    }
}
