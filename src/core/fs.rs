//! Filesystem relevant functions

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::{fs, io};

const DENY: &[&str] = &["a", "lib", "ilk", "h5", "zip", "gz", "tar", "pdb"];

#[rustfmt::skip]
const TEMP_EXTS: &[&str] = &[
    "crdownload", "part", "partial", "download", "opdownload",
    "aria2", "tmp", "temp", "swp", "swo", "swx",
];

/// Recursively copies files and directories from `src` to `dest`, with safety checks.
///
/// Safety checks prevent copying a directory into its own subdirectory,
/// Returns an Error if such an operation is attempted.
pub(crate) fn copy_recursive(src: &Path, dest: &Path) -> io::Result<()> {
    let src_canon = src.canonicalize()?;
    let dest_parent = dest
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Destination has no parent"))?;
    let dest_parent_canon = dest_parent.canonicalize().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination parent does not exist",
        )
    })?;
    let file_name = dest.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "Destination has no file name")
    })?;
    let dest_canon = dest_parent_canon.join(file_name);

    if dest_canon.starts_with(&src_canon) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cannot copy a directory into its own subdirectory",
        ));
    }

    copy_recursive_inner(src, dest)
}

/// Internal helper function to perform the actual recursive copy.
/// Handles directories, files, and symbolic links appropriately.
/// This function is called by [copy_recursive] after performing safety checks.
fn copy_recursive_inner(src: &Path, dest: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(src)?;

    if meta.is_dir() {
        let entries = fs::read_dir(src)?;
        fs::create_dir_all(dest)?;

        for entry in entries {
            let entry = entry?;
            copy_recursive_inner(&entry.path(), &dest.join(entry.file_name()))?;
        }
    } else if meta.file_type().is_symlink() {
        let target = fs::read_link(src)?;
        #[cfg(windows)]
        {
            let is_dir_target = fs::metadata(src).map(|m| m.is_dir()).unwrap_or(false);

            if is_dir_target {
                std::os::windows::fs::symlink_dir(&target, dest)?;
            } else {
                std::os::windows::fs::symlink_file(&target, dest)?;
            }
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, dest)?;
        }
    } else {
        fs::copy(src, dest)?;
    }

    Ok(())
}

/// Finds the next available filename by appending _1, _2, etc. if the target exists
///
/// Example: "notes.txt" -> "notes_1.txt"
pub(crate) fn get_unused_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let name = path.file_name().unwrap_or_default();

    let stem = Path::new(name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();

    let ext = Path::new(name)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    let mut counter = 1;
    loop {
        let new_name = format!("{}_{}{}", stem, counter, ext);
        let target = parent.join(new_name);
        if !target.exists() {
            return target;
        }
        counter += 1;
    }
}

pub(crate) fn merge_dir(src: &Path, dst: &Path, overwrite: bool) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    if dst.starts_with(src) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cannot move a directory into itself",
        ));
    }

    for entry_res in fs::read_dir(src)? {
        let entry = entry_res?;
        let name = entry.file_name();
        let src_path = entry.path();
        let dst_path = dst.join(&name);

        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            if dst_path.exists() {
                let dst_is_dir = fs::symlink_metadata(&dst_path)?.is_dir();

                if dst_is_dir {
                    merge_dir(&src_path, &dst_path, overwrite)?;

                    let _ = fs::remove_dir(&src_path);
                } else {
                    if overwrite {
                        fs::remove_file(&dst_path)?;
                        rename_with_fallback(&src_path, &dst_path, true)?;
                    } else {
                        let unique = get_unused_path(&dst_path);
                        rename_with_fallback(&src_path, &unique, true)?;
                    }
                }
            } else {
                rename_with_fallback(&src_path, &dst_path, true)?;
            }
        } else {
            if dst_path.exists() {
                let dst_is_dir = fs::symlink_metadata(&dst_path)?.is_dir();

                if overwrite {
                    if dst_is_dir {
                        let unique = get_unused_path(&dst_path);
                        rename_with_fallback(&src_path, &unique, false)?;
                    } else {
                        fs::remove_file(&dst_path)?;
                        rename_with_fallback(&src_path, &dst_path, false)?;
                    }
                } else {
                    let unique = get_unused_path(&dst_path);
                    rename_with_fallback(&src_path, &unique, false)?;
                }
            } else {
                rename_with_fallback(&src_path, &dst_path, false)?;
            }
        }
    }

    let _ = fs::remove_dir(src);

    Ok(())
}

/// Helper function to safely handle EXDEV (Cross-device link) errors
/// when moving files or directories between different drives/partitions.
pub(crate) fn rename_with_fallback(src: &Path, dst: &Path, is_dir: bool) -> io::Result<()> {
    match fs::rename(src, dst) {
        Ok(_) => Ok(()),
        Err(_) => {
            if is_dir {
                copy_recursive(src, dst)?;
                fs::remove_dir_all(src)
            } else {
                fs::copy(src, dst)?;
                fs::remove_file(src)
            }
        }
    }
}

/// Check for file extension to deny file previews
pub(crate) fn is_preview_deny(path: &Path) -> bool {
    match path.extension() {
        Some(ext) => DENY.iter().any(|&s| ext == OsStr::new(s)),
        None => false,
    }
}

/// Checks if a file is likely a temporary file based on its extension or naming patterns.
/// Will be used to exclude temp files from file previews.
pub(crate) fn is_temp_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext = ext.to_ascii_lowercase();
        if TEMP_EXTS.contains(&ext.as_str()) {
            return true;
        }
    }

    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name.ends_with('~') || name.starts_with("~$") || name.starts_with(".~"))
}

/// Helper utils integration tests
#[cfg(test)]
mod tests {
    use super::*;

    use std::error;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn path_collision_increments() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let path = dir.path().join("data.csv");

        assert_eq!(get_unused_path(&path.clone()), path);

        File::create(&path)?;
        assert_eq!(
            get_unused_path(&path.clone()),
            dir.path().join("data_1.csv")
        );

        File::create(dir.path().join("data_1.csv"))?;
        assert_eq!(get_unused_path(&path), dir.path().join("data_2.csv"));
        Ok(())
    }

    #[test]
    fn hidden_file_collision() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let path = dir.path().join(".config");

        File::create(&path)?;
        // Result: .config_1
        assert_eq!(get_unused_path(&path), dir.path().join(".config_1"));
        Ok(())
    }

    #[test]
    fn get_unused_path_basic() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");

        let path1 = get_unused_path(&file_path);
        assert_eq!(path1, file_path);

        File::create(&file_path)?;
        let path2 = get_unused_path(&file_path);
        let path2_fname = path2
            .file_name()
            .ok_or("Failed to get file name from path2")?
            .to_str()
            .ok_or("File name not valid UTF-8")?;
        assert_eq!(path2_fname, "test_1.txt");

        File::create(&path2)?;
        let path3 = get_unused_path(&file_path);
        let path3_fname = path3
            .file_name()
            .ok_or("Failed to get file name from path3")?
            .to_str()
            .ok_or("File name not valid UTF-8")?;
        assert_eq!(path3_fname, "test_2.txt");
        Ok(())
    }

    #[test]
    fn get_unused_path_no_extension() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let folder_path = dir.path().join("my_folder");

        File::create(&folder_path)?;
        let path = get_unused_path(&folder_path);

        // Should handle files/folders without extensions correctly
        let fname = path
            .file_name()
            .ok_or("No file name in path")?
            .to_str()
            .ok_or("File name not valid UTF-8")?;
        assert_eq!(fname, "my_folder_1");
        Ok(())
    }

    #[test]
    fn get_unused_path_hidden_file() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let dot_file = dir.path().join(".gitignore");

        File::create(&dot_file)?;
        let path = get_unused_path(&dot_file);

        let fname = path
            .file_name()
            .ok_or("No file name in path")?
            .to_str()
            .ok_or("File name not valid UTF-8")?;
        assert_eq!(fname, ".gitignore_1");
        Ok(())
    }

    #[test]
    fn get_unused_path_complex_extension() -> Result<(), Box<dyn error::Error>> {
        let dir = tempdir()?;
        let tar_gz = dir.path().join("archive.tar.gz");

        File::create(&tar_gz)?;
        let path = get_unused_path(&tar_gz);

        let name = path
            .file_name()
            .ok_or("No file name in path")?
            .to_str()
            .ok_or("File name not valid UTF-8")?;
        assert!(name.contains("_1"), "Suffix missing: got {:?}", name);
        Ok(())
    }

    #[test]
    fn copy_recursive_basic_file() -> Result<(), Box<dyn error::Error>> {
        let src_dir = tempdir()?;
        let dest_dir = tempdir()?;

        let file_path = src_dir.path().join("test.txt");
        fs::write(&file_path, "hello runa")?;

        let dest_path = dest_dir.path().join("test_copied.txt");
        copy_recursive(&file_path, &dest_path)?;

        assert!(dest_path.exists());
        assert_eq!(fs::read_to_string(dest_path)?, "hello runa");
        Ok(())
    }

    #[test]
    fn copy_recursive_directory_structure() -> Result<(), Box<dyn error::Error>> {
        let src_dir = tempdir()?;
        let dest_base = tempdir()?;
        let dest_path = dest_base.path().join("backup");

        let subdir = src_dir.path().join("subdir");
        fs::create_dir(&subdir)?;
        fs::write(subdir.join("inner.txt"), "nested data")?;

        copy_recursive(src_dir.path(), &dest_path)?;

        assert!(dest_path.join("subdir").is_dir());
        assert_eq!(
            fs::read_to_string(dest_path.join("subdir").join("inner.txt"))?,
            "nested data"
        );
        Ok(())
    }

    #[test]
    fn copy_recursive_prevention_subdir() -> Result<(), Box<dyn error::Error>> {
        let src_dir = tempdir()?;
        let src_path = src_dir.path();

        let dest_path = src_path.join("backup");

        let result = copy_recursive(src_path, &dest_path);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("subdirectory"));

        Ok(())
    }
}
