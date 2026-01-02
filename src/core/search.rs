use crate::core::FileEntry;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::ffi::OsString;
use std::io;
use std::path::Path;
use walkdir::WalkDir;

pub fn find(root: &Path, query: &str, results: &mut Vec<(FileEntry, i64)>) -> io::Result<()> {
    let matcher = SkimMatcherV2::default();

    for entry in WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Build FileEntry fields
        let name: OsString = entry.file_name().to_os_string();
        let name_str = name.to_string_lossy().to_string();
        let lowercase_name = name_str.to_lowercase();
        let is_dir = entry.file_type().is_dir();
        let display_name = if is_dir {
            format!("{}/", name_str)
        } else {
            name_str.clone()
        };

        // Hidden/system flag detection
        #[cfg(unix)]
        let (is_hidden, is_system) = {
            use std::os::unix::ffi::OsStrExt;
            let is_hidden = name.as_bytes().first() == Some(&b'.');
            (is_hidden, false)
        };

        #[cfg(windows)]
        let (is_hidden, is_system) = {
            use std::os::windows::fs::MetadataExt;
            let starts_with_dot = lowercase_name.starts_with('.');
            if let Ok(md) = entry.metadata() {
                let attrs = md.file_attributes();
                ((attrs & 0x2 != 0) || starts_with_dot, (attrs & 0x4 != 0))
            } else {
                (starts_with_dot, false)
            }
        };

        // Fuzzy match on name_str
        if let Some(score) = matcher.fuzzy_match(&name_str, query) {
            let file_entry = FileEntry::new(
                name,
                name_str,
                lowercase_name,
                display_name,
                is_dir,
                is_hidden,
                is_system,
            );
            results.push((file_entry, score));
        }
    }
    Ok(())
}
