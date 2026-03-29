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
    fn get_unix_sync_str<F>(&self, selector: F) -> Option<Arc<str>>
    where
        F: FnOnce(&UnixMetadata) -> &Mutex<Option<Arc<str>>>,
    {
        let meta = self.unix_meta.as_ref()?;
        let mutex = selector(meta);
        mutex.lock().ok()?.as_ref().map(Arc::clone)
    }

    #[cfg(unix)]
    #[inline]
    pub(crate) fn owner_name(&self) -> Option<Arc<str>> {
        self.get_unix_sync_str(|m| &m.owner_name)
    }

    #[cfg(unix)]
    #[inline]
    pub(crate) fn group_name(&self) -> Option<Arc<str>> {
        self.get_unix_sync_str(|m| &m.group_name)
    }
}

#[cfg(unix)]
#[derive(Debug)]
pub(crate) struct UnixMetadata {
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) owner: Mutex<Option<Arc<str>>>,
    pub(crate) group: Mutex<Option<Arc<str>>>,
}

#[derive(Debug)]
pub(crate) struct CachedFileInfo {
    pub(crate) path: PathBuf,
    pub(crate) strings: FileInfoStrings,
    #[cfg(unix)]
    pub(crate) unix_meta: Option<Box<UnixMetadata>>,
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

        #[cfg(unix)]
        let unix_meta = info.owner_uid.and_then(|uid| {
            info.group_gid.map(|gid| {
                Box::new(UnixMetadata {
                    uid,
                    gid,
                    owner_name: Mutex::new(None),
                    group_name: Mutex::new(None),
                })
            })
        });

        Self {
            path,
            strings,
            #[cfg(unix)]
            unix_meta,
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
        let meta = self.unix_meta.as_ref()?;
        let uid = meta.uid;

        if let Ok(guard) = meta.owner_name.lock() {
            if let Some(name) = guard.as_ref() {
                return Some(Arc::clone(name));
            }
        }

        let name: Arc<str> = uzers::get_user_by_uid(uid)
            .map(|u| u.name().to_string_lossy().into())
            .unwrap_or_else(|| uid.to_string().into());

        if let Ok(mut guard) = meta.owner_name.lock() {
            *guard = Some(Arc::clone(&name));
        }
        Some(name)
    }

    #[cfg(unix)]
    pub(crate) fn resolved_group_name(&self) -> Option<Arc<str>> {
        let meta = self.unix_meta.as_ref()?;
        let gid = meta.gid;

        if let Ok(guard) = meta.group_name.lock() {
            if let Some(name) = guard.as_ref() {
                return Some(Arc::clone(name));
            }
        }

        let name: Arc<str> = uzers::get_group_by_gid(gid)
            .map(|g| g.name().to_string_lossy().into())
            .unwrap_or_else(|| gid.to_string().into());

        if let Ok(mut guard) = meta.group_name.lock() {
            *guard = Some(Arc::clone(&name));
        }
        Some(name)
    }
}

#[cfg(unix)]
pub(crate) mod unix_info {
    use std::collections::HashMap;
    use std::sync::Arc;

    pub(crate) struct IdentityCache {
        users: HashMap<u32, Arc<str>>,
        groups: HashMap<u32, Arc<str>>,
    }

    impl IdentityCache {
        pub(crate) fn new() -> Self {
            Self {
                users: HashMap::with_capacity(32),
                groups: HashMap::with_capacity(32),
            }
        }

        pub(crate) fn resolve_user(&mut self, uid: u32) -> Arc<str> {
            if let Some(name) = self.users.get(&uid) {
                return Arc::clone(name);
            }
            let name: Arc<str> = uzers::get_user_by_uid(uid)
                .map(|u| u.name().to_string_lossy().into())
                .unwrap_or_else(|| uid.to_string().into());

            self.users.insert(uid, Arc::clone(&name));
            name
        }

        pub(crate) fn resolve_group(&mut self, gid: u32) -> Arc<str> {
            if let Some(name) = self.groups.get(&gid) {
                return Arc::clone(name);
            }
            let name: Arc<str> = uzers::get_group_by_gid(gid)
                .map(|g| g.name().to_string_lossy().into())
                .unwrap_or_else(|| gid.to_string().into());

            self.groups.insert(gid, Arc::clone(&name));
            name
        }
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
