use rand::rng;
use rand::seq::SliceRandom;
use runa_tui::app::NavState;
use runa_tui::file_manager::browse_dir;
use std::fs;
use std::fs::File;
use tempfile::tempdir;

#[test]
fn test_navstate_rapid_navigation() {
    let dir = tempdir().expect("failed to create sandbox");
    let file_count = 10;

    for i in 0..file_count {
        let file_path = dir.path().join(format!("testfile_{i}.txt"));
        File::create(&file_path).expect("failed to create dummy file");
    }

    let entries = browse_dir(dir.path()).expect("failed to read tempdir");
    assert!(!entries.is_empty(), "sandbox should not be empty");

    let mut nav = NavState::new(dir.path().to_path_buf());
    nav.update_from_worker(dir.path().to_path_buf(), entries.clone(), None);

    assert_eq!(
        nav.entries().len(),
        file_count,
        "initial entry count mismatch"
    );

    let down_presses = 100000;
    for _ in 0..down_presses {
        assert!(nav.move_down(), "nav.move_down() failed during stress");
    }

    let expected_idx = down_presses % file_count;
    assert_eq!(
        nav.selected_idx(),
        expected_idx,
        "wrong index after DOWN stress"
    );

    let selected = nav.selected_entry().expect("no entry selected after DOWN");
    assert_eq!(selected.name_str(), entries[expected_idx].name_str());

    let up_presses = 100000;
    for _ in 0..up_presses {
        assert!(nav.move_up(), "nav.move_up() failed during stress");
    }

    // Mathematical wrap-around check
    let expected_idx_up = (expected_idx + file_count - (up_presses % file_count)) % file_count;
    assert_eq!(
        nav.selected_idx(),
        expected_idx_up,
        "wrong index after UP stress"
    );

    let selected_up = nav.selected_entry().expect("no entry selected after UP");
    assert_eq!(selected_up.name_str(), entries[expected_idx_up].name_str());

    // Ensure the internal state hasn't corrupted the entry list
    for (i, entry) in nav.entries().iter().enumerate() {
        assert_eq!(
            entry.name_str(),
            entries[i].name_str(),
            "data corruption at index {i}"
        );
    }
}

#[test]
fn test_navstate_navigation_stress() {
    let base = tempdir().expect("sandbox setup failed");
    let base_path = base.path().to_path_buf();
    let subdir_path = base_path.join("subdir");
    let subsubdir_path = subdir_path.join("subsub");

    fs::create_dir(&subdir_path).unwrap();
    fs::create_dir(&subsubdir_path).unwrap();
    File::create(base_path.join("file_base.txt")).unwrap();
    File::create(subdir_path.join("file_sub.txt")).unwrap();
    File::create(subsubdir_path.join("file_subsub.txt")).unwrap();

    let base_entries = browse_dir(&base_path).unwrap();
    let sub_entries = browse_dir(&subdir_path).unwrap();
    let subsub_entries = browse_dir(&subsubdir_path).unwrap();

    let mut nav = NavState::new(base_path.clone());
    let repetitions = 1_000_000;

    for i in 0..repetitions {
        nav.set_path(subdir_path.clone());
        nav.update_from_worker(subdir_path.clone(), sub_entries.clone(), None);

        assert_eq!(nav.current_dir(), &subdir_path);
        assert!(
            nav.entries().iter().any(|e| e.name() == "subsub"),
            "Iter {i} missing subsub"
        );

        let parent_path = nav.current_dir().parent().unwrap();
        assert_eq!(parent_path, base_path, "Iter {i} parent mismatch");

        nav.set_path(subsubdir_path.clone());
        nav.update_from_worker(subsubdir_path.clone(), subsub_entries.clone(), None);

        assert_eq!(nav.current_dir(), &subsubdir_path);
        assert!(nav.entries().iter().any(|e| e.name() == "file_subsub.txt"));

        nav.set_path(subdir_path.clone());
        nav.update_from_worker(subdir_path.clone(), sub_entries.clone(), None);
        assert_eq!(nav.current_dir(), &subdir_path);

        nav.set_path(base_path.clone());
        nav.update_from_worker(base_path.clone(), base_entries.clone(), None);

        assert_eq!(nav.current_dir(), &base_path);
        assert!(nav.entries().iter().any(|e| e.name() == "subdir"));
    }
}

#[test]
fn test_navstate_selection_persistence_stress() {
    let base = tempdir().expect("sandbox setup failed");
    let base_path = base.path().to_path_buf();
    let subdir_path = base_path.join("subdir");

    fs::create_dir_all(&subdir_path).unwrap();
    for i in 0..20 {
        File::create(subdir_path.join(format!("file_{}.txt", i))).unwrap();
    }

    let base_entries = browse_dir(&base_path).unwrap();
    let sub_entries = browse_dir(&subdir_path).unwrap();

    let mut nav = NavState::new(base_path.clone());
    let repetitions = 1_000_000;

    nav.set_path(subdir_path.clone());
    nav.update_from_worker(subdir_path.clone(), sub_entries.clone(), None);

    for _ in 0..5 {
        nav.move_down();
    }
    assert_eq!(nav.selected_idx(), 5, "Initial move failed");

    for i in 0..repetitions {
        nav.set_path(base_path.clone());
        nav.update_from_worker(base_path.clone(), base_entries.clone(), None);

        nav.move_down();

        // Return to Subdir
        nav.set_path(subdir_path.clone());
        nav.update_from_worker(subdir_path.clone(), sub_entries.clone(), None);

        assert_eq!(
            nav.selected_idx(),
            5,
            "Selection lost at iteration {}. Should have stayed at 5.",
            i
        );
    }
}

#[test]
fn test_navstate_filter_persistence() {
    let dir = tempdir().expect("failed to create sandbox");
    let base_path = dir.path().to_path_buf();

    let names = vec![
        "main.rs",
        "lib.rs",
        "cargo.toml",
        "readme.md",
        "app.rs",
        "ui.rs",
        "file_manager.rs",
        "config.json",
        "styles.css",
    ];
    for name in &names {
        fs::write(base_path.join(name), "").expect("failed to write sandbox file");
    }

    let mut entries = browse_dir(&base_path).expect("failed to read sandbox");
    entries.shuffle(&mut rng());

    let mut nav = NavState::new(base_path.clone());
    nav.update_from_worker(base_path.clone(), entries, None);

    let target_name = "file_manager.rs";

    let actual_start_pos = nav
        .shown_entries()
        .position(|e| e.name_str() == target_name)
        .expect("Target not found in nav state");

    for _ in 0..actual_start_pos {
        nav.move_down();
    }

    assert_eq!(nav.selected_entry().unwrap().name_str(), target_name);

    let input_buffer = "file".to_string();
    nav.set_filter(input_buffer);

    let final_entry = nav.selected_entry().expect("Selection lost after filter");

    assert_eq!(
        final_entry.name_str(),
        target_name,
        "Selection persistence failed! Found {} instead. Filter was 'file'.",
        final_entry.name_str()
    );
}

#[test]
fn test_navstate_marker_persistence() {
    let dir = tempdir().expect("failed to create sandbox");
    let base_path = dir.path().to_path_buf();

    let names = vec!["apple.txt", "banana.txt", "crab.txt"];
    for name in &names {
        fs::write(base_path.join(name), "").expect("failed to write sandbox file");
    }

    let mut entries = browse_dir(&base_path).expect("failed to read sandbox");
    // Shuffle to ensure we arent relying on alphabetical order
    entries.shuffle(&mut rng());

    let mut nav = NavState::new(base_path.clone());
    nav.update_from_worker(base_path.clone(), entries, None);

    let to_mark = vec!["apple.txt", "banana.txt"];
    for target in to_mark {
        // Find it in the current view
        let pos = nav
            .shown_entries()
            .position(|e| e.name_str() == target)
            .expect("target not found");

        // Reset to top and move down to simulate real user navigation
        while nav.selected_idx() > 0 {
            nav.move_up();
        }
        for _ in 0..pos {
            nav.move_down();
        }

        assert_eq!(nav.selected_entry().unwrap().name_str(), target);
        nav.toggle_marker();
    }

    assert_eq!(nav.markers().len(), 2);

    nav.set_filter("crab".to_string());

    assert_eq!(nav.shown_entries_len(), 1);
    assert_eq!(nav.selected_entry().unwrap().name_str(), "crab.txt");

    let targets = nav.get_action_targets();
    assert_eq!(
        targets.len(),
        2,
        "Should target 2 marked files even if hidden"
    );
    assert!(targets.contains(&base_path.join("apple.txt")));
    assert!(targets.contains(&base_path.join("banana.txt")));
    assert!(
        !targets.contains(&base_path.join("cherry.txt")),
        "Should ignore selection when markers exist"
    );

    nav.clear_filters();
    // Navigate to apple.txt
    let apple_pos = nav
        .shown_entries()
        .position(|e| e.name_str() == "apple.txt")
        .unwrap();
    while nav.selected_idx() < apple_pos {
        nav.move_down();
    }
    while nav.selected_idx() > apple_pos {
        nav.move_up();
    }

    nav.toggle_marker();
    assert_eq!(nav.markers().len(), 1);
    assert!(nav.markers().contains(&base_path.join("banana.txt")));
}
