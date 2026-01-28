//! File and directory browsing logic for runa.
//!
//! Provides the FileEntry struct which is used throughout runa.
//! Also holds all the FileInfo and FileType structs used by the ShowInfo Overlay

use crate::core::formatter::format_attributes;
use crate::utils::with_lowered_stack;

use std::borrow::Cow;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs::{self, symlink_metadata};
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

/// Represents a single entry in a directory listing
/// Holds the name, display name, and attributes like is_dir, is_hidden, is_system
/// Used throughout runa for directory browsing and file management
/// Created and populated by the browse_dir function.
#[derive(Debug, Clone)]
pub(crate) struct FileEntry {
    name: Box<OsStr>,
    flags: u8,
    symlink: Option<PathBuf>,
}

impl FileEntry {
    // Flag bit definitions
    // These are used to set and check attributes in the flags field
    pub(super) const IS_DIR: u8 = 1 << 0;
    pub(super) const IS_HIDDEN: u8 = 1 << 1;
    pub(super) const IS_SYSTEM: u8 = 1 << 2;
    pub(super) const IS_SYMLINK: u8 = 1 << 3;
    pub(super) const IS_BROKEN_SYM: u8 = 1 << 4;
    pub(super) const IS_EXECUTABLE: u8 = 1 << 5;

    /// Used to set the IS_EXECUTABLE flag for files which can be executed.
    /// Used for coloring executable files in UI
    #[cfg(unix)]
    pub(super) const EXEC_FLAG: u32 = 0o111;

    pub(crate) fn new(name: OsString, flags: u8, symlink: Option<PathBuf>) -> Self {
        FileEntry {
            name: name.into_boxed_os_str(),
            flags,
            symlink,
        }
    }

    // Accessors

    #[inline]
    pub(crate) fn name(&self) -> &OsStr {
        &self.name
    }

    #[inline]
    pub(crate) fn name_str(&self) -> Cow<'_, str> {
        self.name.to_string_lossy()
    }

    #[inline(always)]
    pub(crate) fn flags(&self) -> u8 {
        self.flags
    }

    #[inline]
    pub(crate) fn symlink(&self) -> Option<&PathBuf> {
        self.symlink.as_ref()
    }

    #[inline]
    pub(crate) fn is_dir(&self) -> bool {
        self.flags & Self::IS_DIR != 0
    }

    #[inline]
    pub(crate) fn is_symlink(&self) -> bool {
        self.flags & Self::IS_SYMLINK != 0
    }

    #[inline]
    pub(crate) fn is_broken_sym(&self) -> bool {
        self.flags & Self::IS_BROKEN_SYM != 0
    }

    #[inline]
    pub(crate) fn is_executable(&self) -> bool {
        self.flags & Self::IS_EXECUTABLE != 0
    }

    #[cfg(windows)]
    pub(super) fn match_executable_extension(ext: &str, flags: &mut u8) {
        with_lowered_stack(ext, |lowered| match lowered {
            "exe" | "com" | "bat" | "cmd" | "ps1" => *flags |= FileEntry::IS_EXECUTABLE,
            _ => {}
        })
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
            use std::os::unix::fs::PermissionsExt;

            let md_res = if (flags & FileEntry::IS_SYMLINK) != 0 {
                fs::metadata(entry.path())
            } else {
                entry.metadata()
            };

            if let Ok(md) = md_res {
                if md.is_dir() {
                    flags |= FileEntry::IS_DIR;
                }

                if md.permissions().mode() & FileEntry::EXEC_FLAG != 0 {
                    flags |= FileEntry::IS_EXECUTABLE;
                }
            } else if (flags & FileEntry::IS_SYMLINK) != 0 {
                flags |= FileEntry::IS_BROKEN_SYM;
            }

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

                if (flags & FileEntry::IS_SYMLINK) != 0 {
                    match fs::metadata(entry.path()) {
                        Ok(target_md) => {
                            if target_md.is_dir() {
                                flags |= FileEntry::IS_DIR;
                            }
                        }
                        Err(_) => {
                            flags |= FileEntry::IS_BROKEN_SYM;
                        }
                    }
                } else if attrs & 0x10 != 0 {
                    flags |= FileEntry::IS_DIR;
                }
            }

            if ft.is_file() {
                let path = Path::new(&name);

                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    FileEntry::match_executable_extension(ext, &mut flags);
                }
            }
        }
        let symlink = if (flags & FileEntry::IS_SYMLINK) != 0 {
            fs::read_link(entry.path()).ok()
        } else {
            None
        };
        entries.push(FileEntry::new(name, flags, symlink));
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
        let fe_file = FileEntry::new(OsString::from("file.txt"), 0, None);
        assert!(!fe_file.is_dir());
        assert_eq!(fe_file.name_str(), "file.txt");

        let flags = FileEntry::IS_DIR | FileEntry::IS_HIDDEN;
        let fe_dir = FileEntry::new(OsString::from(".hidden_folder"), flags, None);
        assert!(fe_dir.is_dir());
        assert!(!fe_dir.is_symlink());
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
    fn browse_nonexistent() -> Result<(), Box<dyn std::error::Error>> {
        let path = PathBuf::from("/path/does/not/exist");
        let result = browse_dir(&path);
        assert!(result.is_err());
        Ok(())
    }
}
