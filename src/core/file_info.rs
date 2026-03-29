//! FileInfo struct and related functions for retrieving and formatting
//! file information for the ShowInfo overlay and the status line.
//!
//! This module defines the [FileInfo] struct which holds relevant
//! information about a file, such as its name, size, modified time,
//! attributes, and type.
//!
//! The main entry point is [FileInfo::new], which takes a
//! file path and returns a populated [FileInfo] instance.

use crate::core::formatter::{
    format_attributes, format_file_size, format_file_time, format_file_type,
};

use std::fs::{self, symlink_metadata};
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
    path: PathBuf,
    size: Option<u64>,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
    accessed: Option<SystemTime>,
    attributes: String,
    file_type: FileType,
    #[cfg(unix)]
    unix_meta: Option<UnixMetadata>,
}

impl FileInfo {
    // Accessors

    /// Main file info getter used by the ShowInfo overlay functions
    /// # Returns
    /// A FileInfo struct populated with the file's information.
    pub(crate) fn new(path: PathBuf, metadata: Option<fs::Metadata>) -> io::Result<FileInfo> {
        let metadata = match metadata {
            Some(m) => m,
            None => symlink_metadata(&path)?,
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
        let unix_meta = {
            use std::os::unix::fs::MetadataExt;
            Some(UnixMetadata {
                uid: metadata.uid(),
                gid: metadata.gid(),
            })
        };

        Ok(FileInfo {
            path,
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
            unix_meta,
        })
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
    pub(crate) fn uid(&self) -> u32 {
        self.unix_meta.as_ref().map(|m| m.uid).unwrap_or(0)
    }

    #[cfg(unix)]
    pub(crate) fn gid(&self) -> u32 {
        self.unix_meta.as_ref().map(|m| m.gid).unwrap_or(0)
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct UnixMetadata {
    pub(crate) uid: u32,
    pub(crate) gid: u32,
}

#[cfg(unix)]
pub(crate) mod unix_info {
    use std::collections::HashMap;
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    pub(crate) struct UserGroupCache {
        users: HashMap<u32, Arc<str>>,
        groups: HashMap<u32, Arc<str>>,
    }

    impl UserGroupCache {
        pub(crate) fn new() -> Self {
            Self {
                users: HashMap::with_capacity(8),
                groups: HashMap::with_capacity(8),
            }
        }

        pub(crate) fn fetch_user(uid: u32) -> Arc<str> {
            uzers::get_user_by_uid(uid)
                .map(|u| u.name().to_string_lossy().into())
                .unwrap_or_else(|| uid.to_string().into())
        }

        pub(crate) fn fetch_group(gid: u32) -> Arc<str> {
            uzers::get_group_by_gid(gid)
                .map(|g| g.name().to_string_lossy().into())
                .unwrap_or_else(|| gid.to_string().into())
        }

        pub(crate) fn resolve_user(&mut self, uid: u32) -> Arc<str> {
            self.users
                .entry(uid)
                .or_insert_with(|| Self::fetch_user(uid))
                .clone()
        }

        pub(crate) fn resolve_group(&mut self, gid: u32) -> Arc<str> {
            self.groups
                .entry(gid)
                .or_insert_with(|| Self::fetch_group(gid))
                .clone()
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

        let info = FileInfo::new(file_path, None)?;
        assert_eq!(&info.file_type, &FileType::File);
        assert_eq!(info.name(), "hello.txt");
        assert!(info.size.is_some());
        Ok(())
    }

    #[test]
    fn file_info_directory() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = TempDir::new()?;
        let dir_path = tmp.path().join("emptydir");
        fs::create_dir(&dir_path)?;

        let info = FileInfo::new(dir_path, None)?;
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

    #[test]
    #[cfg(unix)]
    fn test_file_info_symlink() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = TempDir::new()?;
        let target_path = tmp.path().join("target.txt");
        File::create(&target_path)?;

        let link_path = tmp.path().join("link.txt");
        std::os::unix::fs::symlink(&target_path, &link_path)?;

        let info = FileInfo::new(link_path, None)?;
        assert_eq!(info.file_type, FileType::Symlink);
        assert_eq!(info.name(), "link.txt");
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn test_unix_metadata_retrieval() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = TempDir::new()?;
        let file_path = tmp.path().join("unix_test.txt");
        File::create(&file_path)?;
        let info = FileInfo::new(file_path, None)?;

        assert!(info.unix_meta.is_some());
        assert!(info.uid() > 0 || info.uid() == 0);
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn test_user_group_cache() {
        let mut cache = unix_info::UserGroupCache::new();

        let user = cache.resolve_user(0);
        assert!(!user.is_empty());

        let user_again = cache.resolve_user(0);
        assert_eq!(user, user_again);
    }
}
