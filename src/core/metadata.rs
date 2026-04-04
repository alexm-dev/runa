//! FileMetadata struct and related functions for retrieving and formatting
//! file information for the ShowInfo overlay and the status line.
//!
//! This module defines the [FileMetadata] struct which holds relevant
//! information about a file, such as its name, size, modified time,
//! attributes, and type.
//!
//! The main entry point is [FileMetadata::new], which takes a
//! file path and returns a populated [FileMetadata] instance.

use crate::config::display::ShowInfoOptions;
use crate::core::formatter::{
    TimeFormatCtx, format_attributes, format_file_size, format_file_time, format_file_type,
};

use chrono::{DateTime, Local};
use dashmap::DashMap;

use std::fs::symlink_metadata;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};
use std::time::SystemTime;

#[cfg(windows)]
pub(crate) const PERMS_WIDTH: usize = 5;
#[cfg(unix)]
pub(crate) const PERMS_WIDTH: usize = 10;

/// Cached metadata for sorting.
/// Holds system calls required info only.
#[derive(Clone, Copy)]
pub(crate) struct CachedMetaKey {
    pub(crate) epoch: u64,
    pub(crate) size: Option<u64>,
    pub(crate) modified: Option<SystemTime>,
    pub(crate) created: Option<SystemTime>,
    pub(crate) accessed: Option<SystemTime>,
}

static META_SORT_EPOCH: OnceLock<AtomicU64> = OnceLock::new();
static META_SORT_CACHE: OnceLock<DashMap<PathBuf, CachedMetaKey>> = OnceLock::new();

#[inline]
fn epoch_atomic() -> &'static AtomicU64 {
    META_SORT_EPOCH.get_or_init(|| AtomicU64::new(1))
}

#[inline]
pub(crate) fn meta_cache() -> &'static DashMap<PathBuf, CachedMetaKey> {
    META_SORT_CACHE.get_or_init(DashMap::new)
}

#[inline]
fn meta_sort_epoch() -> u64 {
    epoch_atomic().load(Ordering::Relaxed)
}

pub(crate) fn bump_meta_sort_epoch() {
    epoch_atomic().fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn get_or_update_cached_meta(path: &Path) -> Option<CachedMetaKey> {
    let epoch = meta_sort_epoch();
    let cache = meta_cache();

    if let Some(c) = cache.get(path)
        && c.epoch == epoch
    {
        return Some(*c);
    }

    let md = symlink_metadata(path).ok();
    let c = CachedMetaKey {
        epoch,
        size: md.as_ref().filter(|m| m.is_file()).map(|m| m.len()),
        modified: md.as_ref().and_then(|m| m.modified().ok()),
        created: md.as_ref().and_then(|m| m.created().ok()),
        accessed: md.as_ref().and_then(|m| m.accessed().ok()),
    };
    cache.insert(path.to_path_buf(), c);
    Some(c)
}

/// Enumerator for the filye types which are then shown inside [FileMetadata]
///
/// Hold File, Directory, Symlink and Other types.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FileType {
    File,
    Directory,
    Symlink,
    Other,
}

/// Main FileMetadata struct that holds each info field for the ShowInfo overlay widget.
/// Holds name, size, modified time, attributes string, and file type.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FileMetadata {
    path: PathBuf,
    size: Option<u64>,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
    accessed: Option<SystemTime>,
    attributes: String,
    file_type: FileType,
    #[cfg(unix)]
    unix_meta: Option<unix_meta::UnixMetadata>,
}

impl FileMetadata {
    // Accessors

    /// Main file info getter used by the ShowInfo overlay functions
    /// # Returns
    /// A FileMetadata struct populated with the file's information.
    pub(crate) fn new(path: &Path) -> io::Result<FileMetadata> {
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
        let unix_meta = {
            use std::os::unix::fs::MetadataExt;
            Some(unix_meta::UnixMetadata {
                uid: metadata.uid(),
                gid: metadata.gid(),
            })
        };

        Ok(FileMetadata {
            path: path.to_path_buf(),
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
    pub(crate) fn modified(&self, ctx: &TimeFormatCtx) -> String {
        format_file_time(self.modified, ctx)
    }

    #[inline]
    pub(crate) fn created(&self, ctx: &TimeFormatCtx) -> String {
        format_file_time(self.created, ctx)
    }

    #[inline]
    pub(crate) fn accessed(&self, ctx: &TimeFormatCtx) -> String {
        format_file_time(self.accessed, ctx)
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

#[derive(Debug, Clone)]
pub(crate) struct FileMetadataCache {
    name: Option<Arc<str>>,
    perms: Option<Arc<str>>,
    size: Option<Arc<str>>,
    modified: Option<Arc<str>>,
    created: Option<Arc<str>>,
    accessed: Option<Arc<str>>,
    file_type: Option<Arc<str>>,
    #[cfg(unix)]
    owner: Option<Arc<str>>,
    #[cfg(unix)]
    group: Option<Arc<str>>,
}

impl FileMetadataCache {
    #[rustfmt::skip]
    pub(crate) fn from(
        meta: &FileMetadata,
        date_format: &str,
        needs: &MetadataNeeds,
        now: DateTime<Local>,
        #[cfg(unix)] ug_cache: &mut unix_meta::UserGroupCache,
    ) -> Self {
        let ctx = TimeFormatCtx::new(date_format, now);

        Self {
            name:       if needs.name { Some(Arc::from(meta.name())) } else { None },
            perms:      if needs.perms { Some(Arc::from(meta.perms())) } else { None },
            size:       if needs.size { Some(Arc::from(meta.size())) } else { None },
            modified:   if needs.modified { Some(Arc::from(meta.modified(&ctx))) } else { None },
            created:    if needs.created { Some(Arc::from(meta.created(&ctx))) } else { None },
            accessed:   if needs.accessed { Some(Arc::from(meta.accessed(&ctx))) } else { None },
            file_type:  if needs.file_type { Some(Arc::from(meta.file_type())) } else { None },
            #[cfg(unix)]
            owner:      if needs.owner { Some(ug_cache.resolve_user(meta.uid())) } else { None },
            #[cfg(unix)]
            group:      if needs.group { Some(ug_cache.resolve_group(meta.gid())) } else { None },
        }
    }

    crate::option_arc_str_getters! {
        name,
        perms,
        size,
        modified,
        created,
        accessed,
        file_type,
    }

    #[cfg(unix)]
    pub(crate) fn owner(&self) -> &str {
        self.owner.as_deref().unwrap_or_default()
    }

    #[cfg(unix)]
    pub(crate) fn group(&self) -> &str {
        self.group.as_deref().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MetadataNeeds {
    name: bool,
    file_type: bool,
    size: bool,
    modified: bool,
    created: bool,
    accessed: bool,
    perms: bool,
    #[cfg(unix)]
    owner: bool,
    #[cfg(unix)]
    group: bool,
}

impl MetadataNeeds {
    pub(crate) fn from_show_info(info: &ShowInfoOptions) -> Self {
        Self {
            name: info.name(),
            file_type: info.file_type(),
            size: info.size(),
            modified: info.modified(),
            created: info.created(),
            accessed: info.accessed(),
            perms: info.perms(),
            #[cfg(unix)]
            owner: info.owner(),
            #[cfg(unix)]
            group: info.group(),
        }
    }
}

#[cfg(unix)]
pub(crate) mod unix_meta {
    use std::collections::HashMap;
    use std::sync::Arc;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub(crate) struct UnixMetadata {
        pub(crate) uid: u32,
        pub(crate) gid: u32,
    }

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
    fn file_metada_basic_file() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = TempDir::new()?;
        let file_path = tmp.path().join("hello.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "abc123")?;

        let info = FileMetadata::new(&file_path)?;
        assert_eq!(&info.file_type, &FileType::File);
        assert_eq!(info.name(), "hello.txt");
        assert!(info.size.is_some());
        Ok(())
    }

    #[test]
    fn file_metada_directory() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = TempDir::new()?;
        let dir_path = tmp.path().join("emptydir");
        fs::create_dir(&dir_path)?;

        let info = FileMetadata::new(&dir_path)?;
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
    fn file_metada_empty_name() {
        let info = FileMetadata {
            path: PathBuf::from(""),
            size: None,
            modified: None,
            created: None,
            accessed: None,
            attributes: "".into(),
            file_type: FileType::Other,
            #[cfg(unix)]
            unix_meta: None,
        };

        assert_eq!(info.name(), "");
        assert_eq!(info.file_type(), format_file_type(&FileType::Other));
        assert_eq!(info.size(), format_file_size(None, false));
    }

    #[test]
    fn file_metada_time_formatting_none() {
        let now = Local::now();
        let fmt = "%Y-%m-%d %H:%M";
        let ctx = TimeFormatCtx::new(fmt, now);
        let info = FileMetadata {
            path: PathBuf::from("dummy"),
            size: None,
            modified: None,
            created: None,
            accessed: None,
            attributes: "".into(),
            file_type: FileType::File,
            #[cfg(unix)]
            unix_meta: None,
        };

        assert_eq!(info.modified(&ctx), format_file_time(None, &ctx));
        assert_eq!(info.created(&ctx), format_file_time(None, &ctx),);
        assert_eq!(info.accessed(&ctx), format_file_time(None, &ctx));
    }

    #[test]
    fn file_metada_perms_width() {
        let info = FileMetadata {
            path: PathBuf::from("file.txt"),
            size: None,
            modified: None,
            created: None,
            accessed: None,
            attributes: "rwx".into(),
            file_type: FileType::File,
            #[cfg(unix)]
            unix_meta: None,
        };

        let perms = info.perms();
        assert!(perms.len() >= 3);
        assert!(perms.contains("rwx"));
    }

    #[test]
    #[cfg(unix)]
    fn file_metada_unix_meta_defaults() {
        let info = FileMetadata {
            path: PathBuf::from("dummy"),
            size: None,
            modified: None,
            created: None,
            accessed: None,
            attributes: "".into(),
            file_type: FileType::File,
            unix_meta: None,
        };

        // Should fall back to 0
        assert_eq!(info.uid(), 0);
        assert_eq!(info.gid(), 0);
    }

    #[test]
    #[cfg(unix)]
    fn test_file_metada_symlink() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = TempDir::new()?;
        let target_path = tmp.path().join("target.txt");
        File::create(&target_path)?;

        let link_path = tmp.path().join("link.txt");
        std::os::unix::fs::symlink(&target_path, &link_path)?;

        let info = FileMetadata::new(&link_path)?;
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
        let info = FileMetadata::new(&file_path)?;

        assert!(info.unix_meta.is_some());
        assert!(info.uid() > 0 || info.uid() == 0);
        Ok(())
    }
}
