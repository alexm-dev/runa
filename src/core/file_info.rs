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
use std::fs::symlink_metadata;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
    attributes: String,
    file_type: FileType,
    owner: Option<String>,
    group: Option<String>,
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

        #[cfg(unix)]
        let (owner, group) = {
            use std::os::unix::fs::MetadataExt;
            (
                Some(unix_info::resolve_user(metadata.uid())),
                Some(unix_info::resolve_group(metadata.gid())),
            )
        };

        #[cfg(not(unix))]
        let (owner, group) = (None, None);

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
            owner,
            group,
        })
    }
}

pub(crate) struct FileInfoStrings {
    perms: Option<String>,
    size: Option<String>,
    #[cfg(unix)]
    owner: Option<String>,
    #[cfg(unix)]
    group: Option<String>,
    date: Option<String>,
    file_type: Option<&'static str>,
}

impl FileInfoStrings {
    #[inline]
    pub(crate) fn perms(&self) -> Option<&str> {
        self.perms.as_deref()
    }

    #[inline]
    pub(crate) fn size(&self) -> Option<&str> {
        self.size.as_deref()
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

    #[inline]
    pub(crate) fn date(&self) -> Option<&str> {
        self.date.as_deref()
    }

    #[inline]
    pub(crate) fn file_type(&self) -> Option<&'static str> {
        self.file_type
    }
}

pub(crate) struct CachedFileInfo {
    path: PathBuf,
    strings: FileInfoStrings,
}

impl CachedFileInfo {
    pub(crate) fn new(path: PathBuf, info: FileInfo) -> Self {
        let is_dir = *info.file_type() == FileType::Directory;

        let strings = FileInfoStrings {
            perms: Some(format!("{:width$}", info.attributes(), width = PERMS_WIDTH)),
            size: Some(format!("{:>8}", format_file_size(*info.size(), is_dir))),

            #[cfg(unix)]
            owner: info.owner().map(|o| o.to_string()),
            #[cfg(unix)]
            group: info.group().map(|g| g.to_string()),

            date: Some(format_file_time(*info.modified())),
            file_type: Some(format_file_type(info.file_type())),
        };

        Self { path, strings }
    }

    #[inline]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    #[inline]
    pub(crate) fn strings(&self) -> &FileInfoStrings {
        &self.strings
    }
}

#[cfg(unix)]
mod unix_info {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use uzers::{get_group_by_gid, get_user_by_uid};

    thread_local! {
        static USER_CACHE: RefCell<HashMap<u32, String>> = RefCell::new(HashMap::new());
        static GROUP_CACHE: RefCell<HashMap<u32, String>> = RefCell::new(HashMap::new());
    }

    pub fn resolve_user(uid: u32) -> String {
        USER_CACHE.with(|cache| {
            let mut map = cache.borrow_mut();
            map.entry(uid)
                .or_insert_with(|| {
                    get_user_by_uid(uid)
                        .map(|u| u.name().to_string_lossy().into_owned())
                        .unwrap_or_else(|| uid.to_string())
                })
                .clone()
        })
    }

    pub fn resolve_group(gid: u32) -> String {
        GROUP_CACHE.with(|cache| {
            let mut map = cache.borrow_mut();
            map.entry(gid)
                .or_insert_with(|| {
                    get_group_by_gid(gid)
                        .map(|g| g.name().to_string_lossy().into_owned())
                        .unwrap_or_else(|| gid.to_string())
                })
                .clone()
        })
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
