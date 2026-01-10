//! Tests for the `get_unused_path` utility function.
//! These tests ensure that the function correctly generates unused file paths
//!
//! Is used by correctly handling name collisions by appending numerical suffixes.
//! Temporary directories and files are created for testing purposes and
//! are automatically cleaned up after the tests complete.

use runa_tui::utils::get_unused_path;
use std::error;
use std::fs::File;
use tempfile::tempdir;

#[test]
fn test_path_collision_increments() -> Result<(), Box<dyn error::Error>> {
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
fn test_hidden_file_collision() -> Result<(), Box<dyn error::Error>> {
    let dir = tempdir()?;
    let path = dir.path().join(".config");

    File::create(&path)?;
    // Result: .config_1
    assert_eq!(get_unused_path(&path), dir.path().join(".config_1"));
    Ok(())
}

#[test]
fn test_get_unused_path_basic() -> Result<(), Box<dyn error::Error>> {
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
fn test_get_unused_path_no_extension() -> Result<(), Box<dyn error::Error>> {
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
fn test_get_unused_path_hidden_file() -> Result<(), Box<dyn error::Error>> {
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
fn test_get_unused_path_complex_extension() -> Result<(), Box<dyn error::Error>> {
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
