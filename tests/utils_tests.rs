use runa_tui::utils::get_unused_path;
use std::fs::File;
use tempfile::tempdir;

#[test]
fn test_path_collision_increments() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("data.csv");

    assert_eq!(get_unused_path(&path.clone()), path);

    File::create(&path).unwrap();
    assert_eq!(
        get_unused_path(&path.clone()),
        dir.path().join("data_1.csv")
    );

    File::create(dir.path().join("data_1.csv")).unwrap();
    assert_eq!(get_unused_path(&path), dir.path().join("data_2.csv"));
}

#[test]
fn test_hidden_file_collision() {
    let dir = tempdir().unwrap();
    let path = dir.path().join(".config");

    File::create(&path).unwrap();
    // Result: .config_1
    assert_eq!(get_unused_path(&path), dir.path().join(".config_1"));
}

#[test]
fn test_get_unused_path_basic() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");

    let path1 = get_unused_path(&file_path);
    assert_eq!(path1, file_path);

    File::create(&file_path).unwrap();
    let path2 = get_unused_path(&file_path);
    assert_eq!(path2.file_name().unwrap(), "test_1.txt");

    File::create(&path2).unwrap();
    let path3 = get_unused_path(&file_path);
    assert_eq!(path3.file_name().unwrap(), "test_2.txt");
}

#[test]
fn test_get_unused_path_no_extension() {
    let dir = tempdir().unwrap();
    let folder_path = dir.path().join("my_folder");

    File::create(&folder_path).unwrap();
    let path = get_unused_path(&folder_path);

    // Should handle files/folders without extensions correctly
    assert_eq!(path.file_name().unwrap(), "my_folder_1");
}

#[test]
fn test_get_unused_path_hidden_file() {
    let dir = tempdir().unwrap();
    let dot_file = dir.path().join(".gitignore");

    File::create(&dot_file).unwrap();
    let path = get_unused_path(&dot_file);

    assert_eq!(path.file_name().unwrap(), ".gitignore_1");
}

#[test]
fn test_get_unused_path_complex_extension() {
    let dir = tempdir().unwrap();
    let tar_gz = dir.path().join("archive.tar.gz");

    File::create(&tar_gz).unwrap();
    let path = get_unused_path(&tar_gz);

    let name = path.file_name().unwrap().to_str().unwrap();
    assert!(name.contains("_1"), "Suffix missing");
}
