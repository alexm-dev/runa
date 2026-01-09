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
/// * `is_dir` - Boolean indicating if the entry is a directory
/// * `is_hidden` - Boolean indicating if the entry is hidden
/// * `is_system` - Boolean indicating if the entry is a system file (useful on Windows)
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
    pub fn new(
        name: OsString,
        name_str: String,
        lowercase_name: String,
        display_name: String,
        is_dir: bool,
        is_hidden: bool,
        is_system: bool,
    ) -> Self {
        FileEntry {
            name,
            name_str,
            lowercase_name,
            display_name,
            is_dir,
            is_hidden,
            is_system,
        }
    }

    // Getters / accessors

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

    // Setters

    pub fn set_display_name(&mut self, new_name: String) {
        self.display_name = new_name;
    }
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

    // Main file info getter used by the ShowInfo overlay functions
    //
    // # Arguments
    // * `path` - Path reference to the file or directory to get info for
    //
    // # Returns
    // A FileInfo struct populated with the file's information.
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
