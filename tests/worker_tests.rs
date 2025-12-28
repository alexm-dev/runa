use crossbeam_channel::unbounded;
use rand::{Rng, rng};
use runa_tui::worker::{WorkerResponse, WorkerTask, start_worker};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

#[test]
fn test_worker_load_current_dir() {
    let (task_tx, task_rx) = unbounded();
    let (res_tx, res_rx) = unbounded();

    start_worker(task_rx, res_tx);

    let curr_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => panic!("Failed to get current dir: {}", e),
    };

    task_tx
        .send(WorkerTask::LoadDirectory {
            path: curr_dir,
            focus: None,
            dirs_first: true,
            show_hidden: false,
            show_system: false,
            case_insensitive: true,
            always_show: Arc::new(HashSet::new()),
            pane_width: 20,
            request_id: 1,
        })
        .unwrap();

    match res_rx.recv() {
        Ok(WorkerResponse::DirectoryLoaded { entries, .. }) => {
            assert!(!entries.is_empty(), "Current dir should not be empty");

            // Check display name width
            for entry in entries {
                let disp = entry.display_name();
                assert!(
                    disp.chars().count() <= 20,
                    "Entry '{}' too wide",
                    entry.name_str()
                );
            }
        }
        Ok(WorkerResponse::Error(e)) => panic!("Worker error: {}", e),
        _ => panic!("Unexpected worker response"),
    }
}

#[test]
fn stress_worker_dir_load_requests_multithreaded() {
    let temp_dir = env::temp_dir();
    let safe_subdir = temp_dir.join("runa_test_safe_dir");
    fs::create_dir_all(&safe_subdir).expect("Could not create safe test dir");

    let curr_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => panic!("Failed to get current dir: {}", e),
    };

    let dirs = vec![curr_dir, temp_dir.clone(), safe_subdir.clone()];

    let pane_base = 20;
    let thread_count = 4;
    let requests_per_thread = 250;

    let (task_tx, task_rx) = unbounded();
    let (res_tx, res_rx) = unbounded();

    start_worker(task_rx, res_tx);

    // Spawn threads to send requests in parallel
    let mut handles = Vec::new();
    for t in 0..thread_count {
        let task_tx = task_tx.clone();
        let dirs = dirs.clone();
        let pane_base = pane_base;
        handles.push(thread::spawn(move || {
            let mut rng = rng();
            for i in 0..requests_per_thread {
                let dir = &dirs[rng.random_range(0..dirs.len())];
                task_tx
                    .send(WorkerTask::LoadDirectory {
                        path: dir.clone(),
                        focus: None,
                        dirs_first: rng.random_bool(0.5),
                        show_hidden: rng.random_bool(0.5),
                        show_system: rng.random_bool(0.5),
                        case_insensitive: rng.random_bool(0.5),
                        always_show: Arc::new(HashSet::new()),
                        pane_width: pane_base + rng.random_range(0..10),
                        request_id: (t * requests_per_thread + i) as u64,
                    })
                    .expect("Couldn't send task to worker");
                if i % 50 == 0 {
                    thread::sleep(Duration::from_millis(rng.random_range(0..10)));
                }
            }
        }));
    }

    // Wait for all senders to finish
    for h in handles {
        if let Err(err) = h.join() {
            panic!("Thread panicked during stress test: {:?}", err);
        }
    }

    // Read responses
    let total_requests = thread_count * requests_per_thread;
    let mut valid_responses = 0;
    for _ in 0..total_requests {
        match res_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(WorkerResponse::DirectoryLoaded { entries, .. }) => {
                valid_responses += 1;
                for entry in &entries {
                    let disp = entry.display_name();
                    let visual_width = UnicodeWidthStr::width(disp);
                    assert!(
                        visual_width <= pane_base + 10,
                        "Entry '{}' display width {} > allowed ({}).",
                        entry.name_str(),
                        visual_width,
                        pane_base + 10
                    );
                    assert!(
                        !entry.name_str().is_empty(),
                        "Entry name_str must not be empty."
                    );
                    assert!(
                        !entry.name_str().contains('\0'),
                        "Entry name_str must not contain null."
                    );
                }
            }
            Ok(WorkerResponse::Error(e)) => panic!("Worker error: {}", e),
            Ok(_) => panic!("Unexpected WorkerResponse variant"),
            Err(_) => panic!("Missing worker response (timeout)"),
        }
    }

    assert_eq!(
        valid_responses, total_requests,
        "Not all worker requests returned results!"
    );

    // Cleanup test subdir
    if safe_subdir.exists() {
        let _ = fs::remove_dir_all(&safe_subdir);
    }
}
