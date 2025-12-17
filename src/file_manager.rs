use std::ffi::OsString;
use std::fs;

pub struct FileEntry {
    name: OsString,
    is_dir: bool,
    is_hidden: bool,
    // flag for hidding system hidden files
    is_system: bool,
}

impl FileEntry {
    pub fn name(&self) -> &OsString {
        &self.name
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
}

pub fn browse_dir(path: &std::path::Path) -> std::io::Result<Vec<FileEntry>> {
    let mut entries = Vec::with_capacity(256);
    for entry in fs::read_dir(path)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name();
        #[cfg(unix)]
        let (is_dir, is_hidden, is_system) = {
            use std::os::unix::ffi::OsStrExt;
            let is_hidden = name.as_bytes().get(0) == Some(&b'.');
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            (is_dir, is_hiddenm, false)
        };

        #[cfg(windows)]
        let (is_dir, is_hidden, is_system) = {
            use std::os::windows::fs::MetadataExt;

            if let Ok(md) = entry.metadata() {
                let attrs = md.file_attributes();
                let is_hidden = attrs & 0x2 != 0;
                let is_system = attrs & 0x4 != 0;
                let is_dir = md.is_dir();
                (is_dir, is_hidden, is_system)
            } else {
                (false, false, false)
            }
        };
        entries.push(FileEntry {
            name,
            is_dir,
            is_hidden,
            is_system,
        });
    }
    Ok(entries)
}
