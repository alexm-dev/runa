//! File and directory browsing logic for runa.
//!
//! Provides the FileEntry struct which is used throughout runa.
//! Also holds all the FileInfo and FileType structs used by the ShowInfo Overlay

use crate::core::format_attributes;

use std::ffi::OsString;
use std::fs::{self, symlink_metadata};
use std::io;
use std::path::Path;
use std::time::SystemTime;

/// Represents a single entry in a directory listing
/// Holds the name, display name, and attributes like is_dir, is_hidden, is_system
/// Used throughout runa for directory browsing and file management
/// Created and populated by the browse_dir function.
///
/// # Fields
/// * `name` - The original OsString name of the file or directory
/// * `name_str` - The String representation of the name
/// * `lowercase_name` - The lowercase version of the name for case-insensitive comparisons
/// * `display_name` - The name formatted for display (e.g., with trailing slash for directories)
/// * `file_flags` - Struct holding boolean flags for is_dir, is_hidden, is_system, is_symlink
#[derive(Debug, Clone)]
pub struct FileEntry {
    name: OsString,
    name_str: String,
    lowercase_name: String,
    display_name: String,
    file_flags: FileFlags,
}

impl FileEntry {
    fn new(
        name: OsString,
        name_str: String,
        lowercase_name: String,
        display_name: String,
        file_flags: FileFlags,
    ) -> Self {
        FileEntry {
            name,
            name_str,
            lowercase_name,
            display_name,
            file_flags,
        }
    }

    // Accessors

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
        self.file_flags.is_dir
    }

    pub fn is_hidden(&self) -> bool {
        self.file_flags.is_hidden
    }

    pub fn is_system(&self) -> bool {
        self.file_flags.is_system
    }

    pub fn is_symlink(&self) -> bool {
        self.file_flags.is_symlink
    }

    pub fn extension(&self) -> Option<String> {
        Path::new(&self.name_str)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
    }

    // Setters

    pub fn set_display_name(&mut self, new_name: String) {
        self.display_name = new_name;
    }
}

/// Struct to hold file attribute flags for FileEntry
/// Holds is_dir, is_hidden, is_system, is_symlink booleans.
/// Used internally by FileEntry.
/// # Fields
/// * `is_dir` - Boolean indicating if the entry is a directory
/// * `is_hidden` - Boolean indicating if the entry is hidden
/// * `is_system` - Boolean indicating if the entry is a system file
/// * `is_symlink` - Boolean indicating if the entry is a symlink
#[derive(Debug, Clone, Copy)]
struct FileFlags {
    is_dir: bool,
    is_hidden: bool,
    is_system: bool,
    is_symlink: bool,
}

/// Enumerator for the filye types which are then shown inside [FileInfo]
///
/// Hold File, Directory, Symlink and Other types.
#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    File,
    Directory,
    Symlink,
    Other,
}

/// Main FileInfo struct that holds each info field for the ShowInfo overlay widget.
/// Holds name, size, modified time, attributes string, and file type.
///
/// # Fields
/// * `name` - The OsString name of the file or directory
/// * `size` - The size of the file in bytes (None for directories)
/// * `modified` - The last modified time as SystemTime (None if unavailable)
/// * `attributes` - A formatted string of file attributes
/// * `file_type` - The FileType enum indicating if it's a file, directory, symlink, or other
#[derive(Debug, Clone, PartialEq)]
pub struct FileInfo {
    name: OsString,
    size: Option<u64>,
    modified: Option<SystemTime>,
    attributes: String,
    file_type: FileType,
}

impl FileInfo {
    // Accessors

    pub fn name(&self) -> &OsString {
        &self.name
    }

    pub fn size(&self) -> &Option<u64> {
        &self.size
    }

    pub fn modified(&self) -> &Option<SystemTime> {
        &self.modified
    }

    pub fn attributes(&self) -> &str {
        &self.attributes
    }

    pub fn file_type(&self) -> &FileType {
        &self.file_type
    }

    /// Main file info getter used by the ShowInfo overlay functions
    ///
    /// # Arguments
    /// * `path` - Path reference to the file or directory to get info for
    ///
    /// # Returns
    /// A FileInfo struct populated with the file's information.
    pub fn get_file_info(path: &Path) -> io::Result<FileInfo> {
        let metadata = symlink_metadata(path)?;
        let file_type = if metadata.is_file() {
            FileType::File
        } else if metadata.is_dir() {
            FileType::Directory
        } else if metadata.file_type().is_symlink() {
            FileType::Symlink
        } else {
            FileType::Other
        };

        Ok(FileInfo {
            name: path.file_name().unwrap_or_default().to_os_string(),
            size: if metadata.is_file() {
                Some(metadata.len())
            } else {
                None
            },
            modified: metadata.modified().ok(),
            attributes: format_attributes(&metadata),
            file_type,
        })
    }
}

/// Reads the cotents of the proviced directory and returns them in a vector of FileEntry
///
/// # Arguments
/// * `path` - Path reference to the directory to browse
///
/// # Returns
/// A Result containing a vector of FileEntry structs or an std::io::Error
pub fn browse_dir(path: &std::path::Path) -> std::io::Result<Vec<FileEntry>> {
    let mut entries = Vec::with_capacity(256);

    for entry in fs::read_dir(path)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name();
        let name_lossy = name.to_string_lossy();

        let (is_dir, is_symlink) = match entry.file_type() {
            Ok(ft) => (ft.is_dir(), ft.is_symlink()),
            Err(_) => (false, false),
        };

        let is_hidden: bool;
        let is_system: bool;

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            is_hidden = name.as_bytes().first() == Some(&b'.');
            is_system = false;
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if let Ok(md) = entry.metadata() {
                let attrs = md.file_attributes();
                is_hidden = (attrs & 0x2 != 0) || name_lossy.starts_with('.');
                is_system = attrs & 0x4 != 0;
            } else {
                is_hidden = name_lossy.starts_with('.');
                is_system = false;
            }
        }

        let name_str: String = name_lossy.into_owned();
        let lowercase_name: String = name_str.to_lowercase();

        let mut display_name = name_str.clone();
        if is_dir {
            display_name.push('/');
        }

        let file_flags = FileFlags {
            is_dir,
            is_hidden,
            is_system,
            is_symlink,
        };

        entries.push(FileEntry::new(
            name,
            name_str,
            lowercase_name,
            display_name,
            file_flags,
        ));
    }
    Ok(entries)
}
