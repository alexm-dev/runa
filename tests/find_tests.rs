use runa_tui::core::find::find;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tempfile::tempdir;

fn fd_available() -> bool {
    which::which("fd").is_ok()
}

macro_rules! skip_if_no_fd {
    () => {
        if !fd_available() {
            return Ok(());
        }
    };
}

#[test]
fn test_find_recursive_unit() -> Result<(), Box<dyn std::error::Error>> {
    skip_if_no_fd!();

    let dir = tempdir()?;
    std::fs::File::create(dir.path().join("crab.txt"))?;
    std::fs::File::create(dir.path().join("other.txt"))?;
    let cancel = Arc::new(AtomicBool::new(false));
    let mut out = Vec::new();
    find(dir.path(), "crab", &mut out, cancel, 11)?;
    let candidate = out
        .iter()
        .find(|r| r.path().file_name().unwrap() == "crab.txt");
    assert!(
        candidate.is_some(),
        "Expected 'crab.txt' in find results. Got: {:?}",
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
    skip_if_no_fd!();
    let dir = tempdir()?;
    fs::File::create(dir.path().join("something.txt"))?;
    let cancel = Arc::new(AtomicBool::new(false));
    let mut out = Vec::new();
    find(dir.path(), "", &mut out, cancel, 10)?;
    assert!(out.is_empty());
    Ok(())
}

#[test]
fn test_find_recursive_subdirectory() -> Result<(), Box<dyn std::error::Error>> {
    skip_if_no_fd!();
    let dir = tempdir()?;
    let subdir = dir.path().join("nested");
    std::fs::create_dir(&subdir)?;
    std::fs::File::create(subdir.join("crabby.rs"))?;
    let cancel = Arc::new(AtomicBool::new(false));
    let mut out = Vec::new();
    find(dir.path(), "crabby", &mut out, cancel, 10)?;
    let candidate = out
        .iter()
        .find(|r| r.path().file_name().unwrap() == "crabby.rs");
    assert!(
        candidate.is_some(),
        "Expected 'crabby.rs' in find results. Got: {:?}",
        out.iter()
            .map(|r| r.path().display().to_string())
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_find_recursive_reports_dir() -> Result<(), Box<dyn std::error::Error>> {
    skip_if_no_fd!();
    let dir = tempdir()?;
    let subdir = dir.path().join("crabdir");
    fs::create_dir(&subdir)?;
    let cancel = Arc::new(AtomicBool::new(false));
    let mut out = Vec::new();
    find(dir.path(), "crab", &mut out, cancel, 10)?;
    assert!(out.iter().any(|r| r.is_dir()));
    Ok(())
}
