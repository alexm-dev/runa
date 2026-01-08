//! Sorting, filtering, and display formatting for file entries in runa.
//!
//! The [Formatter] struct holds pane width and rules for sorting and filtering entries,
//! based on user preferences from the runa.toml configuration.
//! Used to prepare file lists for display in each pane.
//!
//! Also formatts FileTypes to be used by FileInfo and ShowInfo overlay widget.

use crate::core::FileEntry;
use crate::core::file_manager::FileType;

use chrono::{DateTime, Local};
use humansize::{DECIMAL, format_size};

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::Metadata;
use std::sync::Arc;
use std::time::SystemTime;

pub struct Formatter {
    dirs_first: bool,
    show_hidden: bool,
    show_system: bool,
    case_insensitive: bool,
    always_show: Arc<HashSet<OsString>>,
    always_show_lowercase: Arc<HashSet<String>>,
    pane_width: usize,
}

impl Formatter {
    pub fn new(
        dirs_first: bool,
        show_hidden: bool,
        show_system: bool,
        case_insensitive: bool,
        always_show: Arc<HashSet<OsString>>,
        pane_width: usize,
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
            pane_width,
        }
    }

    pub fn format(&self, entries: &mut [FileEntry]) {
        // Sort the entries
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
                a.name_str().cmp(b.name_str())
            }
        });

        // Apply pane_width to the display_name
        for entry in entries.iter_mut() {
            let suffix = if entry.is_dir() { "/" } else { "" };
            let base_name = format!("{}{}", entry.name_str(), suffix);

            let mut out = String::with_capacity(self.pane_width);
            let mut current_w = 0;

            for c in base_name.chars() {
                // simple truncation for the main list
                let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                if current_w + w > self.pane_width {
                    if !out.is_empty() {
                        out.pop();
                        out.push('…');
                    }
                    break;
                }
                out.push(c);
                current_w += w;
            }

            if current_w < self.pane_width {
                out.push_str(&" ".repeat(self.pane_width - current_w));
            }
            entry.set_display_name(out);
        }
    }

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
        self.format(entries);
    }
}

/// Formatts the file attributes like Directory, Symlink, and permissions in a unix-like format
///
/// On Unix: Returns a string like 'drwxr-xr-x' etc. for directories and files.
/// On Windows: Returns a short string showing file type and attributes like:
/// (`d`, `l`, `h` for hidden, `s` for system, `a` for archive, `r` for read-only). Not all flags map 1:1 to Unix.
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

pub fn format_file_type(file_type: &FileType) -> &'static str {
    match file_type {
        FileType::File => "File",
        FileType::Directory => "Directory",
        FileType::Symlink => "Symlink",
        FileType::Other => "Other",
    }
}

pub fn format_file_size(size: Option<u64>, is_dir: bool) -> String {
    if is_dir {
        "-".into()
    } else if let Some(sz) = size {
        format_size(sz, DECIMAL)
    } else {
        "-".to_string()
    }
}

pub fn format_file_time(modified: Option<SystemTime>) -> String {
    modified
        .map(|mtime| {
            let dt: DateTime<Local> = DateTime::from(mtime);
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_else(|| "-".to_string())
}
