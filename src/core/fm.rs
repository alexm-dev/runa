//! File and directory browsing logic for runa.
//!
//! Provides the FileEntry struct which is used throughout runa.
//! Also holds all the FileInfo and FileType structs used by the ShowInfo Overlay

use crate::core::formatter::format_attributes;

use std::borrow::Cow;
use std::ffi::OsString;
use std::fs::{self, symlink_metadata};
use std::io;
use std::path::Path;
use std::time::SystemTime;

/// Represents a single entry in a directory listing
/// Holds the name, display name, and attributes like is_dir, is_hidden, is_system
/// Used throughout runa for directory browsing and file management
/// Created and populated by the browse_dir function.
#[derive(Debug, Clone)]
pub(crate) struct FileEntry {
    name: OsString,
    lowercase_name: Box<str>,
    flags: u8,
}

impl FileEntry {
    // Flag bit definitions
    // These are used to set and check attributes in the flags field
    const IS_DIR: u8 = 1 << 0;
    const IS_HIDDEN: u8 = 1 << 1;
    const IS_SYSTEM: u8 = 1 << 2;
    const IS_SYMLINK: u8 = 1 << 3;

    fn new(name: OsString, flags: u8) -> Self {
        let lowercase_name = name.to_string_lossy().to_lowercase().into_boxed_str();
        FileEntry {
            name,
            lowercase_name,
            flags,
        }
    }

    // Accessors

    #[inline]
    pub(crate) fn name(&self) -> &OsString {
        &self.name
    }

    pub(crate) fn name_str(&self) -> Cow<'_, str> {
        self.name.to_string_lossy()
    }

    #[inline]
    pub(crate) fn lowercase_name(&self) -> &str {
        &self.lowercase_name
    }

    #[inline]
    pub(crate) fn is_dir(&self) -> bool {
        self.flags & Self::IS_DIR != 0
    }

    #[inline]
    pub(crate) fn is_hidden(&self) -> bool {
        self.flags & Self::IS_HIDDEN != 0
    }

    #[inline]
    pub(crate) fn is_system(&self) -> bool {
        self.flags & Self::IS_SYSTEM != 0
    }

    #[inline]
    pub(crate) fn is_symlink(&self) -> bool {
        self.flags & Self::IS_SYMLINK != 0
    }
}

/// Enumerator for the filye types which are then shown inside [FileInfo]
///
/// Hold File, Directory, Symlink and Other types.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FileType {
    File,
    Directory,
    Symlink,
    Other,
}

/// Main FileInfo struct that holds each info field for the ShowInfo overlay widget.
/// Holds name, size, modified time, attributes string, and file type.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FileInfo {
    name: OsString,
    size: Option<u64>,
    modified: Option<SystemTime>,
    attributes: String,
    file_type: FileType,
}

impl FileInfo {
    // Accessors

    #[inline]
    pub(crate) fn name(&self) -> &OsString {
        &self.name
    }

    #[inline]
    pub(crate) fn size(&self) -> &Option<u64> {
        &self.size
    }

    #[inline]
    pub(crate) fn modified(&self) -> &Option<SystemTime> {
        &self.modified
    }

    #[inline]
    pub(crate) fn attributes(&self) -> &str {
        &self.attributes
    }

    #[inline]
    pub(crate) fn file_type(&self) -> &FileType {
        &self.file_type
    }

    /// Main file info getter used by the ShowInfo overlay functions
    /// # Returns
    /// A FileInfo struct populated with the file's information.
    pub(crate) fn get_file_info(path: &Path) -> io::Result<FileInfo> {
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
/// # Returns
/// A Result containing a vector of FileEntry structs or an std::io::Error
pub(crate) fn browse_dir(path: &Path) -> io::Result<Vec<FileEntry>> {
    let mut entries = Vec::with_capacity(256);

    for entry in fs::read_dir(path)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name();
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        let mut flags = 0u8;
        if ft.is_dir() {
            flags |= FileEntry::IS_DIR;
        }
        if ft.is_symlink() {
            flags |= FileEntry::IS_SYMLINK;
        }

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            if name.as_bytes().first() == Some(&b'.') {
                flags |= FileEntry::IS_HIDDEN;
            }
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if let Ok(md) = entry.metadata() {
                let attrs = md.file_attributes();

                if attrs & 0x2 != 0 {
                    flags |= FileEntry::IS_HIDDEN;
                }
                if attrs & 0x4 != 0 {
                    flags |= FileEntry::IS_SYSTEM;
                }
            }

            if flags & FileEntry::IS_HIDDEN == 0 && name.to_string_lossy().starts_with('.') {
                flags |= FileEntry::IS_HIDDEN;
            }
        }

        entries.push(FileEntry::new(name, flags));
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn file_entry_flags() -> Result<(), Box<dyn std::error::Error>> {
        let fe_file = FileEntry::new(OsString::from("file.txt"), 0);
        assert!(!fe_file.is_dir());
        assert!(!fe_file.is_hidden());
        assert_eq!(fe_file.name_str(), "file.txt");

        let flags = FileEntry::IS_DIR | FileEntry::IS_HIDDEN;
        let fe_dir = FileEntry::new(OsString::from(".hidden_folder"), flags);
        assert!(fe_dir.is_dir());
        assert!(fe_dir.is_hidden());
        assert!(!fe_dir.is_system());
        assert!(!fe_dir.is_symlink());
        assert_eq!(fe_dir.lowercase_name(), ".hidden_folder");
        Ok(())
    }

    #[test]
    fn file_info_basic_file() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = TempDir::new()?;
        let file_path = tmp.path().join("hello.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "abc123")?;

        let info = FileInfo::get_file_info(&file_path)?;
        assert_eq!(info.file_type(), &FileType::File);
        assert_eq!(info.name().to_string_lossy(), "hello.txt");
        assert!(info.size().is_some());
        Ok(())
    }

    #[test]
    fn file_info_directory() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = TempDir::new()?;
        let dir_path = tmp.path().join("emptydir");
        fs::create_dir(&dir_path)?;

        let info = FileInfo::get_file_info(&dir_path)?;
        assert_eq!(info.file_type(), &FileType::Directory);
        assert_eq!(info.size(), &None);
        Ok(())
    }

    #[test]
    fn browse_dir_nonexistent() -> Result<(), Box<dyn std::error::Error>> {
        let path = PathBuf::from("/path/does/not/exist");
        let result = browse_dir(&path);
        assert!(result.is_err());
        Ok(())
    }
}
