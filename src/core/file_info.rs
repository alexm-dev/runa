//! FileInfo struct and related functions for retrieving and formatting
//! file information for the ShowInfo overlay and the status line.
//!
//! This module defines the [FileInfo] struct which holds relevant
//! information about a file, such as its name, size, modified time,
//! attributes, and type.
//!
//! The main entry point is [FileInfo::get_file_info], which takes a
//! file path and returns a populated [FileInfo] instance.

use crate::core::formatter::{
    format_attributes, format_file_size, format_file_time, format_file_type,
};
use std::ffi::OsString;
use std::fs::{self, symlink_metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(unix)]
use std::sync::Arc;
#[cfg(unix)]
use std::sync::Mutex;

#[cfg(windows)]
pub(crate) const PERMS_WIDTH: usize = 5;
#[cfg(unix)]
pub(crate) const PERMS_WIDTH: usize = 10;

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
    created: Option<SystemTime>,
    accessed: Option<SystemTime>,
    attributes: String,
    file_type: FileType,
    #[cfg(unix)]
    owner_uid: Option<u32>,
    #[cfg(unix)]
    group_gid: Option<u32>,
}

impl FileInfo {
    // Accessors

    /// Main file info getter used by the ShowInfo overlay functions
    /// # Returns
    /// A FileInfo struct populated with the file's information.
    pub(crate) fn get_file_info(
        path: &Path,
        metadata: Option<fs::Metadata>,
    ) -> io::Result<FileInfo> {
        let metadata = match metadata {
            Some(m) => m,
            None => symlink_metadata(path)?,
        };

        let file_type = if metadata.is_file() {
            FileType::File
        } else if metadata.is_dir() {
            FileType::Directory
        } else if metadata.file_type().is_symlink() {
            FileType::Symlink
        } else {
            FileType::Other
        };

        #[cfg(unix)]
        let (owner_uid, group_gid) = {
            use std::os::unix::fs::MetadataExt;
            (Some(metadata.uid()), Some(metadata.gid()))
        };

        Ok(FileInfo {
            name: path.file_name().unwrap_or_default().to_os_string(),
            size: if metadata.is_file() {
                Some(metadata.len())
            } else {
                None
            },
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
            accessed: metadata.accessed().ok(),
            attributes: format_attributes(&metadata),
            file_type,
            #[cfg(unix)]
            owner_uid,
            #[cfg(unix)]
            group_gid,
        })
    }
}

#[derive(Debug)]
pub(crate) struct FileInfoStrings {
    name: Option<String>,
    perms: Option<String>,
    size: Option<String>,
    modified: Option<String>,
    created: Option<String>,
    accessed: Option<String>,
    file_type: Option<&'static str>,
    #[cfg(unix)]
    owner: Option<Arc<str>>,
    #[cfg(unix)]
    group: Option<Arc<str>>,
}

impl FileInfoStrings {
    #[inline]
    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[inline]
    pub(crate) fn perms(&self) -> Option<&str> {
        self.perms.as_deref()
    }

    #[inline]
    pub(crate) fn size(&self) -> Option<&str> {
        self.size.as_deref()
    }

    #[inline]
    pub(crate) fn modified(&self) -> Option<&str> {
        self.modified.as_deref()
    }

    #[inline]
    pub(crate) fn created(&self) -> Option<&str> {
        self.created.as_deref()
    }

    #[inline]
    pub(crate) fn accessed(&self) -> Option<&str> {
        self.accessed.as_deref()
    }

    #[inline]
    pub(crate) fn file_type(&self) -> Option<&'static str> {
        self.file_type
    }

    #[cfg(unix)]
    #[inline]
    pub(crate) fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    #[cfg(unix)]
    #[inline]
    pub(crate) fn group(&self) -> Option<&str> {
        self.group.as_deref()
    }
}

#[derive(Debug)]
pub(crate) struct CachedFileInfo {
    path: PathBuf,
    strings: FileInfoStrings,
    #[cfg(unix)]
    owner_uid: Option<u32>,
    #[cfg(unix)]
    group_gid: Option<u32>,
    #[cfg(unix)]
    owner_name: Mutex<Option<Arc<str>>>,
    #[cfg(unix)]
    group_name: Mutex<Option<Arc<str>>>,
}

impl CachedFileInfo {
    pub(crate) fn new(path: PathBuf, info: FileInfo, time_format: &str) -> Self {
        let is_dir = info.file_type == FileType::Directory;

        let strings = FileInfoStrings {
            name: Some(info.name.to_string_lossy().into_owned()),
            perms: Some(format!("{:width$}", info.attributes, width = PERMS_WIDTH)),
            size: Some(format_file_size(info.size, is_dir)),
            modified: Some(format_file_time(info.modified, time_format)),
            created: Some(format_file_time(info.created, time_format)),
            accessed: Some(format_file_time(info.accessed, time_format)),
            file_type: Some(format_file_type(&info.file_type)),
            #[cfg(unix)]
            owner: None,
            #[cfg(unix)]
            group: None,
        };

        Self {
            path,
            strings,
            #[cfg(unix)]
            owner_uid: info.owner_uid,
            #[cfg(unix)]
            group_gid: info.group_gid,
            #[cfg(unix)]
            owner_name: Mutex::new(None),
            #[cfg(unix)]
            group_name: Mutex::new(None),
        }
    }

    #[inline]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    #[inline]
    pub(crate) fn strings(&self) -> &FileInfoStrings {
        &self.strings
    }

    #[cfg(unix)]
    pub(crate) fn resolved_owner_name(&self) -> Option<Arc<str>> {
        use std::ops::Deref;
        let uid = self.owner_uid?;
        {
            let guard = self.owner_name.lock().unwrap();
            if let Some(name) = guard.deref() {
                return Some(Arc::clone(name));
            }
        }
        let name = unix_info::resolve_user(uid);
        let mut guard = self.owner_name.lock().unwrap();
        *guard = Some(Arc::clone(&name));
        Some(name)
    }

    #[cfg(unix)]
    pub(crate) fn resolved_group_name(&self) -> Option<Arc<str>> {
        let gid = self.group_gid?;
        {
            let guard = self.group_name.lock().unwrap();
            if let Some(name) = guard.as_ref() {
                return Some(Arc::clone(name));
            }
        }
        let name = unix_info::resolve_group(gid);
        let mut guard = self.group_name.lock().unwrap();
        *guard = Some(Arc::clone(&name));
        Some(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::browse_dir;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn file_info_basic_file() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = TempDir::new()?;
        let file_path = tmp.path().join("hello.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "abc123")?;

        let info = FileInfo::get_file_info(&file_path, None)?;
        assert_eq!(&info.file_type, &FileType::File);
        assert_eq!(info.name.to_string_lossy(), "hello.txt");
        assert!(info.size.is_some());
        Ok(())
    }

    #[test]
    fn file_info_directory() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = TempDir::new()?;
        let dir_path = tmp.path().join("emptydir");
        fs::create_dir(&dir_path)?;

        let info = FileInfo::get_file_info(&dir_path, None)?;
        assert_eq!(&info.file_type, &FileType::Directory);
        assert_eq!(&info.size, &None);
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
