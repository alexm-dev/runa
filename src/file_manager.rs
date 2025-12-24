use std::ffi::OsString;
use std::fs;

#[derive(Debug, Clone)]
pub struct FileEntry {
    name: OsString,
    name_str: String,
    lowercase_name: String,
    display_name: String,
    is_dir: bool,
    is_hidden: bool,
    is_system: bool,
}

impl FileEntry {
    pub fn name(&self) -> &OsString {
        &self.name
    }
    pub fn name_str(&self) -> &str {
        &self.name_str
    }
    pub fn lowercase_name(&self) -> &str {
        &self.lowercase_name
    }
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
    pub fn is_hidden(&self) -> bool {
        self.is_hidden
    }
    pub fn is_system(&self) -> bool {
        self.is_system
    }
    pub fn set_display_name(&mut self, new_name: String) {
        self.display_name = new_name;
    }
}

pub fn browse_dir(path: &std::path::Path) -> std::io::Result<Vec<FileEntry>> {
    let mut entries = Vec::with_capacity(256);
    for entry in fs::read_dir(path)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name();
        let name_lossy = name.to_string_lossy();
        let name_str = name_lossy.to_string();
        let lowercase_name = name_lossy.to_lowercase();

        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

        let display_name = if is_dir {
            format!("{}/", name_lossy)
        } else {
            name_lossy.into_owned()
        };

        #[cfg(unix)]
        let (is_hidden, is_system) = {
            use std::os::unix::ffi::OsStrExt;
            // Native byte check: no string conversion needed
            let is_hidden = name.as_bytes().first() == Some(&b'.');
            (is_hidden, false)
        };

        #[cfg(windows)]
        let (is_dir, is_hidden, is_system) = {
            use std::os::windows::fs::MetadataExt;
            let starts_with_dot = lowercase_name.starts_with('.');

            if let Ok(md) = entry.metadata() {
                let attrs = md.file_attributes();
                (
                    md.is_dir(),
                    (attrs & 0x2 != 0) || starts_with_dot,
                    (attrs & 0x4 != 0),
                )
            } else {
                (is_dir, starts_with_dot, false)
            }
        };

        entries.push(FileEntry {
            name,
            name_str,
            lowercase_name,
            display_name,
            is_dir,
            is_hidden,
            is_system,
        });
    }
    Ok(entries)
}
