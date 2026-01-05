use runa_tui::core::find::find_recursive;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tempfile::tempdir;

#[test]
fn test_find_recursive_unit() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    std::fs::File::create(dir.path().join("crab.txt"))?;
    std::fs::File::create(dir.path().join("other.txt"))?;
    let cancel = Arc::new(AtomicBool::new(false));
    let mut out = Vec::new();
    find_recursive(dir.path(), "crab", &mut out, cancel, 10)?;
    assert_eq!(
        out.len(),
        1,
        "Expected 1 result for 'crab', got {}: {:?}",
        out.len(),
        out.iter()
            .map(|r| r.path().display().to_string())
            .collect::<Vec<_>>()
    );

    let filename = out[0]
        .path()
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Could not extract valid UTF-8 file name")?;
    assert!(
        filename.contains("crab"),
        "Filename does not contain 'crab': got '{}'",
        filename
    );
    Ok(())
}

#[test]
fn test_find_recursive_empty_query() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    fs::File::create(dir.path().join("something.txt"))?;
    let cancel = Arc::new(AtomicBool::new(false));
    let mut out = Vec::new();
    find_recursive(dir.path(), "", &mut out, cancel, 10)?;
    assert!(out.is_empty());
    Ok(())
}

#[test]
fn test_find_recursive_subdirectory() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let subdir = dir.path().join("nested");
    std::fs::create_dir(&subdir)?;
    std::fs::File::create(subdir.join("crabby.rs"))?;
    let cancel = Arc::new(AtomicBool::new(false));
    let mut out = Vec::new();
    find_recursive(dir.path(), "crabby", &mut out, cancel, 10)?;
    assert_eq!(
        out.len(),
        1,
        "Expected 1 result for 'crabby', got {}: {:?}",
        out.len(),
        out.iter()
            .map(|r| r.path().display().to_string())
            .collect::<Vec<_>>()
    );
    let filename = out[0]
        .path()
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Could not extract valid UTF-8 file name")?;

    assert!(
        filename.contains("crabby"),
        "Filename does not contain 'crabby': got '{}'",
        filename
    );

    Ok(())
}

#[test]
fn test_find_recursive_reports_dir() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let subdir = dir.path().join("crabdir");
    fs::create_dir(&subdir)?;
    let cancel = Arc::new(AtomicBool::new(false));
    let mut out = Vec::new();
    find_recursive(dir.path(), "crab", &mut out, cancel, 10)?;
    assert!(out.iter().any(|r| r.is_dir()));
    Ok(())
}
