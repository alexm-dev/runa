//! Worker thread for the runa core operations.
//!
//! Handles directory reads, previews and file operatios on a background thread.
//! All results and errors are sent back via channels.
//!
//! Small changes here can have big effects since this module is tightly integrated with every part
//! of runa.
//!
//! Requests [WorkerTask] come in from the AppState or UI via channels, and results or errors
//! [WorkerResponse] go back the same way. All filesystem I/O and previews happen on these threads
//!
//! # Caution:
//! This module is a central protocol boundary. Small changes (adding or editing variants, fields, or error handling)
//! may require corresponding changes throughout state, response-handling code and UI.

use crate::config::display::PreviewMethod;
use crate::core::{
    FileEntry, FindResult, Formatter, browse_dir, find, preview_bat, safe_read_preview,
};
use crate::utils::{copy_recursive, get_unused_path};

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

/// Manages worker threads channels for different task types.
pub struct Workers {
    io_tx: Sender<WorkerTask>,
    find_tx: Sender<WorkerTask>,
    preview_tx: Sender<WorkerTask>,
    fileop_tx: Sender<WorkerTask>,
    response_rx: Receiver<WorkerResponse>,
}

/// Manages worker thread channels for different task types.
///
/// Each major operation (I/O, preview, find, file-ops) has its own dedicated worker thread.
///
/// The find worker uses a bounded channel of size 1: this design ensures that only the
/// latest find request will be processed, automatically skipping obsolete queued requests
/// from rapid-fire user input. This keeps search operations efficient, responsive, and
/// guarantees only one concurrent find per application.
impl Workers {
    /// Create the worker set.
    ///
    /// Spawns dedicated threads for I/O, preview, find and file operations.
    pub fn spawn() -> Self {
        let (io_tx, io_rx) = unbounded::<WorkerTask>();
        let (preview_tx, preview_rx) = unbounded::<WorkerTask>();
        let (find_tx, find_rx) = bounded::<WorkerTask>(1);
        let (fileop_tx, fileop_rx) = unbounded::<WorkerTask>();
        let (res_tx, response_rx) = unbounded::<WorkerResponse>();

        start_io_worker(io_rx, res_tx.clone());
        start_preview_worker(preview_rx, res_tx.clone());
        start_find_worker(find_rx, res_tx.clone());
        start_fileop_worker(fileop_rx, res_tx.clone());

        Self {
            io_tx,
            preview_tx,
            find_tx,
            fileop_tx,
            response_rx,
        }
    }

    /// Accessor the I/O worker task sender.
    pub fn io_tx(&self) -> &Sender<WorkerTask> {
        &self.io_tx
    }

    /// Accessor for the preview worker task sender.
    pub fn preview_tx(&self) -> &Sender<WorkerTask> {
        &self.preview_tx
    }

    /// Accessor for the find worker task sender.
    pub fn find_tx(&self) -> &Sender<WorkerTask> {
        &self.find_tx
    }

    /// Accessor for the file operation worker task sender.
    pub fn fileop_tx(&self) -> &Sender<WorkerTask> {
        &self.fileop_tx
    }

    /// Accessor for the worker response receiver.
    pub fn response_rx(&self) -> &Receiver<WorkerResponse> {
        &self.response_rx
    }
}

/// Tasks sent to the worker thread via channel.
///
/// Each variant describes a filesystem or a preview operation to perform.
pub enum WorkerTask {
    LoadDirectory {
        path: PathBuf,
        focus: Option<OsString>,
        dirs_first: bool,
        show_hidden: bool,
        show_system: bool,
        case_insensitive: bool,
        always_show: Arc<HashSet<OsString>>,
        request_id: u64,
    },
    LoadPreview {
        path: PathBuf,
        max_lines: usize,
        pane_width: usize,
        preview_method: PreviewMethod,
        args: Vec<OsString>,
        request_id: u64,
    },
    FileOp {
        op: FileOperation,
        request_id: u64,
    },
    FindRecursive {
        base_dir: PathBuf,
        query: String,
        max_results: usize,
        cancel: Arc<AtomicBool>,
        request_id: u64,
    },
}

/// Supported file system operations the worker can perform.
pub enum FileOperation {
    Delete(Vec<PathBuf>),
    Rename {
        old: PathBuf,
        new: PathBuf,
    },
    Copy {
        src: Vec<PathBuf>,
        dest: PathBuf,
        cut: bool,
        focus: Option<OsString>,
    },
    Create {
        path: PathBuf,
        is_dir: bool,
    },
}

/// Responses sent form the worker thread back to the main thread via the channel
///
/// Each variant delivers the result or error from a request taks.
#[derive(Debug)]
pub enum WorkerResponse {
    DirectoryLoaded {
        path: PathBuf,
        entries: Vec<FileEntry>,
        focus: Option<OsString>,
        request_id: u64,
    },
    PreviewLoaded {
        lines: Vec<String>,
        request_id: u64,
    },
    OperationComplete {
        message: String,
        request_id: u64,
        need_reload: bool,
        focus: Option<OsString>,
    },
    FindResults {
        base_dir: PathBuf,
        results: Vec<FindResult>,
        request_id: u64,
    },
    Error(String),
}

/// Starts the io worker thread, wich listens to [WorkerTask] and sends back to [WorkerResponse]
///
/// # Arguments
/// * `task_rx` - Receiver channel for incoming tasks
/// * `res_tx` - Sender channel for outgoing responses
fn start_io_worker(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::LoadDirectory {
                path,
                focus,
                dirs_first,
                show_hidden,
                show_system,
                case_insensitive,
                always_show,
                request_id,
            } = task
            else {
                continue;
            };
            match browse_dir(&path) {
                Ok(mut entries) => {
                    let formatter = Formatter::new(
                        dirs_first,
                        show_hidden,
                        show_system,
                        case_insensitive,
                        always_show,
                    );
                    formatter.filter_entries(&mut entries);
                    let _ = res_tx.send(WorkerResponse::DirectoryLoaded {
                        path,
                        entries,
                        focus,
                        request_id,
                    });
                }
                Err(e) => {
                    let _ = res_tx.send(WorkerResponse::Error(format!("I/O Error: {}", e)));
                }
            }
        }
    });
}

/// Starts the preview worker thread
///
/// # Arguments
/// * `task_rx` - Receiver channel for incoming tasks
/// * `res_tx` - Sender channel for outgoing responses
fn start_preview_worker(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::LoadPreview {
                mut path,
                mut max_lines,
                mut pane_width,
                mut preview_method,
                mut args,
                mut request_id,
            } = task
            else {
                continue;
            };

            // Coalesce multiple LoadPreview tasks to only process the latest
            while let Ok(next) = task_rx.try_recv() {
                if let WorkerTask::LoadPreview {
                    path: p,
                    max_lines: m,
                    pane_width: w,
                    preview_method: pm,
                    args: a,
                    request_id: id,
                } = next
                {
                    path = p;
                    max_lines = m;
                    pane_width = w;
                    preview_method = pm;
                    args = a;
                    request_id = id;
                }
            }

            let lines = match preview_method {
                // Use internal preview method
                PreviewMethod::Internal => safe_read_preview(&path, max_lines, pane_width),
                PreviewMethod::Bat => match preview_bat(&path, max_lines, args.as_slice()) {
                    // Bat preview succeeded
                    // If bat fails, fallback to internal preview
                    // If bat is not installed or returns error, we fallback to internal preview
                    Ok(lines) => lines,
                    Err(_) => safe_read_preview(&path, max_lines, pane_width),
                },
            };
            let _ = res_tx.send(WorkerResponse::PreviewLoaded { lines, request_id });
        }
    });
}

/// Starts the find worker thread
///
/// # Arguments
/// * `task_rx` - Receiver channel for incoming tasks
/// * `res_tx` - Sender channel for outgoing responses
fn start_find_worker(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::FindRecursive {
                mut base_dir,
                mut query,
                mut max_results,
                mut request_id,
                mut cancel,
            } = task
            else {
                continue;
            };

            while let Ok(next) = task_rx.try_recv() {
                if let WorkerTask::FindRecursive {
                    base_dir: base,
                    query: q,
                    max_results: max,
                    request_id: id,
                    cancel: c,
                } = next
                {
                    base_dir = base;
                    query = q;
                    max_results = max;
                    request_id = id;
                    cancel = c;
                }
            }

            let mut results = Vec::new();
            let _ = find(
                &base_dir,
                &query,
                &mut results,
                Arc::clone(&cancel),
                max_results,
            );
            if results.len() > max_results {
                results.truncate(max_results);
            }

            if cancel.load(Ordering::Acquire) {
                continue;
            }

            let _ = res_tx.send(WorkerResponse::FindResults {
                base_dir,
                results,
                request_id,
            });
        }
    });
}

/// Starts the file operation worker thread
///
/// # Arguments
/// * `task_rx` - Receiver channel for incoming tasks
/// * `res_tx` - Sender channel for outgoing responses
fn start_fileop_worker(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::FileOp { op, request_id } = task else {
                continue;
            };
            let mut focus_target: Option<OsString> = None;
            let result: Result<String, String> = match op {
                FileOperation::Delete(paths) => {
                    for p in paths {
                        let res = if p.is_dir() {
                            std::fs::remove_dir_all(&p)
                        } else {
                            std::fs::remove_file(&p)
                        };
                        if let Err(e) = res {
                            eprintln!("Failed to delete {}: {}", p.display(), e);
                        }
                    }
                    Ok("Items deleted".to_string())
                }
                FileOperation::Rename { old, new } => {
                    let target = new;

                    if target.exists() {
                        Err(format!(
                            "Rename failed: '{}' already exists",
                            target.file_name().unwrap_or_default().to_string_lossy()
                        ))
                    } else {
                        focus_target = target.file_name().map(|n| n.to_os_string());
                        std::fs::rename(old, &target)
                            .map(|_| "Renamed".into())
                            .map_err(|e| e.to_string())
                    }
                }
                FileOperation::Create { path, is_dir } => {
                    let target = get_unused_path(&path);
                    focus_target = target.file_name().map(|n| n.to_os_string());

                    let res = if is_dir {
                        std::fs::create_dir_all(&target)
                    } else {
                        std::fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(&target)
                            .map(|_| ())
                    };
                    res.map(|_| "Created".into()).map_err(|e| e.to_string())
                }
                FileOperation::Copy {
                    src,
                    dest,
                    cut,
                    focus,
                } => {
                    focus_target = focus;
                    for s in src {
                        if let Some(name) = s.file_name() {
                            let target = get_unused_path(&dest.join(name));

                            if let Some(ref ft) = focus_target
                                && ft == name
                            {
                                focus_target = target.file_name().map(|n| n.to_os_string());
                            }

                            let _ = if cut {
                                std::fs::rename(s, &target)
                            } else if s.is_dir() {
                                copy_recursive(&s, &target)
                            } else {
                                std::fs::copy(s, &target).map(|_| ())
                            };
                        }
                    }
                    Ok("Pasted".into())
                }
            };

            match result {
                Ok(msg) => {
                    let _ = res_tx.send(WorkerResponse::OperationComplete {
                        message: msg,
                        request_id,
                        need_reload: true,
                        focus: focus_target,
                    });
                }
                Err(e) => {
                    let _ = res_tx.send(WorkerResponse::Error(format!("Op Error: {}", e)));
                }
            }
        }
    });
}

/// Worker threads integration tests.
#[cfg(test)]
mod tests {
    use super::*;

    use rand::{Rng, rng};
    use std::collections::HashSet;
    use std::env;
    use std::fs::{self, File};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn test_worker_load_current_dir() -> Result<(), Box<dyn std::error::Error>> {
        let workers = Workers::spawn();
        let task_tx = workers.io_tx();
        let res_rx = workers.response_rx();

        let curr_dir = env::current_dir()?;

        task_tx.send(WorkerTask::LoadDirectory {
            path: curr_dir,
            focus: None,
            dirs_first: true,
            show_hidden: false,
            show_system: false,
            case_insensitive: true,
            always_show: Arc::new(HashSet::new()),
            request_id: 1,
        })?;

        match res_rx.recv()? {
            WorkerResponse::DirectoryLoaded { entries, .. } => {
                assert!(!entries.is_empty(), "Current dir should not be empty");

                // Check display name width
                for entry in entries {
                    assert!(!entry.name_str().is_empty());
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

        let thread_count = 2;
        let requests_per_thread = 25;

        let workers = Workers::spawn();
        let task_tx = workers.io_tx();
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
                    task_tx
                        .send(WorkerTask::LoadDirectory {
                            path: dir.clone(),
                            focus: None,
                            dirs_first: rng.random_bool(0.5),
                            show_hidden: rng.random_bool(0.5),
                            show_system: rng.random_bool(0.5),
                            case_insensitive: rng.random_bool(0.5),
                            always_show: Arc::new(HashSet::new()),
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
                        let name = entry.name_str();

                        assert!(!name.is_empty(), "Entry name_str must not be empty.");
                        assert!(
                            !name.contains('\0'),
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

    #[test]
    fn test_find_worker_finds_file() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::File::create(temp.path().join("crab.txt"))?;

        let workers = Workers::spawn();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        workers.find_tx().send(WorkerTask::FindRecursive {
            base_dir: temp.path().to_path_buf(),
            query: "crab".to_string(),
            max_results: 5,
            cancel: cancel.clone(),
            request_id: 2,
        })?;

        let resp = workers
            .response_rx()
            .recv_timeout(std::time::Duration::from_secs(2))?;

        match resp {
            WorkerResponse::FindResults { results, .. } => {
                if !results
                    .iter()
                    .any(|res| res.path().file_name().unwrap() == "crab.txt")
                {
                    return Err("Expected 'crab.txt' in find results".into());
                }
            }
            r => return Err(format!("Unexpected response: {:?}", r).into()),
        }
        Ok(())
    }

    #[test]
    fn test_preview_worker_internal() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let preview_file = temp.path().join("preview.txt");
        std::fs::write(&preview_file, "A\nB\nC\nD\n")?;
        let workers = Workers::spawn();
        workers.preview_tx().send(WorkerTask::LoadPreview {
            path: preview_file.clone(),
            max_lines: 2,
            pane_width: 40,
            preview_method: PreviewMethod::Internal,
            args: vec![],
            request_id: 3,
        })?;

        match workers
            .response_rx()
            .recv_timeout(std::time::Duration::from_secs(2))?
        {
            WorkerResponse::PreviewLoaded { lines, .. } => {
                let previewed: Vec<_> = lines.iter().take(2).map(|s| s.trim_end()).collect();
                if previewed != vec!["A", "B"] {
                    return Err(format!("Preview did not match expected, got {:?}", lines).into());
                }
            }
            r => return Err(format!("Unexpected response: {:?}", r).into()),
        }
        Ok(())
    }

    #[test]
    fn test_fileop_worker_create_and_delete_file() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let file_path = temp.path().join("touch.txt");
        let workers = Workers::spawn();

        workers.fileop_tx().send(WorkerTask::FileOp {
            op: FileOperation::Create {
                path: file_path.clone(),
                is_dir: false,
            },
            request_id: 4,
        })?;

        let r = workers
            .response_rx()
            .recv_timeout(std::time::Duration::from_secs(2))?;
        match r {
            WorkerResponse::OperationComplete { .. } => {
                if !file_path.exists() {
                    return Err("Expected file to exist after creation".into());
                }
            }
            other => return Err(format!("Unexpected response: {:?}", other).into()),
        }

        workers.fileop_tx().send(WorkerTask::FileOp {
            op: FileOperation::Delete(vec![file_path.clone()]),
            request_id: 5,
        })?;

        let r = workers
            .response_rx()
            .recv_timeout(std::time::Duration::from_secs(2))?;
        match r {
            WorkerResponse::OperationComplete { .. } => {
                if file_path.exists() {
                    return Err("Expected file to not exist after deletion".into());
                }
            }
            other => return Err(format!("Unexpected response: {:?}", other).into()),
        }
        Ok(())
    }
}
