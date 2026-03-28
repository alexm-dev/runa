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

#[cfg(unix)]
use std::sync::Arc;

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
    #[cfg(unix)]
    owner: Option<Arc<str>>,
    #[cfg(unix)]
    group: Option<Arc<str>>,
}

impl FileInfo {
    // Accessors

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
            #[cfg(unix)]
            owner,
            #[cfg(unix)]
            group,
        })
    }
}

pub(crate) struct FileInfoStrings {
    name: Option<String>,
    perms: Option<String>,
    size: Option<String>,
    #[cfg(unix)]
    owner: Option<Arc<str>>,
    #[cfg(unix)]
    group: Option<Arc<str>>,
    date: Option<String>,
    file_type: Option<&'static str>,
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
        let is_dir = info.file_type == FileType::Directory;

        let strings = FileInfoStrings {
            name: Some(info.name.to_string_lossy().into_owned()),
            perms: Some(format!("{:width$}", info.attributes, width = PERMS_WIDTH)),
            size: Some(format_file_size(info.size, is_dir)),

            #[cfg(unix)]
            owner: info.owner,
            #[cfg(unix)]
            group: info.group,

            date: Some(format_file_time(info.modified)),
            file_type: Some(format_file_type(&info.file_type)),
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
    use std::collections::HashMap;
    use std::sync::{Arc, OnceLock, RwLock};
    use uzers::{get_group_by_gid, get_user_by_uid};

    static USER_CACHE: OnceLock<RwLock<HashMap<u32, Arc<str>>>> = OnceLock::new();
    static GROUP_CACHE: OnceLock<RwLock<HashMap<u32, Arc<str>>>> = OnceLock::new();

    fn get_cache(
        lock: &'static OnceLock<RwLock<HashMap<u32, Arc<str>>>>,
    ) -> &'static RwLock<HashMap<u32, Arc<str>>> {
        lock.get_or_init(|| RwLock::new(HashMap::new()))
    }

    fn resolve_id<F>(
        id: u32,
        lock: &'static OnceLock<RwLock<HashMap<u32, Arc<str>>>>,
        f: F,
    ) -> Arc<str>
    where
        F: FnOnce(u32) -> Option<String>,
    {
        let cache = get_cache(lock);

        if let Ok(map) = cache.read()
            && let Some(name) = map.get(&id)
        {
            return Arc::clone(name);
        }

        let mut map = cache.write().unwrap_or_else(|e| e.into_inner());
        map.entry(id)
            .or_insert_with(|| {
                let name = f(id).unwrap_or_else(|| id.to_string());
                Arc::from(name)
            })
            .clone()
    }

    pub(super) fn resolve_user(uid: u32) -> Arc<str> {
        resolve_id(uid, &USER_CACHE, |id| {
            get_user_by_uid(id).map(|u| u.name().to_string_lossy().into_owned())
        })
    }

    pub(super) fn resolve_group(gid: u32) -> Arc<str> {
        resolve_id(gid, &GROUP_CACHE, |id| {
            get_group_by_gid(id).map(|g| g.name().to_string_lossy().into_owned())
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

        let info = FileInfo::get_file_info(&dir_path)?;
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
