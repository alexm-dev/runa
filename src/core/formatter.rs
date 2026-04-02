//! Sorting, filtering, and display formatting for file entries in runa.
//!
//! The [Formatter] struct holds pane width and rules for sorting and filtering entries,
//! based on user preferences from the runa.toml configuration.
//! Used to prepare file lists for display in each pane.
//!
//! Also formatts FileTypes to be used by FileMetadata and ShowInfo overlay widget.

use crate::app::nav::{SortConfig, SortMode, SortOrder};
use crate::core::FileEntry;
use crate::core::metadata::{FileType, get_or_update_cached_meta};
use crate::utils::{clean_display_path, is_regular_file, shorten_home_path, with_lowered_stack};

use chrono::{DateTime, Local};
use humansize::{DECIMAL, format_size};
use unicode_width::UnicodeWidthChar;

use std::cmp::Ordering;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{File, Metadata};
use std::io::{BufRead, BufReader, ErrorKind, Read, Seek};
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// Minimum number of lines shown in any preview
const MIN_PREVIEW_LINES: usize = 3;
// Maximum file size allowed for preview (5gb)
const MAX_PREVIEW_SIZE: u64 = 5_000 * 1024 * 1024;
// Number of bytes to peek from file start for header checks (eg. PNG, ZIP, etc..)
const HEADER_PEEK_BYTES: usize = 8;
// Bytes to peek for null bytes in binary detections
const BINARY_PEEK_BYTES: usize = 1024;

#[derive(Clone)]
pub(crate) struct DirListOptions {
    pub(crate) dirs_first: bool,
    pub(crate) show_hidden: bool,
    pub(crate) show_symlink: bool,
    pub(crate) show_system: bool,
    pub(crate) case_insensitive: bool,
}

#[derive(Clone, Copy)]
enum MetadataSortField {
    Size,
    Modified,
    Created,
    Accessed,
}

/// Formatter struct to handle sorting, filtering, and formatting of file entries
/// based on user preferences.
pub(crate) struct Formatter {
    list: DirListOptions,
    sort_config: SortConfig,
    always_show: Option<Arc<HashSet<OsString>>>,
    always_show_lowercase: Option<Arc<HashSet<String>>>,
}

impl Formatter {
    const PRIO_DIR: u8 = 0;
    const PRIO_FILE: u8 = 1;

    pub(crate) fn new(
        list: DirListOptions,
        sort_config: SortConfig,
        always_show: Arc<HashSet<OsString>>,
    ) -> Self {
        let (always_show, always_show_lowercase) = if always_show.is_empty() {
            (None, None)
        } else if list.case_insensitive {
            let lower = Arc::new(
                always_show
                    .iter()
                    .map(|s| s.to_string_lossy().to_lowercase())
                    .collect::<HashSet<_>>(),
            );
            (Some(always_show), Some(lower))
        } else {
            (Some(always_show), None)
        };

        Self {
            list,
            sort_config,
            always_show,
            always_show_lowercase,
        }
    }

    #[inline]
    fn prio_for_entry(&self, entry: &FileEntry) -> u8 {
        if self.list.dirs_first && (entry.flags() & FileEntry::IS_DIR) != 0 {
            Self::PRIO_DIR
        } else {
            Self::PRIO_FILE
        }
    }

    /// Sorts the given file entries in place according to the formatter's settings.
    pub(crate) fn sort_entries(
        &self,
        directory_path: &Path,
        entries: &mut Vec<FileEntry>,
        list_date_format: &str,
    ) -> Option<Vec<Arc<str>>> {
        match self.sort_config.mode {
            SortMode::Name => {
                self.sort_by_name(entries);
                None
            }
            SortMode::Natural => {
                self.sort_by_natural(entries);
                None
            }
            SortMode::Extension => {
                self.sort_by_extension(entries);
                None
            }
            SortMode::Size => Some(self.sort_by_metadata(
                directory_path,
                entries,
                MetadataSortField::Size,
                list_date_format,
            )),
            SortMode::Modified => Some(self.sort_by_metadata(
                directory_path,
                entries,
                MetadataSortField::Modified,
                list_date_format,
            )),
            SortMode::Created => Some(self.sort_by_metadata(
                directory_path,
                entries,
                MetadataSortField::Created,
                list_date_format,
            )),
            SortMode::Accessed => Some(self.sort_by_metadata(
                directory_path,
                entries,
                MetadataSortField::Accessed,
                list_date_format,
            )),
        }
    }

    fn sort_by_name(&self, entries: &mut [FileEntry]) {
        let sort_order = self.sort_config.order;

        if !self.list.case_insensitive {
            entries.sort_unstable_by(|left_entry, right_entry| {
                let left_priority = self.prio_for_entry(left_entry);
                let right_priority = self.prio_for_entry(right_entry);

                if left_priority != right_priority {
                    return left_priority.cmp(&right_priority);
                }

                let result = left_entry.name().cmp(right_entry.name());
                if sort_order == SortOrder::Ascending {
                    result
                } else {
                    result.reverse()
                }
            });
            return;
        }

        entries.sort_unstable_by(|a, b| {
            let ord = self.prio_for_entry(a).cmp(&self.prio_for_entry(b));
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }

            let result = a.lowered().cmp(b.lowered());

            if sort_order == SortOrder::Ascending {
                result
            } else {
                result.reverse()
            }
        });
    }

    fn sort_by_natural(&self, entries: &mut [FileEntry]) {
        let sort_order = self.sort_config.order;
        let case_insensitive = self.list.case_insensitive;

        entries.sort_unstable_by(|a, b| {
            let ord = self.prio_for_entry(a).cmp(&self.prio_for_entry(b));
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }

            let result = if case_insensitive {
                natural_cmp_ascii_ci(a.lowered(), b.lowered())
            } else {
                natural_cmp_ascii(a.lowered(), b.lowered())
            };

            if sort_order == SortOrder::Ascending {
                result
            } else {
                result.reverse()
            }
        });
    }

    fn sort_by_extension(&self, entries: &mut [FileEntry]) {
        let sort_order = self.sort_config.order;
        let case_insensitive = self.list.case_insensitive;

        entries.sort_unstable_by(|a, b| {
            let ord = self.prio_for_entry(a).cmp(&self.prio_for_entry(b));
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }

            let a_ext = Path::new(a.name()).extension();
            let b_ext = Path::new(b.name()).extension();

            let mut result = if case_insensitive {
                let a_ext_str = a_ext.and_then(|s| s.to_str()).unwrap_or_default();
                with_lowered_stack(a_ext_str, |a_low| {
                    with_lowered_stack(
                        &b_ext.map(|s| s.to_string_lossy()).unwrap_or_default(),
                        |b_low| a_low.cmp(b_low),
                    )
                })
            } else {
                a_ext.cmp(&b_ext)
            };

            if result == std::cmp::Ordering::Equal {
                result = if case_insensitive {
                    a.lowered().cmp(b.lowered())
                } else {
                    a.name().cmp(b.name())
                };
            }

            if sort_order == SortOrder::Ascending {
                result
            } else {
                result.reverse()
            }
        });
    }

    fn sort_by_metadata(
        &self,
        directory_path: &Path,
        entries: &mut Vec<FileEntry>,
        metadata_sort_field: MetadataSortField,
        list_date_format: &str,
    ) -> Vec<Arc<str>> {
        use crate::core::formatter::{format_file_size, format_file_time};

        let mut keys: Vec<(u8, u128, usize)> = Vec::with_capacity(entries.len());
        let mut column: Vec<Arc<str>> = Vec::with_capacity(entries.len());

        let mut path_buffer = directory_path.to_path_buf();

        for (index, file_entry) in entries.iter().enumerate() {
            let priority = self.prio_for_entry(file_entry);

            path_buffer.push(file_entry.name());

            if let Some(cached) = get_or_update_cached_meta(&path_buffer) {
                let (key, display) = match metadata_sort_field {
                    MetadataSortField::Size => {
                        let val = cached.size.unwrap_or(0);
                        (
                            val as u128,
                            format_file_size(cached.size, file_entry.is_dir()),
                        )
                    }
                    MetadataSortField::Modified => (
                        system_time_to_key(cached.modified),
                        format_file_time(cached.modified, list_date_format),
                    ),
                    MetadataSortField::Created => (
                        system_time_to_key(cached.created),
                        format_file_time(cached.created, list_date_format),
                    ),
                    MetadataSortField::Accessed => (
                        system_time_to_key(cached.accessed),
                        format_file_time(cached.accessed, list_date_format),
                    ),
                };

                keys.push((priority, key, index));
                column.push(Arc::from(display));
            } else {
                keys.push((priority, 0, index));
                column.push(Arc::from("-"));
            }

            path_buffer.pop();
        }

        let sort_order = self.sort_config.order;

        keys.sort_unstable_by(|left, right| {
            let p_ord = left.0.cmp(&right.0);
            if p_ord != Ordering::Equal {
                return p_ord;
            }

            let mut m_ord = if sort_order == SortOrder::Ascending {
                left.1.cmp(&right.1)
            } else {
                right.1.cmp(&left.1)
            };

            if m_ord == Ordering::Equal {
                let a = &entries[left.2];
                let b = &entries[right.2];
                m_ord = if self.list.case_insensitive {
                    a.lowered().cmp(b.lowered())
                } else {
                    a.name().cmp(b.name())
                };
            }
            m_ord
        });

        let old_entries = std::mem::take(entries);
        let old_column = column;

        let mut new_column = Vec::with_capacity(entries.len());

        for (_, _, idx) in &keys {
            entries.push(old_entries[*idx].clone());
            new_column.push(old_column[*idx].clone());
        }

        new_column
    }

    pub(crate) fn filter_entries(&self, entries: &mut Vec<FileEntry>) {
        let mut hide = 0u8;
        if !self.list.show_hidden {
            hide |= FileEntry::IS_HIDDEN;
        }
        if !self.list.show_system {
            hide |= FileEntry::IS_SYSTEM;
        }
        if !self.list.show_symlink {
            hide |= FileEntry::IS_SYMLINK;
        }
        entries.retain(|e| {
            let flags = e.flags();

            if (flags & hide) != 0 {
                if self.list.case_insensitive {
                    if let Some(set) = &self.always_show_lowercase {
                        return set.contains(e.lowered());
                    }
                } else if let Some(set) = &self.always_show {
                    return set.contains(e.name());
                }
                return false;
            }
            true
        });
    }
}

/// Formatts the file attributes like Directory, Symlink, and permissions in a unix-like format
///
/// On Unix: Returns a string like 'drwxr-xr-x' etc. for directories and files.
/// On Windows: Returns a short string showing file type and attributes like:
/// (`d`, `l`, `h` for hidden, `s` for system, `a` for archive, `r` for read-only). Not all flags map 1:1 to Unix.
///
/// # Returns
/// A string representing the formatted file attributes used by FileMetadata
pub(crate) fn format_attributes(meta: &Metadata) -> String {
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

        out.push(if attr & 0x20 != 0 { 'a' } else { '-' });
        out.push(if attr & 0x01 != 0 { 'r' } else { '-' });
        out.push(if attr & 0x02 != 0 { 'h' } else { '-' });
        out.push(if attr & 0x04 != 0 { 's' } else { '-' });
        out
    }
}

/// Formats the FileType enum into a human-readable string.
/// # Returns
/// A static string representing the file type.
pub(crate) fn format_file_type(file_type: &FileType) -> &'static str {
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
pub(crate) fn format_file_size(size: Option<u64>, is_dir: bool) -> String {
    if is_dir {
        "-".into()
    } else if let Some(sz) = size {
        format_size(sz, DECIMAL)
    } else {
        "-".to_string()
    }
}

/// Formats the file modification time into a human-readable string.
pub(crate) fn format_file_time(time: Option<SystemTime>, format: &str) -> String {
    let Some(t) = time else {
        return "-".to_string();
    };
    let dt: DateTime<Local> = DateTime::from(t);

    let formatted = dt.format(format).to_string();
    if formatted.is_empty() {
        return dt.format("%Y-%m-%d %H:%M").to_string();
    }
    formatted
}

pub(crate) fn format_display_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    let cleaned = clean_display_path(&path_str);
    let path_short = shorten_home_path(cleaned);
    #[cfg(windows)]
    {
        path_short.replace('/', "\\")
    }
    #[cfg(not(windows))]
    {
        path_short
    }
}

/// Calculating the pane widht and clean the output to the widht of the pane
/// by removing control characters, expanding tabs to 4 spaces,
/// and truncating or padding the string to fit exactly.
/// # Returns
/// A sanitized string that fits exactly within the specified pane width.
pub(crate) fn sanitize_to_exact_width(line: &str, pane_width: usize) -> String {
    let mut out = String::with_capacity(pane_width);
    let mut current_w = 0;

    for ch in line.chars() {
        if ch == '\t' {
            let space_count = 4 - (current_w % 4);
            if current_w + space_count > pane_width {
                break;
            }
            for _ in 0..space_count {
                out.push(' ');
            }
            current_w += space_count;
            continue;
        }

        if ch.is_control() {
            continue;
        }

        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);

        if current_w + ch_width > pane_width {
            break;
        }

        out.push(ch);
        current_w += ch_width;
    }

    while current_w < pane_width {
        out.push(' ');
        current_w += 1;
    }

    out
}

/// Loads a preview for any path (directory or file), returning an error or a padded lines for
/// display.
/// large binaries/unreadable and unsupported files are replaced with a notice.
///
/// # Returns
/// A vector of strings, each representing a line from the file or directory preview.
pub(crate) fn safe_read_preview(path: &Path, max_lines: usize, pane_width: usize) -> Vec<String> {
    let max_lines = std::cmp::max(max_lines, MIN_PREVIEW_LINES);

    if !is_regular_file(path) {
        return vec![sanitize_to_exact_width(
            "[Not a regular file - preview skipped]",
            pane_width,
        )];
    }

    // File Read and binary Check
    match File::open(path) {
        Ok(mut file) => {
            if let Ok(metadata) = file.metadata() {
                if !metadata.is_file() {
                    return vec![sanitize_to_exact_width(
                        "[Not a regular file - preview skipped]",
                        pane_width,
                    )];
                }

                if metadata.len() > MAX_PREVIEW_SIZE {
                    return vec![sanitize_to_exact_width(
                        "[File too large for preview]",
                        pane_width,
                    )];
                }
            }

            // Peek for null bytes to detect binary files
            let mut buffer = [0u8; BINARY_PEEK_BYTES];
            let n = file.read(&mut buffer).unwrap_or(0);

            let header_len = std::cmp::min(n, HEADER_PEEK_BYTES);
            let header = &buffer[..header_len];

            if header.len() >= 5 && &header[..5] == b"%PDF-" {
                return vec![sanitize_to_exact_width(
                    "[Binary file - preview hidden]",
                    pane_width,
                )];
            }

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

#[inline]
fn system_time_to_key(system_time: Option<SystemTime>) -> u128 {
    system_time
        .and_then(|st| st.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn natural_cmp_ascii(a: &str, b: &str) -> Ordering {
    natural_cmp_bytes(a.as_bytes(), b.as_bytes(), false)
}

fn natural_cmp_ascii_ci(a: &str, b: &str) -> Ordering {
    natural_cmp_bytes(a.as_bytes(), b.as_bytes(), true)
}

fn natural_cmp_bytes(a: &[u8], b: &[u8], fold_case: bool) -> Ordering {
    let mut i = 0usize;
    let mut j = 0usize;

    while i < a.len() && j < b.len() {
        let a_is_digit = a[i].is_ascii_digit();
        let b_is_digit = b[j].is_ascii_digit();

        if a_is_digit && b_is_digit {
            let start_i = i;
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            let start_j = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }

            let mut nonzero_i = start_i;
            while nonzero_i < i && a[nonzero_i] == b'0' {
                nonzero_i += 1;
            }
            let mut nonzero_j = start_j;
            while nonzero_j < j && b[nonzero_j] == b'0' {
                nonzero_j += 1;
            }

            let len_i = i - nonzero_i;
            let len_j = j - nonzero_j;

            if len_i != len_j {
                return len_i.cmp(&len_j);
            }

            for digit_offset in 0..len_i {
                let digit_i = a[nonzero_i + digit_offset];
                let digit_j = b[nonzero_j + digit_offset];
                if digit_i != digit_j {
                    return digit_i.cmp(&digit_j);
                }
            }

            let leading_zeros_i = nonzero_i - start_i;
            let leading_zeros_j = nonzero_j - start_j;
            if leading_zeros_i != leading_zeros_j {
                return leading_zeros_i.cmp(&leading_zeros_j);
            }

            continue;
        }

        let mut byte_i = a[i];
        let mut byte_j = b[j];

        if fold_case && (byte_i.is_ascii_uppercase() || byte_j.is_ascii_uppercase()) {
            byte_i = byte_i.to_ascii_lowercase();
            byte_j = byte_j.to_ascii_lowercase();
        }

        if byte_i != byte_j {
            return byte_i.cmp(&byte_j);
        }

        i += 1;
        j += 1;
    }

    a.len().cmp(&b.len())
}

/// Formatter integration tests
#[cfg(test)]
mod tests {

    use super::*;
    use crate::core;
    use tempfile::tempdir;

    #[test]
    fn ui_sanitization_and_exact_width() {
        let pane_width = 10;

        let cases = vec![
            ("short.txt", 10),
            ("very_long_filename.txt", 10),
            ("🦀_crab.rs", 10),
            ("\t_tab", 10),
        ];

        for (input, expected_width) in cases {
            let result = sanitize_to_exact_width(input, pane_width);

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
    fn formatter_filters_entries_by_flags() {
        let normal = FileEntry::new(OsString::from("normal.txt"), 0, None);
        let hidden = FileEntry::new(OsString::from(".hidden"), FileEntry::IS_HIDDEN, None);
        let system = FileEntry::new(OsString::from("system.sys"), FileEntry::IS_SYSTEM, None);
        let symlink = FileEntry::new(
            OsString::from("symlink"),
            FileEntry::IS_SYMLINK,
            Some(Path::new("target").to_path_buf()),
        );

        let mut entries = vec![
            normal.clone(),
            hidden.clone(),
            system.clone(),
            symlink.clone(),
        ];

        let list_hide = DirListOptions {
            dirs_first: true,
            show_hidden: false,
            show_system: false,
            show_symlink: false,
            case_insensitive: true,
        };
        let short_config = SortConfig::default();

        let fmt = Formatter::new(list_hide, short_config, Arc::new(HashSet::new()));
        fmt.filter_entries(&mut entries);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name_str(), "normal.txt");

        let mut entries = vec![
            normal.clone(),
            hidden.clone(),
            system.clone(),
            symlink.clone(),
        ];

        let list_show = DirListOptions {
            dirs_first: true,
            show_hidden: true,
            show_system: true,
            show_symlink: true,
            case_insensitive: true,
        };

        let fmt = Formatter::new(list_show, short_config, Arc::new(HashSet::new()));
        fmt.filter_entries(&mut entries);
        assert_eq!(entries.len(), 4);
        assert!(entries.iter().any(|e| e.name_str() == ".hidden"));
        assert!(entries.iter().any(|e| e.name_str() == "system.sys"));
        assert!(entries.iter().any(|e| e.name_str() == "symlink"));
        assert!(entries.iter().any(|e| e.name_str() == "normal.txt"));
    }

    #[test]
    fn core_empty_dir() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let entries = core::browse_dir(temp_dir.path())?;

        assert!(entries.is_empty(), "Directory should be empty");
        Ok(())
    }
}
