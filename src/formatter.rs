use crate::file_manager::FileEntry;

pub struct Formatter {
    dirs_first: bool,
    show_hidden: bool,
    show_system: bool,
    case_insensitive: bool,
}

impl Formatter {
    pub fn new(
        dirs_first: bool,
        show_hidden: bool,
        show_system: bool,
        case_insensitive: bool,
    ) -> Self {
        Self {
            dirs_first,
            show_hidden,
            show_system,
            case_insensitive,
        }
    }

    pub fn format(&self, entries: &mut [FileEntry]) {
        let cmp_name = |a: &FileEntry, b: &FileEntry| {
            if self.case_insensitive {
                a.name()
                    .to_string_lossy()
                    .to_lowercase()
                    .cmp(&b.name().to_string_lossy().to_lowercase())
            } else {
                a.name().cmp(b.name())
            }
        };
        if self.dirs_first {
            entries.sort_unstable_by(|a, b| match (a.is_dir(), b.is_dir()) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => cmp_name(a, b),
            });
        } else {
            entries.sort_unstable_by(cmp_name);
        }
    }
    pub fn filter_entries(&self, entries: &mut Vec<FileEntry>) {
        entries.retain(|e| {
            let hidden_ok = self.show_hidden || !e.is_hidden();
            let system_ok = self.show_system || !e.is_system();
            hidden_ok && system_ok
        });
        self.format(entries);
    }
} // impl Formatter
