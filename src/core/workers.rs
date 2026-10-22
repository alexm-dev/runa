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
mod runner;
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
    preview::ListingRow,
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
    runner_tx: Sender<WorkerTask>,
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
        let (runner_tx, runner_rx) = unbounded::<WorkerTask>();
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
        runner::start(runner_rx, res_tx.clone());
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
            runner_tx,
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
        runner_tx: &Sender<WorkerTask>,
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
    Bat {
        args: Vec<OsString>,
    },
    Archive {
        program: OsString,
        args: Vec<OsString>,
        dir: Option<PathBuf>,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum PreviewLines {
    Text(Vec<String>),
    Listing(Vec<ListingRow>),
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
    RunCommand {
        program: PathBuf,
        args: Vec<OsString>,
        dir: Option<PathBuf>,
        check_status: bool,
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
        lines: PreviewLines,
        scroll: usize,
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

/// Shared helpers for the worker tests.
#[cfg(test)]
mod test_support {
    use std::time::Duration;

    use crate::core::cache::DirListOptions;

    pub(super) const TEST_TIMEOUT: Duration = Duration::from_secs(15);

    pub(super) const TEST_TAB_ID: Option<usize> = None;

    pub(super) fn fd_available() -> bool {
        which::which("fd").is_ok()
    }

    pub(super) fn bat_available() -> bool {
        which::which("bat").is_ok()
    }

    pub(super) fn test_list_opts() -> DirListOptions {
        DirListOptions {
            dirs_first: true,
            show_hidden: false,
            show_symlink: false,
            show_system: false,
            case_insensitive: true,
        }
    }
}
