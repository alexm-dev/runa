//! IO worker thread module.
//!
//! Handles the nav, parent, preview I/O operations.

use std::sync::Arc;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use dashmap::DashMap;

use crate::core::{
    cache::DirCache,
    fm::{self, FileEntry},
    formatter::Formatter,
    workers::{LoadDirTask, WorkerResponse},
};
use crate::utils::text::StrBuffer;

/// Starts the io worker thread, wich listens to [LoadDirTask] and sends back to [WorkerResponse]
pub(super) fn start(
    task_rx: Receiver<LoadDirTask>,
    res_tx: Sender<WorkerResponse>,
    cache: Arc<DirCache>,
) {
    thread::spawn(move || {
        while let Ok(LoadDirTask {
            path,
            focus,
            list,
            sort_config,
            sort_date_format,
            always_show,
            request_id,
            tab_id,
        }) = task_rx.recv()
        {
            match fm::browse_dir(&path) {
                Ok(mut entries) => {
                    let meta_cache = DashMap::with_capacity(entries.len());
                    let formatter = Formatter::new(list.clone(), sort_config, always_show);
                    formatter.filter_entries(&mut entries);
                    let sort_column =
                        formatter.sort_entries(&path, &mut entries, &sort_date_format, &meta_cache);

                    let entries_arc: Arc<[FileEntry]> = Arc::from(entries);
                    let sort_column_arc: Option<Arc<StrBuffer>> =
                        sort_column.map(|v| Arc::new(StrBuffer::from_iter(v)));

                    cache.insert_if_newer(
                        &path,
                        sort_config,
                        &list,
                        Arc::clone(&entries_arc),
                        sort_column_arc.clone(),
                        request_id,
                    );

                    let _ = res_tx.send(WorkerResponse::DirectoryLoaded {
                        path,
                        entries: entries_arc,
                        focus,
                        sort_column: sort_column_arc,
                        request_id,
                        tab_id,
                    });
                }
                Err(e) => {
                    let _ = res_tx.send(WorkerResponse::DirectoryFailed {
                        path,
                        message: format!("I/O Error: {}", e),
                        request_id,
                        tab_id,
                    });
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::core::cache::DirListOptions;
    use crate::core::sort::SortConfig;
    use crate::core::workers::test_support::{TEST_TAB_ID, TEST_TIMEOUT, test_list_opts};
    use crate::core::workers::{LoadDirTask, WorkerResponse, Workers};

    use rand::{RngExt, rng};
    use std::collections::HashSet;
    use std::fs;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    #[test]
    fn worker_load_current_dir() -> Result<(), Box<dyn std::error::Error>> {
        let workers = Workers::spawn();
        let temp = tempdir()?;
        let task_tx = &workers.nav_io_tx;
        let res_rx = workers.response_rx();

        let temp_path = temp.path().join("test_dir");
        fs::create_dir(&temp_path)?;
        fs::File::create(temp_path.join("crab.txt"))?;

        task_tx.send(LoadDirTask {
            path: temp_path,
            focus: None,
            list: test_list_opts(),
            sort_date_format: Arc::<str>::from(""),
            sort_config: SortConfig::default(),
            always_show: Arc::new(HashSet::new()),
            request_id: 1,
            tab_id: TEST_TAB_ID,
        })?;

        match res_rx.recv()? {
            WorkerResponse::DirectoryLoaded { entries, .. } => {
                assert!(!entries.is_empty(), "Current dir should not be empty");

                // Check display name width
                for entry in entries.iter().enumerate() {
                    assert!(!entry.1.name_str().is_empty());
                }
            }
            WorkerResponse::DirectoryFailed { message, .. } => panic!("Worker error: {}", message),
            _ => panic!("Unexpected worker response"),
        }
        Ok(())
    }

    #[test]
    fn worker_dir_load_requests_multithreaded() -> Result<(), Box<dyn std::error::Error>> {
        let temp_root = tempdir()?;

        let dir_a = temp_root.path().join("dir_a");
        let dir_b = temp_root.path().join("dir_b");
        fs::create_dir(&dir_a)?;
        fs::create_dir(&dir_b)?;
        fs::write(dir_a.join("file.txt"), "content")?;

        let dirs = vec![temp_root.path().to_path_buf(), dir_a, dir_b];

        let thread_count = 2;
        let requests_per_thread = 25;

        let workers = Workers::spawn();
        let task_tx = &workers.nav_io_tx;
        let res_rx = workers.response_rx();

        // Spawn threads to send requests in parallel
        let mut handles = Vec::new();
        for t in 0..thread_count {
            let task_tx = task_tx.clone();
            let dirs = dirs.clone();
            handles.push(thread::spawn(move || {
                let mut rng = rng();
                for i in 0..requests_per_thread {
                    let dir = &dirs[rng.random_range(0..dirs.len())];
                    let list = DirListOptions {
                        dirs_first: rng.random_bool(0.5),
                        show_hidden: rng.random_bool(0.5),
                        show_symlink: rng.random_bool(0.5),
                        show_system: rng.random_bool(0.5),
                        case_insensitive: rng.random_bool(0.5),
                    };
                    task_tx
                        .send(LoadDirTask {
                            path: dir.clone(),
                            focus: None,
                            list,
                            sort_config: SortConfig::default(),
                            sort_date_format: Arc::<str>::from(""),
                            always_show: Arc::new(HashSet::new()),
                            request_id: (t * requests_per_thread + i) as u64,
                            tab_id: TEST_TAB_ID,
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
        let timeout = Instant::now() + TEST_TIMEOUT;

        for _ in 0..total_requests {
            let remaining = timeout.saturating_duration_since(Instant::now());
            match res_rx.recv_timeout(remaining.min(Duration::from_millis(500))) {
                Ok(WorkerResponse::DirectoryLoaded { entries, .. }) => {
                    valid_responses += 1;
                    for entry in entries.iter().enumerate() {
                        let name = entry.1.name_str();

                        assert!(!name.is_empty(), "Entry name_str must not be empty.");
                        assert!(
                            !name.contains('\0'),
                            "Entry name_str must not contain null."
                        );
                    }
                }
                Ok(WorkerResponse::DirectoryFailed { message, .. }) => {
                    panic!("Worker error: {}", message)
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }

        assert_eq!(
            valid_responses, total_requests,
            "Not all worker requests returned results!"
        );
        Ok(())
    }
}
