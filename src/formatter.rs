//! Sorting, filtering, and display formatting for file entries in runa.
//!
//! The [Formatter] struct holds pane width and rules for sorting and filtering entries,
//! based on user preferences from the runa.toml configuration.
//! Used to prepare file lists for display in each pane.

use crate::file_manager::FileEntry;
use std::collections::HashSet;
use std::ffi::OsString;
use std::sync::Arc;

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
} // impl Formatter
