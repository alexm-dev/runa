use crate::config::Config;
use crate::file_manager::{FileEntry, read_dir};
use crate::formatter::Formatter;

pub struct AppState {
    pub entries: Vec<FileEntry>,
    pub selected: usize,
}

impl AppState {
    pub fn new(config: &Config) -> std::io::Result<Self> {
        let mut entries = read_dir(".")?;

        let formatter = Formatter::new(
            config.dirs_first,
            config.show_hidden,
            config.case_insensitive,
        );

        formatter.filter_hidden(&mut entries);

        Ok(Self {
            entries,
            selected: 0,
        })
    }
}
