//! Sorting, filtering, and display formatting for file entries in runa.
//!
//! The [Formatter] struct holds pane width and rules for sorting and filtering entries,
//! based on user preferences from the runa.toml configuration.
//! Used to prepare file lists for display in each pane.
//!
//! Also formatts FileTypes to be used by FileInfo and ShowInfo overlay widget.

use crate::core::FileType;
use crate::core::{FileEntry, browse_dir};

use chrono::{DateTime, Local};
use humansize::{DECIMAL, format_size};
use unicode_width::UnicodeWidthChar;

use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{File, Metadata};
use std::io::{BufRead, BufReader, ErrorKind, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

// Minimum number of lines shown in any preview
const MIN_PREVIEW_LINES: usize = 3;
// Maximum file size allowed for preview (10mb)
const MAX_PREVIEW_SIZE: u64 = 10 * 1024 * 1024;
// Number of bytes to peek from file start for header checks (eg. PNG, ZIP, etc..)
const HEADER_PEEK_BYTES: usize = 8;
// Bytes to peek for null bytes in binary detections
const BINARY_PEEK_BYTES: usize = 1024;

/// Formatter struct to handle sorting, filtering, and formatting of file entries
/// based on user preferences.
pub struct Formatter {
    dirs_first: bool,
    show_hidden: bool,
    show_system: bool,
    case_insensitive: bool,
    always_show: Arc<HashSet<OsString>>,
    always_show_lowercase: Arc<HashSet<String>>,
}

impl Formatter {
    pub fn new(
        dirs_first: bool,
        show_hidden: bool,
        show_system: bool,
        case_insensitive: bool,
        always_show: Arc<HashSet<OsString>>,
    ) -> Self {
        let always_show_lowercase = Arc::new(
            always_show
                .iter()
                .map(|s| s.to_string_lossy().to_lowercase())
                .collect::<HashSet<String>>(),
        );
        Self {
            dirs_first,
            show_hidden,
            show_system,
            case_insensitive,
            always_show,
            always_show_lowercase,
        }
    }

    /// Sorts the given file entries in place according to the formatter's settings.
    pub fn sort_entries(&self, entries: &mut [FileEntry]) {
        entries.sort_by(|a, b| {
            if self.dirs_first {
                match (a.is_dir(), b.is_dir()) {
                    (true, false) => return std::cmp::Ordering::Less,
                    (false, true) => return std::cmp::Ordering::Greater,
                    _ => {}
                }
            }
            if self.case_insensitive {
                a.lowercase_name().cmp(b.lowercase_name())
            } else {
                a.name_str().cmp(&b.name_str())
            }
        });
    }

    /// Filters the given file entries in place according to the formatter's settings.
    pub fn filter_entries(&self, entries: &mut Vec<FileEntry>) {
        entries.retain(|e| {
            let is_exception = if self.case_insensitive {
                self.always_show_lowercase.contains(e.lowercase_name())
            } else {
                self.always_show.contains(e.name())
            };

            if is_exception {
                return true;
            }

            let hidden_ok = self.show_hidden || !e.is_hidden();
            let system_ok = self.show_system || !e.is_system();
            hidden_ok && system_ok
        });
        self.sort_entries(entries);
    }
}

/// Formatts the file attributes like Directory, Symlink, and permissions in a unix-like format
///
/// On Unix: Returns a string like 'drwxr-xr-x' etc. for directories and files.
/// On Windows: Returns a short string showing file type and attributes like:
/// (`d`, `l`, `h` for hidden, `s` for system, `a` for archive, `r` for read-only). Not all flags map 1:1 to Unix.
///
/// # Returns
/// A string representing the formatted file attributes used by FileInfo
pub fn format_attributes(meta: &Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let file_type = meta.file_type();
        let first = if file_type.is_dir() {
            'd'
        } else if file_type.is_symlink() {
            'l'
        } else {
            '-'
        };
        let mode = meta.permissions().mode();
        let mut chars = [first, '-', '-', '-', '-', '-', '-', '-', '-', '-'];
        let shifts = [6, 3, 0];
        for (i, &shift) in shifts.iter().enumerate() {
            let base = 1 + i * 3;
            if (mode >> (shift + 2)) & 1u32 != 0 {
                chars[base] = 'r';
            }
            if (mode >> (shift + 1)) & 1u32 != 0 {
                chars[base + 1] = 'w';
            }
            if (mode >> shift) & 1u32 != 0 {
                chars[base + 2] = 'x';
            }
        }
        chars.iter().collect()
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        let attr = meta.file_attributes();
        let mut out = String::with_capacity(7);
        out.push(if attr & 0x10 != 0 {
            'd'
        } else if attr & 0x400 != 0 {
            'l'
        } else {
            '-'
        });
        out.push(if attr & 0x02 != 0 { 'h' } else { '-' });
        out.push(if attr & 0x04 != 0 { 's' } else { '-' });
        out.push(if attr & 0x20 != 0 { 'a' } else { '-' });
        out.push(if attr & 0x01 != 0 { 'r' } else { '-' });
        out
    }
}

/// Formats the FileType enum into a human-readable string.
/// # Returns
/// A static string representing the file type.
pub fn format_file_type(file_type: &FileType) -> &'static str {
    match file_type {
        FileType::File => "File",
        FileType::Directory => "Directory",
        FileType::Symlink => "Symlink",
        FileType::Other => "Other",
    }
}

/// Formats the file size into a human-readable string.
/// # Returns
/// A string representing the formatted file size or "-" for directories/unknown sizes.
pub fn format_file_size(size: Option<u64>, is_dir: bool) -> String {
    if is_dir {
        "-".into()
    } else if let Some(sz) = size {
        format_size(sz, DECIMAL)
    } else {
        "-".to_string()
    }
}

/// Formats the file modification time into a human-readable string.
/// # Returns
/// A string representing the formatted modification time or "-" if unknown.
pub fn format_file_time(modified: Option<SystemTime>) -> String {
    modified
        .map(|mtime| {
            let dt: DateTime<Local> = DateTime::from(mtime);
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_else(|| "-".to_string())
}

/// Returns Some(resolved_target) if entry is a symlink and can be resolved, otherwise None.
pub fn symlink_target_resolved(
    entry: &crate::core::FileEntry,
    parent_dir: &Path,
) -> Option<PathBuf> {
    if !entry.is_symlink() {
        return None;
    }
    let entry_path = parent_dir.join(entry.name());
    if let Ok(target) = std::fs::read_link(&entry_path) {
        let resolved = if target.is_absolute() {
            target
        } else {
            entry_path
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .join(target)
        };
        Some(resolved)
    } else {
        None
    }
}

/// Calculating the pane widht and clean the output to the widht of the pane
/// by removing control characters, expanding tabs to 4 spaces,
/// and truncating or padding the string to fit exactly.
/// # Returns
/// A sanitized string that fits exactly within the specified pane width.
pub fn sanitize_to_exact_width(line: &str, pane_width: usize) -> String {
    let mut out = String::with_capacity(pane_width);
    let mut current_w = 0;

    for char in line.chars() {
        if char == '\t' {
            let space_count = 4 - (current_w % 4);
            if current_w + space_count > pane_width {
                break;
            }
            out.push_str(&" ".repeat(space_count));
            current_w += space_count;
            continue;
        }

        if char.is_control() {
            continue;
        }

        let w = char.width().unwrap_or(0);
        if current_w + w > pane_width {
            break;
        }

        out.push(char);
        current_w += w;
    }

    // If the string is shorter than the pane, fill it with spaces.
    if current_w < pane_width {
        out.push_str(&" ".repeat(pane_width - current_w));
    }

    out
}

/// Loads a fixed-width preview of a directory entries
/// # Returns
/// A vector of strings, each representing a line from the directory preview.
pub fn preview_directory(path: &Path, max_lines: usize, pane_width: usize) -> Vec<String> {
    match browse_dir(path) {
        Ok(entries) => {
            let mut lines = Vec::with_capacity(max_lines);
            let total_entries = entries.len();

            for e in entries.iter().take(max_lines) {
                let display_name = if e.is_dir() {
                    e.name().to_string_lossy().clone() + "/"
                } else {
                    e.name().to_string_lossy().clone()
                };
                lines.push(sanitize_to_exact_width(&display_name, pane_width));
            }

            if lines.is_empty() {
                lines.push(sanitize_to_exact_width("[empty directory]", pane_width));
            } else if total_entries > max_lines
                && let Some(last) = lines.last_mut()
            {
                *last = sanitize_to_exact_width("...", pane_width);
            }

            while lines.len() < max_lines {
                lines.push(" ".repeat(pane_width));
            }
            lines
        }
        Err(e) => {
            let err_msg = "[Error: ".to_owned() + &e.to_string() + "]";
            let mut err_lines = vec![sanitize_to_exact_width(&err_msg, pane_width)];
            while err_lines.len() < max_lines {
                err_lines.push(" ".repeat(pane_width));
            }
            err_lines
        }
    }
}

/// Loads a preview for any path (directory or file), returning an error or a padded lines for
/// display.
/// large binaries/unreadable and unsupported files are replaced with a notice.
///
/// # Returns
/// A vector of strings, each representing a line from the file or directory preview.
pub fn safe_read_preview(path: &Path, max_lines: usize, pane_width: usize) -> Vec<String> {
    let max_lines = std::cmp::max(max_lines, MIN_PREVIEW_LINES);

    // Metadata check
    let Ok(meta) = std::fs::metadata(path) else {
        return vec![sanitize_to_exact_width(
            "[Error: Access Denied]",
            pane_width,
        )];
    };

    // Directory Check
    if meta.is_dir() {
        return preview_directory(path, max_lines, pane_width);
    }

    // Size Check
    if meta.len() > MAX_PREVIEW_SIZE {
        return vec![sanitize_to_exact_width(
            "[File too large for preview]",
            pane_width,
        )];
    }

    // Regular File Check
    if !meta.is_file() {
        return vec![sanitize_to_exact_width("[Not a regular file]", pane_width)];
    }

    // File Read and binary Check
    match File::open(path) {
        Ok(mut file) => {
            // Peek for the first 8 bytes to handle edge cases
            let mut header = [0u8; HEADER_PEEK_BYTES];
            let read_bytes = file.read(&mut header).unwrap_or(0);
            if read_bytes >= 5 && &header[..5] == b"%PDF-" {
                return vec![sanitize_to_exact_width(
                    "[Binary file - preview hidden]",
                    pane_width,
                )];
            }

            // Peek for null bytes to detect binary files
            let mut buffer = [0u8; BINARY_PEEK_BYTES];
            let n = file.read(&mut buffer).unwrap_or(0);
            if buffer[..n].contains(&0) {
                return vec![sanitize_to_exact_width(
                    "[Binary file - preview hidden]",
                    pane_width,
                )];
            }

            // Rewind to start for full read
            let _ = file.rewind();

            // Read lines for preview
            let reader = BufReader::new(file);
            let mut preview_lines = Vec::with_capacity(max_lines);

            // Read up to max_lines
            for line_result in reader.lines().take(max_lines) {
                match line_result {
                    Ok(line) => {
                        preview_lines.push(sanitize_to_exact_width(&line, pane_width));
                    }
                    Err(_) => break,
                }
            }

            // Handle Empty File
            if preview_lines.is_empty() {
                preview_lines.push(sanitize_to_exact_width("[Empty file]", pane_width));
            }

            preview_lines
        }
        Err(e) => {
            let msg = match e.kind() {
                ErrorKind::PermissionDenied => "[Error: Permission Denied]",
                ErrorKind::NotFound => "[Error: File Not Found]",
                _ => {
                    return vec![sanitize_to_exact_width(
                        &format!("[Error reading file: {}]", e),
                        pane_width,
                    )];
                }
            };
            vec![sanitize_to_exact_width(msg, pane_width)]
        }
    }
}

/// Normalize a relative path to use forward slashes for consistency across platforms.
pub fn normalize_relative_path(path: &Path) -> String {
    let rel = path.to_string_lossy().into_owned();
    #[cfg(windows)]
    {
        rel.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        rel
    }
}

/// Normalize separators in a given string to use forward slashes.
pub fn normalize_separators<'a>(separator: &'a str) -> Cow<'a, str> {
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
pub fn flatten_separators(separator: &str) -> String {
    let mut buf = String::with_capacity(separator.len());
    for char in separator.chars() {
        if char != '/' && char != '\\' {
            buf.push(char);
        }
    }
    buf
}

/// Formatter integration tests
#[cfg(test)]
mod tests {

    use crate::core;
    use tempfile::tempdir;

    #[test]
    fn test_ui_sanitization_and_exact_width() {
        let pane_width = 10;

        let cases = vec![
            ("short.txt", 10),
            ("very_long_filename.txt", 10),
            ("🦀_crab.rs", 10),
            ("\t_tab", 10),
        ];

        for (input, expected_width) in cases {
            let result = core::sanitize_to_exact_width(input, pane_width);

            let actual_width = unicode_width::UnicodeWidthStr::width(result.as_str());

            assert_eq!(
                actual_width, expected_width,
                "Failed to produce exact width for input: '{}'. Result was: '{}' (width: {})",
                input, result, actual_width
            );

            assert!(
                !result.chars().any(|c| c.is_control() && c != ' '),
                "Result contains control characters: {:?}",
                result
            );
        }
    }

    #[test]
    fn test_core_empty_dir() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let entries = core::browse_dir(temp_dir.path())?;

        assert!(entries.is_empty(), "Directory should be empty");
        Ok(())
    }
}
