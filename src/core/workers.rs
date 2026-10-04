//! Worker thread for the runa core operations.
//!
//! Handles directory reads, previews and file operatios on a background thread.
//! All results and errors are sent back via channels.
//!
//! Small changes here can have big effects since this module is tightly integrated with every part
//! of runa.
//!
//! Requests [WorkerTask] come in from the RunaRoot or UI via channels, and results or errors
//! [WorkerResponse] go back the same way. All filesystem I/O and previews happen on these threads
//!
//! # Caution:
//! This module is a central protocol boundary. Small changes (adding or editing variants, fields, or error handling)
//! may require corresponding changes throughout state, response-handling code and UI.

mod fileop;
mod find;
mod io;
mod metadata;
mod preview;
mod sort;
mod watch;

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};

use crate::core::{
    FileEntry, FindResult,
    cache::{DirCache, DirListOptions},
    metadata::{FileMetadataCache, MetadataNeeds},
    sort::SortConfig,
};
use crate::utils::text::StrBuffer;

/// Manages worker threads channels for different task types.
pub(crate) struct Workers {
    nav_io_tx: Sender<WorkerTask>,
    parent_io_tx: Sender<WorkerTask>,
    preview_dir_tx: Sender<WorkerTask>,
    sort_tx: Sender<WorkerTask>,
    preview_file_tx: Sender<WorkerTask>,
    metadata_tx: Sender<WorkerTask>,
    find_tx: Sender<WorkerTask>,
    fileop_tx: Sender<WorkerTask>,
    watch_cmd_tx: Sender<WatchCommand>,
    response_rx: Receiver<WorkerResponse>,
    active_fileops: Arc<AtomicUsize>,
    cache: Arc<DirCache>,
}

/// Manages worker thread channels for different task types.
///
/// Each major operation (I/O (nav, preview, parent), preview, file metadata, find, file-ops) has its own dedicated worker thread.
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
        let cache = Arc::new(DirCache::new());

        let (nav_io_tx, nav_io_rx) = bounded::<WorkerTask>(1);
        let (parent_io_tx, parent_io_rx) = bounded::<WorkerTask>(1);
        let (preview_dir_tx, preview_dir_rx) = bounded::<WorkerTask>(1);
        let (sort_tx, sort_rx) = bounded::<WorkerTask>(1);

        let (preview_file_tx, preview_file_rx) = bounded::<WorkerTask>(1);
        let (metadata_tx, metadata_rx) = bounded::<WorkerTask>(1);
        let (find_tx, find_rx) = bounded::<WorkerTask>(1);
        let (fileop_tx, fileop_rx) = unbounded::<WorkerTask>();
        let (watch_cmd_tx, watch_cmd_rx) = unbounded::<WatchCommand>();
        let (res_tx, response_rx) = unbounded::<WorkerResponse>();

        let active_fileops = Arc::new(AtomicUsize::new(0));

        io::start(nav_io_rx, res_tx.clone(), Arc::clone(&cache));
        io::start(parent_io_rx, res_tx.clone(), Arc::clone(&cache));
        io::start(preview_dir_rx, res_tx.clone(), Arc::clone(&cache));

        sort::start(sort_rx, res_tx.clone(), Arc::clone(&cache));
        preview::start(preview_file_rx, res_tx.clone());
        metadata::start(metadata_rx, res_tx.clone());
        find::start(find_rx, res_tx.clone());
        fileop::start(fileop_rx, res_tx.clone(), Arc::clone(&active_fileops));
        watch::start(watch_cmd_rx, res_tx.clone());

        Self {
            nav_io_tx,
            parent_io_tx,
            preview_dir_tx,
            sort_tx,
            preview_file_tx,
            metadata_tx,
            find_tx,
            fileop_tx,
            watch_cmd_tx,
            response_rx,
            active_fileops,
            cache,
        }
    }

    crate::getters! {
        nav_io_tx: &Sender<WorkerTask>,
        parent_io_tx: &Sender<WorkerTask>,
        preview_dir_tx: &Sender<WorkerTask>,
        sort_tx: &Sender<WorkerTask>,
        preview_file_tx: &Sender<WorkerTask>,
        metadata_tx: &Sender<WorkerTask>,
        find_tx: &Sender<WorkerTask>,
        fileop_tx: &Sender<WorkerTask>,
        response_rx: &Receiver<WorkerResponse>,
        active_fileops: &Arc<AtomicUsize>,
    }

    pub(crate) fn cache(&self) -> Arc<DirCache> {
        Arc::clone(&self.cache)
    }

    pub(crate) fn retarget_watch(&self, dirs: Vec<PathBuf>) {
        let _ = self.watch_cmd_tx.send(WatchCommand::Retarget(dirs));
    }
}

pub(crate) enum PreviewMode {
    Internal,
    Bat { args: Vec<OsString> },
}

/// Tasks sent to the worker thread via channel.
///
/// Each variant describes a filesystem or a preview operation to perform.
pub(crate) enum WorkerTask {
    LoadDirectory {
        path: PathBuf,
        focus: Option<OsString>,
        list: DirListOptions,
        sort_config: SortConfig,
        sort_date_format: Arc<str>,
        always_show: Arc<HashSet<OsString>>,
        request_id: u64,
        tab_id: Option<usize>,
    },
    SortDirectory {
        path: PathBuf,
        entries: Arc<[FileEntry]>,
        focus: Option<OsString>,
        list: DirListOptions,
        sort_config: SortConfig,
        sort_date_format: Arc<str>,
        always_show: Arc<HashSet<OsString>>,
        request_id: u64,
        tab_id: Option<usize>,
    },
    LoadPreview {
        path: PathBuf,
        max_lines: usize,
        pane_width: usize,
        scroll: usize,
        preview_mode: PreviewMode,
        request_id: u64,
        tab_id: Option<usize>,
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
        tab_id: Option<usize>,
    },
    GetFileMetadata {
        path: PathBuf,
        date_format: String,
        needs: MetadataNeeds,
        request_id: u64,
        tab_id: Option<usize>,
    },
}

/// Supported file system operations the worker can perform.
pub(crate) enum FileOperation {
    Delete(Vec<PathBuf>, bool),
    Rename {
        old: PathBuf,
        new: PathBuf,
        overwrite: bool,
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
        overwrite: bool,
    },
}

/// Responses sent form the worker thread back to the main thread via the channel
///
/// Each variant delivers the result or error from a request taks.
#[derive(Debug, Clone)]
pub(crate) enum WorkerResponse {
    DirectoryLoaded {
        path: PathBuf,
        entries: Arc<[FileEntry]>,
        focus: Option<OsString>,
        sort_column: Option<Arc<StrBuffer>>,
        request_id: u64,
        tab_id: Option<usize>,
    },
    PreviewLoaded {
        path: PathBuf,
        lines: Vec<String>,
        is_eof: bool,
        request_id: u64,
        tab_id: Option<usize>,
    },
    OperationComplete {
        need_reload: bool,
        focus: Option<OsString>,
        modified_dirs: Vec<PathBuf>,
    },
    FindResults {
        base_dir: PathBuf,
        results: Vec<FindResult>,
        request_id: u64,
        tab_id: Option<usize>,
    },
    FileMetadataLoaded {
        metadata: Arc<FileMetadataCache>,
        path: PathBuf,
        request_id: u64,
        tab_id: Option<usize>,
    },
    ConfigChanged,
    DirsChanged {
        dirs: Vec<PathBuf>,
    },
    Error(String, Option<u64>),
}

impl WorkerResponse {
    pub(crate) fn tab_id(&self) -> Option<usize> {
        match self {
            WorkerResponse::DirectoryLoaded { tab_id, .. } => *tab_id,
            WorkerResponse::PreviewLoaded { tab_id, .. } => *tab_id,
            WorkerResponse::FindResults { tab_id, .. } => *tab_id,
            WorkerResponse::FileMetadataLoaded { tab_id, .. } => *tab_id,
            _ => None,
        }
    }
}

pub(crate) enum WatchCommand {
    Retarget(Vec<PathBuf>),
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

    const TEST_TAB_ID: Option<usize> = None;

    fn fd_available() -> bool {
        which::which("fd").is_ok()
    }

    fn bat_available() -> bool {
        which::which("bat").is_ok()
    }

    fn test_list_opts() -> DirListOptions {
        DirListOptions {
            dirs_first: true,
            show_hidden: false,
            show_symlink: false,
            show_system: false,
            case_insensitive: true,
        }
    }

    #[test]
    fn worker_pool_full_integration() -> Result<(), Box<dyn std::error::Error>> {
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
            tab_id: TEST_TAB_ID,
        })?;

        workers.preview_file_tx().send(WorkerTask::LoadPreview {
            path: test_file.clone(),
            max_lines: 1,
            pane_width: 10,
            scroll: 0,
            preview_mode: PreviewMode::Bat { args: vec![] },
            request_id: 20,
            tab_id: TEST_TAB_ID,
        })?;

        workers.nav_io_tx().send(WorkerTask::LoadDirectory {
            path: temp.path().to_path_buf(),
            focus: None,
            list: test_list_opts(),
            sort_date_format: Arc::<str>::from(""),
            sort_config: SortConfig::default(),
            always_show: Arc::new(HashSet::new()),
            request_id: 30,
            tab_id: TEST_TAB_ID,
        })?;

        workers.fileop_tx().send(WorkerTask::FileOp {
            op: FileOperation::Create {
                path: temp.path().join("new_file.txt"),
                is_dir: false,
                overwrite: false,
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
                    let list = DirListOptions {
                        dirs_first: rng.random_bool(0.5),
                        show_hidden: rng.random_bool(0.5),
                        show_symlink: rng.random_bool(0.5),
                        show_system: rng.random_bool(0.5),
                        case_insensitive: rng.random_bool(0.5),
                    };
                    task_tx
                        .send(WorkerTask::LoadDirectory {
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
            tab_id: TEST_TAB_ID,
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
            tab_id: TEST_TAB_ID,
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
            scroll: 0,
            preview_mode: PreviewMode::Internal,
            request_id: 3,
            tab_id: TEST_TAB_ID,
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
        let temp_dir = temp.path().to_path_buf();
        let workers = Workers::spawn();

        workers.fileop_tx().send(WorkerTask::FileOp {
            op: FileOperation::Create {
                path: file_path.clone(),
                is_dir: false,
                overwrite: false,
            },
        })?;

        let r = workers.response_rx().recv_timeout(TEST_TIMEOUT)?;
        match r {
            WorkerResponse::OperationComplete { modified_dirs, .. } => {
                if !modified_dirs.iter().any(|path| path == &temp_dir) {
                    return Err(
                        "Expected operation completion to include file parent as affected".into(),
                    );
                }
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
            WorkerResponse::OperationComplete { modified_dirs, .. } => {
                if !modified_dirs.iter().any(|path| path == &temp_dir) {
                    return Err(
                        "Expected operation completion to include file parent as affected".into(),
                    );
                }
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
            scroll: 0,
            preview_mode: PreviewMode::Bat { args: vec![] },
            request_id: 99,
            tab_id: TEST_TAB_ID,
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
                tab_id: TEST_TAB_ID,
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
