//! Worker thread for the runa core operations.
//!
//! Handles directory reads, previews and file operatios on a background thread.
//! All results and errors are sent back via channels.
//! Small changes here can have big effects since this module is tightly integrated with every part
//! of runa.
//!
//! Requests [WorkerTask] are sent from the app/UI to the worker via channels,
//! and results or errors [WorkerResponse] are sent back the same way. All work
//! (including filesystem IO and preview logic) is executed on this background thread.
//!
//! # Caution:
//! This module is a central protocol boundary. Small changes (adding or editing variants, fields, or error handling)
//! may require corresponding changes throughout state, response-handling code and UI.

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crossbeam_channel::{Receiver, Sender, unbounded};
use unicode_width::UnicodeWidthChar;

use crate::core::find::{FindResult, find_recursive};
use crate::core::{FileEntry, file_manager::browse_dir};
use crate::utils::{Formatter, get_unused_path};

pub struct Workers {
    io_tx: Sender<WorkerTask>,
    find_tx: Sender<WorkerTask>,
    preview_tx: Sender<WorkerTask>,
    fileop_tx: Sender<WorkerTask>,
    response_rx: Receiver<WorkerResponse>,
}

impl Workers {
    pub fn spawn() -> Self {
        let (io_tx, io_rx) = unbounded::<WorkerTask>();
        let (preview_tx, preview_rx) = unbounded::<WorkerTask>();
        let (find_tx, find_rx) = unbounded::<WorkerTask>();
        let (fileop_tx, fileop_rx) = unbounded::<WorkerTask>();
        let (res_tx, response_rx) = unbounded::<WorkerResponse>();

        start_worker(io_rx, res_tx.clone());
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

    pub fn io_tx(&self) -> &Sender<WorkerTask> {
        &self.io_tx
    }

    pub fn preview_tx(&self) -> &Sender<WorkerTask> {
        &self.preview_tx
    }

    pub fn find_tx(&self) -> &Sender<WorkerTask> {
        &self.find_tx
    }

    pub fn fileop_tx(&self) -> &Sender<WorkerTask> {
        &self.fileop_tx
    }

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
        pane_width: usize,
        request_id: u64,
    },
    LoadPreview {
        path: PathBuf,
        max_lines: usize,
        pane_width: usize,
        request_id: u64,
    },
    FileOp {
        op: FileOperation,
        request_id: u64,
    },
    FindRecursive {
        root: PathBuf,
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
        root: PathBuf,
        results: Vec<FindResult>,
        request_id: u64,
    },
    Error(String),
}

/// Starts the worker thread, wich listens to [WorkerTask] and sends back to [WorkerResponse]
pub fn start_worker(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            match task {
                WorkerTask::LoadDirectory {
                    path,
                    focus,
                    dirs_first,
                    show_hidden,
                    show_system,
                    case_insensitive,
                    always_show,
                    pane_width,
                    request_id,
                } => match browse_dir(&path) {
                    Ok(mut entries) => {
                        let formatter = Formatter::new(
                            dirs_first,
                            show_hidden,
                            show_system,
                            case_insensitive,
                            always_show,
                            pane_width,
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
                },
                WorkerTask::LoadPreview { .. } => {
                    // Preview tasks are handled in a separate thread
                }
                WorkerTask::FileOp { .. } => {
                    // File operations are handled in a separate thread
                }
                // Find operations are handled in a separate thread
                WorkerTask::FindRecursive { .. } => {}
            }
        }
    });
}

fn start_preview_worker(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::LoadPreview {
                mut path,
                mut max_lines,
                mut pane_width,
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
                    request_id: id,
                } = next
                {
                    path = p;
                    max_lines = m;
                    pane_width = w;
                    request_id = id;
                }
            }

            let lines = safe_read_preview(&path, max_lines, pane_width);
            let _ = res_tx.send(WorkerResponse::PreviewLoaded { lines, request_id });
        }
    });
}

pub fn start_find_worker(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::FindRecursive {
                mut root,
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
                    root: r,
                    query: q,
                    max_results: max,
                    request_id: id,
                    cancel: c,
                } = next
                {
                    root = r;
                    query = q;
                    max_results = max;
                    request_id = id;
                    cancel = c;
                }
            }

            let mut results = Vec::new();
            let _ = find_recursive(&root, &query, &mut results, Arc::clone(&cancel));
            if results.len() > max_results {
                results.truncate(max_results);
            }

            if cancel.load(Ordering::Relaxed) {
                continue;
            }

            let _ = res_tx.send(WorkerResponse::FindResults {
                root,
                results,
                request_id,
            });
        }
    });
}

pub fn start_fileop_worker(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::FileOp { op, request_id } = task else {
                continue;
            };
            let mut focus_target: Option<OsString> = None;
            let result: Result<String, String> = match op {
                FileOperation::Delete(paths) => {
                    for p in paths {
                        let _ = if p.is_dir() {
                            std::fs::remove_dir_all(p)
                        } else {
                            std::fs::remove_file(p)
                        };
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
                        focus: focus_target, // CRITICA:
                    });
                }
                Err(e) => {
                    let _ = res_tx.send(WorkerResponse::Error(format!("Op Error: {}", e)));
                }
            }
        }
    });
}

// Calculating the pane widht and clean the output to the widht of the pane
fn sanitize_to_exact_width(line: &str, pane_width: usize) -> String {
    let mut out = String::with_capacity(pane_width);
    let mut current_w = 0;

    for char in line.chars() {
        if char == '\t' {
            let space_count = 4 - (current_w % 4);
            if current_w + space_count > pane_width {
                break;
            }
            out.push_str(&" ".repeat(space_count));
            current_w += space_count;
            continue;
        }

        if char.is_control() {
            continue;
        }

        let w = char.width().unwrap_or(0);
        if current_w + w > pane_width {
            break;
        }

        out.push(char);
        current_w += w;
    }

    // If the string is shorter than the pane, fill it with spaces.
    if current_w < pane_width {
        out.push_str(&" ".repeat(pane_width - current_w));
    }

    out
}

/// Loads a fixed-width preview of a directory entries
fn preview_directory(path: &Path, max_lines: usize, pane_width: usize) -> Vec<String> {
    match browse_dir(path) {
        Ok(entries) => {
            let mut lines = Vec::with_capacity(max_lines + 1);

            // Process existing entries
            for e in entries.iter().take(max_lines) {
                let suffix = if e.is_dir() { "/" } else { "" };
                let display_name = format!("{}{}", e.name().to_string_lossy(), suffix);

                // Sanitize and pad to exact width
                lines.push(sanitize_to_exact_width(&display_name, pane_width));
            }

            // Handle Empty State
            if lines.is_empty() {
                lines.push(sanitize_to_exact_width("[empty directory]", pane_width));
            }
            // Handle Overflow Indicator
            else if entries.len() > max_lines {
                lines.pop();
                lines.push(sanitize_to_exact_width("...", pane_width));
            }

            // If the folder has fewer items than the height of the pane,
            // it fills the remaining lines with empty padded strings.
            // This physically erases old content from the bottom of the pane.
            while lines.len() < max_lines {
                lines.push(" ".repeat(pane_width));
            }

            lines
        }
        Err(e) => {
            let mut err_lines = vec![sanitize_to_exact_width(
                &format!("[Error: {}]", e),
                pane_width,
            )];
            // Fill error screen with blanks too
            while err_lines.len() < max_lines {
                err_lines.push(" ".repeat(pane_width));
            }
            err_lines
        }
    }
}

/// Loads a preview for any path (directory or file), returning an error or a padded lines for
/// display.
/// large binaries/unreadable and unsupported files are replaced with a notice.
fn safe_read_preview(path: &Path, max_lines: usize, pane_width: usize) -> Vec<String> {
    // Minimum number of lines shown in any preview
    const MIN_PREVIEW_LINES: usize = 3;
    // Maximum file size allowed for preview (10mb)
    const MAX_PREVIEW_SIZE: u64 = 10 * 1024 * 1024;
    // Number of bytes to peek from file start for header checks (eg. PNG, ZIP, etc..)
    const HEADER_PEEK_BYTES: usize = 8;
    // Bytes to peek for null bytes in binary detections
    const BINARY_PEEK_BYTES: usize = 1024;

    let max_lines = std::cmp::max(max_lines, MIN_PREVIEW_LINES);

    // Metadata check
    let Ok(meta) = std::fs::metadata(path) else {
        return vec![sanitize_to_exact_width(
            "[Error: Access Denied]",
            pane_width,
        )];
    };

    if path.is_dir() {
        return preview_directory(path, max_lines, pane_width);
    }

    // Size Check
    if meta.len() > MAX_PREVIEW_SIZE {
        return vec![sanitize_to_exact_width(
            "[File too large for preview]",
            pane_width,
        )];
    }

    if !meta.is_file() {
        return vec![sanitize_to_exact_width("[Not a regular file]", pane_width)];
    }

    // File Read and binary Check
    match File::open(path) {
        Ok(mut file) => {
            // Peek for the first 8 bytes to handle edge cases
            let mut header = [0u8; HEADER_PEEK_BYTES];
            let read_bytes = file.read(&mut header).unwrap_or(0);
            if read_bytes >= 5 && &header[..5] == b"%PDF-" {
                return vec![sanitize_to_exact_width(
                    "[Binary file - preview hidden]",
                    pane_width,
                )];
            }

            // Peek for null bytes to detect binary files
            let mut buffer = [0u8; BINARY_PEEK_BYTES];
            let n = file.read(&mut buffer).unwrap_or(0);
            if buffer[..n].contains(&0) {
                return vec![sanitize_to_exact_width(
                    "[Binary file - preview hidden]",
                    pane_width,
                )];
            }

            let _ = file.rewind();

            let reader = BufReader::new(file);
            let mut preview_lines = Vec::with_capacity(max_lines);

            for line_result in reader.lines().take(max_lines) {
                match line_result {
                    Ok(line) => {
                        preview_lines.push(sanitize_to_exact_width(&line, pane_width));
                    }
                    Err(_) => break,
                }
            }

            if preview_lines.is_empty() {
                preview_lines.push(sanitize_to_exact_width("[Empty file]", pane_width));
            }

            preview_lines
        }
        Err(e) => {
            let msg = match e.kind() {
                ErrorKind::PermissionDenied => "[Error: Permission Denied]",
                ErrorKind::NotFound => "[Error: File Not Found]",
                _ => {
                    return vec![sanitize_to_exact_width(
                        &format!("[Error reading file: {}]", e),
                        pane_width,
                    )];
                }
            };
            vec![sanitize_to_exact_width(msg, pane_width)]
        }
    }
}
