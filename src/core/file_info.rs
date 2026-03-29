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
use std::sync::OnceLock;

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

#[cfg(unix)]
#[derive(Debug)]
pub(crate) struct UnixMetadata {
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) owner_name: OnceLock<Option<Arc<str>>>,
    pub(crate) group_name: OnceLock<Option<Arc<str>>>,
}

#[derive(Debug)]
pub(crate) struct CachedFileInfo {
    path: PathBuf,
    size: Option<u64>,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
    accessed: Option<SystemTime>,
    attributes: String,
    file_type: FileType,
    #[cfg(unix)]
    unix_meta: Option<Box<UnixMetadata>>,
}

impl CachedFileInfo {
    pub(crate) fn new(path: PathBuf, info: FileInfo) -> Self {
        Self {
            path,
            size: info.size,
            modified: info.modified,
            created: info.created,
            accessed: info.accessed,
            attributes: info.attributes,
            file_type: info.file_type,
            #[cfg(unix)]
            unix_meta: info.owner_uid.and_then(|uid| {
                info.group_gid.map(|gid| {
                    Box::new(UnixMetadata {
                        uid,
                        gid,
                        owner_name: OnceLock::new(),
                        group_name: OnceLock::new(),
                    })
                })
            }),
        }
    }

    #[inline]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    #[inline]
    pub(crate) fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
    }

    #[inline]
    pub(crate) fn perms(&self) -> String {
        format!("{:width$}", self.attributes, width = PERMS_WIDTH)
    }

    #[inline]
    pub(crate) fn size(&self) -> String {
        format_file_size(self.size, self.file_type == FileType::Directory)
    }

    #[inline]
    pub(crate) fn modified(&self, fmt: &str) -> String {
        format_file_time(self.modified, fmt)
    }

    #[inline]
    pub(crate) fn created(&self, fmt: &str) -> String {
        format_file_time(self.created, fmt)
    }

    #[inline]
    pub(crate) fn accessed(&self, fmt: &str) -> String {
        format_file_time(self.accessed, fmt)
    }

    #[inline]
    pub(crate) fn file_type(&self) -> &'static str {
        format_file_type(&self.file_type)
    }

    #[cfg(unix)]
    pub(crate) fn owner(&self) -> Option<Arc<str>> {
        let meta = self.unix_meta.as_ref()?;

        let name_opt = meta.owner_name.get_or_init(|| {
            uzers::get_user_by_uid(meta.uid)
                .map(|u| u.name().to_string_lossy().into())
                .or_else(|| Some(meta.uid.to_string().into()))
        });

        name_opt.as_ref().map(Arc::clone)
    }

    #[cfg(unix)]
    pub(crate) fn group(&self) -> Option<Arc<str>> {
        let meta = self.unix_meta.as_ref()?;

        let name_opt = meta.group_name.get_or_init(|| {
            uzers::get_group_by_gid(meta.gid)
                .map(|g| g.name().to_string_lossy().into())
                .or_else(|| Some(meta.gid.to_string().into()))
        });

        name_opt.as_ref().map(Arc::clone)
    }

    #[cfg(unix)]
    pub(crate) fn prepare_unix_names(
        &mut self,
        id_cache: &mut crate::core::file_info::unix_info::IdentityCache,
    ) {
        if let Some(meta) = self.unix_meta.as_mut() {
            let owner = id_cache.resolve_user(meta.uid);
            let group = id_cache.resolve_group(meta.gid);

            meta.owner_name = OnceLock::from(Some(owner));
            meta.group_name = OnceLock::from(Some(group));
        }
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
