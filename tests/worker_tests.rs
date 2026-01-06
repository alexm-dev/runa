use crossbeam_channel::unbounded;
use rand::{Rng, rng};
use runa_tui::core::worker::{WorkerResponse, WorkerTask, Workers, start_worker};
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use unicode_width::UnicodeWidthStr;

#[test]
fn test_worker_load_current_dir() -> Result<(), Box<dyn std::error::Error>> {
    let (task_tx, task_rx) = unbounded();
    let (res_tx, res_rx) = unbounded();

    start_worker(task_rx, res_tx);

    let curr_dir = env::current_dir()?;

    task_tx.send(WorkerTask::LoadDirectory {
        path: curr_dir,
        focus: None,
        dirs_first: true,
        show_hidden: false,
        show_system: false,
        case_insensitive: true,
        always_show: Arc::new(HashSet::new()),
        pane_width: 20,
        request_id: 1,
    })?;

    match res_rx.recv()? {
        WorkerResponse::DirectoryLoaded { entries, .. } => {
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
        WorkerResponse::Error(e) => panic!("Worker error: {}", e),
        _ => panic!("Unexpected worker response"),
    }
    Ok(())
}

#[test]
fn worker_dir_load_requests_multithreaded() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let safe_subdir = temp_dir.path().join("runa_test_safe_dir");
    fs::create_dir_all(&safe_subdir)?;

    let curr_dir = env::current_dir()?;

    let dirs = vec![curr_dir, temp_dir.path().to_path_buf(), safe_subdir.clone()];

    let pane_base = 20;
    let thread_count = 2;
    let requests_per_thread = 25;

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
    Ok(())
}

fn fd_available() -> bool {
    which::which("fd").is_ok()
}

#[test]
fn test_worker_find_pool() -> Result<(), Box<dyn std::error::Error>> {
    if !fd_available() {
        return Ok(());
    }

    let dir = tempdir()?;
    for i in 0..5 {
        File::create(dir.path().join(format!("crab_{i}.txt")))?;
    }
    File::create(dir.path().join("other.txt"))?;

    let workers = Workers::spawn();
    let find_tx = workers.find_tx();
    let res_rx = workers.response_rx();

    let req_id = 42;
    find_tx.send(WorkerTask::FindRecursive {
        base_dir: dir.path().to_path_buf(),
        query: "crab".to_string(),
        max_results: 10,
        cancel: Arc::new(AtomicBool::new(false)),
        request_id: req_id,
    })?;

    let mut got = false;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let expected_files: HashSet<_> = (0..5).map(|i| format!("crab_{i}.txt")).collect();

    while std::time::Instant::now() < deadline {
        match res_rx.recv_timeout(deadline - std::time::Instant::now()) {
            Ok(WorkerResponse::FindResults {
                results,
                request_id,
                ..
            }) => {
                assert_eq!(request_id, req_id);

                let found_files: HashSet<_> = results
                    .iter()
                    .filter_map(|r| r.path().file_name())
                    .filter_map(|os| os.to_str())
                    .filter(|name| name.contains("crab"))
                    .map(|s| s.to_string())
                    .collect();

                for fname in &expected_files {
                    assert!(
                        found_files.contains(fname),
                        "Expected {fname:?} in results: {:?}",
                        found_files
                    );
                }

                for r in &results {
                    let name = r.path().file_name().unwrap().to_str().unwrap();
                    if name.contains("crab") {
                        assert!(
                            expected_files.contains(name),
                            "Unexpected crab result: {}",
                            name
                        );
                    }
                }

                got = true;
                break;
            }
            Ok(_unexpected) => {
                continue;
            }
            Err(_) => break,
        }
    }

    assert!(got, "Did not receive FindResults response in time");
    Ok(())
}
