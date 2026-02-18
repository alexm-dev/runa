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
    FileEntry, FindResult, Formatter, browse_dir, find, formatter::safe_read_preview, preview_bat,
};
use crate::utils::{copy_recursive, get_unused_path, is_preview_deny};

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

/// Manages worker threads channels for different task types.
pub(crate) struct Workers {
    nav_io_tx: Sender<WorkerTask>,
    parent_io_tx: Sender<WorkerTask>,
    preview_io_tx: Sender<WorkerTask>,
    preview_file_tx: Sender<WorkerTask>,
    find_tx: Sender<WorkerTask>,
    fileop_tx: Sender<WorkerTask>,
    response_rx: Receiver<WorkerResponse>,
    active: Arc<AtomicUsize>,
}

/// Manages worker thread channels for different task types.
///
/// Each major operation (I/O (nav, preview, parent), preview, find, file-ops) has its own dedicated worker thread.
///
/// The find worker uses a bounded channel of size 1: this design ensures that only the
/// latest find request will be processed, automatically skipping obsolete queued requests
/// from rapid-fire user input. This keeps search operations efficient, responsive, and
/// guarantees only one concurrent find per application.
impl Workers {
    /// Create the worker set.
    ///
    /// Spawns dedicated threads for I/O, preview, find and file operations.
    pub(crate) fn spawn() -> Self {
        let (nav_io_tx, nav_io_rx) = bounded::<WorkerTask>(1);
        let (parent_io_tx, parent_io_rx) = bounded::<WorkerTask>(1);
        let (preview_io_tx, preview_io_rx) = bounded::<WorkerTask>(1);
        let (preview_file_tx, preview_file_rx) = bounded::<WorkerTask>(1);
        let (find_tx, find_rx) = bounded::<WorkerTask>(1);
        let (fileop_tx, fileop_rx) = unbounded::<WorkerTask>();
        let (res_tx, response_rx) = unbounded::<WorkerResponse>();

        let active = Arc::new(AtomicUsize::new(0));
        let fileop_active_for_worker = Arc::clone(&active);

        start_io_worker(nav_io_rx, res_tx.clone());
        start_io_worker(parent_io_rx, res_tx.clone());
        start_io_worker(preview_io_rx, res_tx.clone());
        start_preview_worker(preview_file_rx, res_tx.clone());
        start_find_worker(find_rx, res_tx.clone());
        start_fileop_worker(fileop_rx, res_tx.clone(), fileop_active_for_worker);

        Self {
            nav_io_tx,
            parent_io_tx,
            preview_io_tx,
            preview_file_tx,
            find_tx,
            fileop_tx,
            response_rx,
            active,
        }
    }

    /// Accessor the I/O worker task sender.
    #[inline]
    pub(crate) fn nav_io_tx(&self) -> &Sender<WorkerTask> {
        &self.nav_io_tx
    }

    #[inline]
    pub(crate) fn parent_io_tx(&self) -> &Sender<WorkerTask> {
        &self.parent_io_tx
    }

    #[inline]
    pub(crate) fn preview_io_tx(&self) -> &Sender<WorkerTask> {
        &self.preview_io_tx
    }

    /// Accessor for the preview worker task sender.
    #[inline]
    pub(crate) fn preview_file_tx(&self) -> &Sender<WorkerTask> {
        &self.preview_file_tx
    }

    /// Accessor for the find worker task sender.
    #[inline]
    pub(crate) fn find_tx(&self) -> &Sender<WorkerTask> {
        &self.find_tx
    }

    /// Accessor for the file operation worker task sender.
    #[inline]
    pub(crate) fn fileop_tx(&self) -> &Sender<WorkerTask> {
        &self.fileop_tx
    }

    /// Accessor for the worker response receiver.
    #[inline]
    pub(crate) fn response_rx(&self) -> &Receiver<WorkerResponse> {
        &self.response_rx
    }

    #[inline]
    pub(crate) fn active(&self) -> &Arc<AtomicUsize> {
        &self.active
    }
}

struct ActiveOpGuard(Arc<AtomicUsize>);

impl ActiveOpGuard {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self(counter)
    }
}

impl Drop for ActiveOpGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Tasks sent to the worker thread via channel.
///
/// Each variant describes a filesystem or a preview operation to perform.
pub(crate) enum WorkerTask {
    LoadDirectory {
        path: PathBuf,
        focus: Option<OsString>,
        dirs_first: bool,
        show_hidden: bool,
        show_symlink: bool,
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
    },
    FindRecursive {
        base_dir: PathBuf,
        query: String,
        max_results: usize,
        cancel: Arc<AtomicBool>,
        show_hidden: bool,
        request_id: u64,
    },
}

/// Supported file system operations the worker can perform.
pub(crate) enum FileOperation {
    Delete(Vec<PathBuf>, bool),
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
pub(crate) enum WorkerResponse {
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
        need_reload: bool,
        focus: Option<OsString>,
    },
    FindResults {
        base_dir: PathBuf,
        results: Vec<FindResult>,
        request_id: u64,
    },
    Error(String, Option<u64>),
}

/// Starts the io worker thread, wich listens to [WorkerTask] and sends back to [WorkerResponse]
fn start_io_worker(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::LoadDirectory {
                path,
                focus,
                dirs_first,
                show_hidden,
                show_symlink,
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
                        show_symlink,
                        show_system,
                        case_insensitive,
                        always_show,
                    );
                    formatter.filter_entries(&mut entries);
                    formatter.sort_entries(&mut entries);
                    let _ = res_tx.send(WorkerResponse::DirectoryLoaded {
                        path,
                        entries,
                        focus,
                        request_id,
                    });
                }
                Err(e) => {
                    let _ = res_tx.send(WorkerResponse::Error(
                        format!("I/O Error: {}", e),
                        Some(request_id),
                    ));
                }
            }
        }
    });
}

/// Starts the preview worker thread
fn start_preview_worker(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::LoadPreview {
                path,
                max_lines,
                pane_width,
                preview_method,
                args,
                request_id,
            } = task
            else {
                continue;
            };

            let lines = match preview_method {
                PreviewMethod::Internal => safe_read_preview(&path, max_lines, pane_width),
                PreviewMethod::Bat => {
                    if is_preview_deny(&path) {
                        safe_read_preview(&path, max_lines, pane_width)
                    } else {
                        match preview_bat(&path, max_lines, args.as_slice()) {
                            // Bat preview succeeded
                            // If bat fails, fallback to internal preview
                            // If bat is not installed or returns error, we fallback to internal preview
                            Ok(lines) => lines,
                            Err(_) => safe_read_preview(&path, max_lines, pane_width),
                        }
                    }
                }
            };
            let _ = res_tx.send(WorkerResponse::PreviewLoaded { lines, request_id });
        }
    });
}

/// Starts the find worker thread
fn start_find_worker(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::FindRecursive {
                base_dir,
                query,
                max_results,
                cancel,
                show_hidden,
                request_id,
            } = task
            else {
                continue;
            };

            let mut results = Vec::new();
            let _ = find(
                &base_dir,
                &query,
                &mut results,
                Arc::clone(&cancel),
                max_results,
                show_hidden,
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
fn start_fileop_worker(
    task_rx: Receiver<WorkerTask>,
    res_tx: Sender<WorkerResponse>,
    active_count: Arc<AtomicUsize>,
) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let _guard = ActiveOpGuard::new(Arc::clone(&active_count));

            let WorkerTask::FileOp { op } = task else {
                continue;
            };
            let mut focus_target: Option<OsString> = None;
            let result: Result<(), String> = match op {
                FileOperation::Delete(paths, move_to_trash) => {
                    let mut op_result = Ok(());
                    for p in paths {
                        let res = if move_to_trash {
                            trash::delete(&p).map_err(|e| e.to_string())
                        } else if p.is_dir() {
                            std::fs::remove_dir_all(&p).map_err(|e| e.to_string())
                        } else {
                            std::fs::remove_file(&p).map_err(|e| e.to_string())
                        };

                        if let Err(e) = res {
                            op_result = Err(format!("{}: {}", p.display(), e));
                            break;
                        }
                    }
                    op_result
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
                        std::fs::rename(old, &target).map_err(|e| e.to_string())
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
                    res.map_err(|e| e.to_string())
                }
                FileOperation::Copy {
                    src,
                    dest,
                    cut,
                    focus,
                } => {
                    focus_target = focus;
                    let mut op_result = Ok(());

                    for s in src {
                        if let Some(name) = s.file_name() {
                            let target = get_unused_path(&dest.join(name));

                            if let Some(ref ft) = focus_target
                                && ft == name
                            {
                                focus_target = target.file_name().map(|n| n.to_os_string());
                            }

                            if cut {
                                if std::fs::rename(&s, &target).is_err() {
                                    let copy_res = if s.is_dir() {
                                        copy_recursive(&s, &target)
                                    } else {
                                        std::fs::copy(&s, &target).map(|_| ())
                                    };

                                    match copy_res {
                                        Ok(_) => {
                                            let remove_res = if s.is_dir() {
                                                std::fs::remove_dir_all(&s)
                                            } else {
                                                std::fs::remove_file(&s)
                                            };

                                            if let Err(err) = remove_res {
                                                op_result = Err(format!(
                                                    "Copied to destination, but could not remove source: {}",
                                                    err
                                                ));
                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            op_result = Err(format!("{}: {}", s.display(), e));
                                            break;
                                        }
                                    }
                                }
                            } else {
                                let res = if s.is_dir() {
                                    copy_recursive(&s, &target)
                                } else {
                                    std::fs::copy(&s, &target).map(|_| ())
                                };

                                if let Err(e) = res {
                                    op_result = Err(format!("{}: {}", s.display(), e));
                                    break;
                                }
                            }
                        }
                    }
                    op_result
                }
            };

            match result {
                Ok(_) => {
                    let _ = res_tx.send(WorkerResponse::OperationComplete {
                        need_reload: true,
                        focus: focus_target,
                    });
                }
                Err(e) => {
                    let _ = res_tx.send(WorkerResponse::Error(format!("Op Error: {}", e), None));
                }
            }
        }
    });
}

/// Worker threads integration tests.
#[cfg(test)]
mod tests {
    use super::*;

    use rand::{RngExt, rng};
    use std::collections::HashSet;
    use std::fs::{self, File};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::thread;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    const TEST_TIMEOUT: Duration = Duration::from_secs(15);

    fn fd_available() -> bool {
        which::which("fd").is_ok()
    }

    fn bat_available() -> bool {
        which::which("bat").is_ok()
    }

    #[test]
    fn test_worker_pool_full_integration() -> Result<(), Box<dyn std::error::Error>> {
        let workers = Workers::spawn();
        let temp = tempdir()?;
        let test_file = temp.path().join("pool_test.txt");
        fs::write(&test_file, "Hello Pool")?;

        let find_cancel = Arc::new(AtomicBool::new(false));
        workers.find_tx().send(WorkerTask::FindRecursive {
            base_dir: temp.path().to_path_buf(),
            query: "pool".to_string(),
            max_results: 1,
            cancel: find_cancel,
            show_hidden: false,
            request_id: 10,
        })?;

        workers.preview_file_tx().send(WorkerTask::LoadPreview {
            path: test_file.clone(),
            max_lines: 1,
            pane_width: 10,
            preview_method: PreviewMethod::Internal,
            args: vec![],
            request_id: 20,
        })?;

        workers.nav_io_tx().send(WorkerTask::LoadDirectory {
            path: temp.path().to_path_buf(),
            focus: None,
            dirs_first: true,
            show_hidden: false,
            show_symlink: false,
            show_system: false,
            case_insensitive: true,
            always_show: Arc::new(HashSet::new()),
            request_id: 30,
        })?;

        workers.fileop_tx().send(WorkerTask::FileOp {
            op: FileOperation::Create {
                path: temp.path().join("new_file.txt"),
                is_dir: false,
            },
        })?;

        let mut responses_collected = 0;
        let mut results_found = false;
        let mut preview_found = false;
        let mut dir_found = false;
        let mut op_found = false;

        let timeout = Instant::now() + TEST_TIMEOUT;

        while responses_collected < 4 && Instant::now() < timeout {
            if let Ok(resp) = workers
                .response_rx()
                .recv_timeout(Duration::from_millis(500))
            {
                match resp {
                    WorkerResponse::FindResults { request_id, .. } => {
                        assert_eq!(request_id, 10);
                        results_found = true;
                    }
                    WorkerResponse::PreviewLoaded { request_id, .. } => {
                        assert_eq!(request_id, 20);
                        preview_found = true;
                    }
                    WorkerResponse::DirectoryLoaded { request_id, .. } => {
                        assert_eq!(request_id, 30);
                        dir_found = true;
                    }
                    WorkerResponse::OperationComplete { .. } => {
                        op_found = true;
                    }
                    _ => {}
                }
                responses_collected += 1;
            }
        }

        assert!(results_found, "Find worker failed");
        assert!(preview_found, "Preview worker failed");
        assert!(dir_found, "Nav IO worker failed");
        assert!(op_found, "FileOp worker failed");
        assert_eq!(responses_collected, 4);

        Ok(())
    }

    #[test]
    fn worker_load_current_dir() -> Result<(), Box<dyn std::error::Error>> {
        let workers = Workers::spawn();
        let temp = tempdir()?;
        let task_tx = workers.nav_io_tx();
        let res_rx = workers.response_rx();

        let temp_path = temp.path().join("test_dir");
        fs::create_dir(&temp_path)?;
        fs::File::create(temp_path.join("crab.txt"))?;

        task_tx.send(WorkerTask::LoadDirectory {
            path: temp_path,
            focus: None,
            dirs_first: true,
            show_hidden: false,
            show_symlink: false,
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
            WorkerResponse::Error(e, None) => panic!("Worker error: {}", e),
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
        let task_tx = workers.nav_io_tx();
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
                            show_symlink: rng.random_bool(0.5),
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
        let timeout = Instant::now() + TEST_TIMEOUT;

        for _ in 0..total_requests {
            let remaining = timeout.saturating_duration_since(Instant::now());
            match res_rx.recv_timeout(remaining.min(Duration::from_millis(500))) {
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
                Ok(WorkerResponse::Error(e, None)) => panic!("Worker error: {}", e),
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

    #[test]
    fn worker_find_pool() -> Result<(), Box<dyn std::error::Error>> {
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
            show_hidden: false,
            request_id: req_id,
        })?;

        let mut got = false;
        let deadline = Instant::now() + TEST_TIMEOUT;
        let expected_files: HashSet<_> = (0..5).map(|i| format!("crab_{i}.txt")).collect();

        while Instant::now() < deadline {
            match res_rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
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
    fn find_worker_finds_file() -> Result<(), Box<dyn std::error::Error>> {
        if !fd_available() {
            return Ok(());
        }

        let temp = tempfile::tempdir()?;
        std::fs::File::create(temp.path().join("crab.txt"))?;

        let workers = Workers::spawn();
        workers.find_tx().send(WorkerTask::FindRecursive {
            base_dir: temp.path().to_path_buf(),
            query: "crab".to_string(),
            max_results: 5,
            cancel: Arc::new(AtomicBool::new(false)),
            show_hidden: false,
            request_id: 2,
        })?;

        let resp = workers.response_rx().recv_timeout(TEST_TIMEOUT)?;

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
    fn preview_worker_internal() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let preview_file = temp.path().join("preview.txt");
        std::fs::write(&preview_file, "A\nB\nC\nD\n")?;
        let workers = Workers::spawn();
        workers.preview_file_tx().send(WorkerTask::LoadPreview {
            path: preview_file.clone(),
            max_lines: 2,
            pane_width: 40,
            preview_method: PreviewMethod::Internal,
            args: vec![],
            request_id: 3,
        })?;

        match workers.response_rx().recv_timeout(TEST_TIMEOUT)? {
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
    fn fileop_worker_create_and_delete_file() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let file_path = temp.path().join("touch.txt");
        let workers = Workers::spawn();

        workers.fileop_tx().send(WorkerTask::FileOp {
            op: FileOperation::Create {
                path: file_path.clone(),
                is_dir: false,
            },
        })?;

        let r = workers.response_rx().recv_timeout(TEST_TIMEOUT)?;
        match r {
            WorkerResponse::OperationComplete { .. } => {
                if !file_path.exists() {
                    return Err("Expected file to exist after creation".into());
                }
            }
            other => return Err(format!("Unexpected response: {:?}", other).into()),
        }

        workers.fileop_tx().send(WorkerTask::FileOp {
            op: FileOperation::Delete(vec![file_path.clone()], false),
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

    #[test]
    fn preview_fallback_on_failure() -> Result<(), Box<dyn std::error::Error>> {
        if !bat_available() {
            return Ok(());
        }

        let temp = tempdir()?;
        let file_path = temp.path().join("fallback.txt");
        fs::write(&file_path, "Standard Text Content")?;

        let workers = Workers::spawn();

        workers.preview_file_tx().send(WorkerTask::LoadPreview {
            path: file_path,
            max_lines: 5,
            pane_width: 40,
            preview_method: PreviewMethod::Bat,
            args: vec![],
            request_id: 99,
        })?;

        let resp = workers.response_rx().recv_timeout(TEST_TIMEOUT)?;
        if let WorkerResponse::PreviewLoaded { lines, .. } = resp {
            assert!(
                !lines.is_empty(),
                "Should have fallen back to internal reader"
            );
            assert_eq!(lines[0].trim(), "Standard Text Content");
        } else {
            panic!("Worker failed to provide fallback preview");
        }
        Ok(())
    }

    #[test]
    fn find_worker_sequential_execution() -> Result<(), Box<dyn std::error::Error>> {
        if !fd_available() {
            return Ok(());
        }

        let temp = tempdir()?;
        let temp_path = temp.path().to_path_buf();
        fs::File::create(temp_path.join("search_1.txt"))?;
        fs::File::create(temp_path.join("search_2.txt"))?;

        let workers = Workers::spawn();

        for i in 1..=2 {
            workers.find_tx().send(WorkerTask::FindRecursive {
                base_dir: temp_path.clone(),
                query: format!("search_{i}"),
                max_results: 1,
                cancel: Arc::new(AtomicBool::new(false)),
                show_hidden: false,
                request_id: i as u64,
            })?;
        }

        let resp1 = workers.response_rx().recv_timeout(TEST_TIMEOUT)?;
        if let WorkerResponse::FindResults { request_id, .. } = resp1 {
            assert_eq!(request_id, 1);
        }

        let resp2 = workers.response_rx().recv_timeout(TEST_TIMEOUT)?;
        if let WorkerResponse::FindResults { request_id, .. } = resp2 {
            assert_eq!(request_id, 2);
        }

        Ok(())
    }
}
