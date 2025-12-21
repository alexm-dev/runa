use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{Receiver, Sender};

use crate::file_manager::{FileEntry, browse_dir};
use crate::formatter::Formatter;

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
        request_id: u64,
    },
}

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
    Error(String),
}

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
                    request_id,
                } => match browse_dir(&path) {
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
                },
                WorkerTask::LoadPreview {
                    path,
                    max_lines,
                    request_id,
                } => {
                    let lines = safe_read_preview(&path, max_lines);
                    let _ = res_tx.send(WorkerResponse::PreviewLoaded { lines, request_id });
                }
            }
        }
    });
}

fn safe_read_preview(path: &Path, max_lines: usize) -> Vec<String> {
    let max_lines = std::cmp::max(max_lines, 3);

    if path.is_dir() {
        return match browse_dir(path) {
            Ok(entries) => {
                let mut lines: Vec<String> = entries
                    .iter()
                    .take(max_lines)
                    .map(|e| {
                        if e.is_dir() {
                            format!("{}/", e.name().to_string_lossy())
                        } else {
                            e.name().to_string_lossy().to_string()
                        }
                    })
                    .collect();

                if lines.is_empty() {
                    lines.push("[empty directory]".into());
                } else if entries.len() > max_lines {
                    lines.push("...".into());
                }
                lines
            }
            Err(e) => vec![format!("[error reading dir: {}]", e)],
        };
    }

    let Ok(meta) = std::fs::metadata(path) else {
        return vec!["[Error: Access Denied]".into()];
    };

    if meta.len() > 10 * 1024 * 1024 {
        return vec!["[File too large for preview]".into()];
    }

    if !meta.is_file() {
        return vec!["[Not a regular file]".into()];
    }

    if let Some(fname) = path.file_name().and_then(|os| os.to_str()) {
        let lower = fname.to_lowercase();
        let well_known = ["readme", "license", "copying", "makefile"];
        if !fname.contains('.') && fname.len() <= 3 && !well_known.contains(&lower.as_str()) {
            return vec!["[Short, extensionless file - preview hidden]".into()];
        }
    }

    match File::open(path) {
        Ok(mut file) => {
            let meta = file.metadata().ok();
            if let Some(m) = meta
                && m.len() > 10 * 1024 * 1024
            {
                return vec!["[File is too large]".into()];
            }
            let mut header = [0u8; 1024];
            let n = file.read(&mut header).unwrap_or(0);

            if header[..n].contains(&0) {
                return vec!["[Binary file - preview hidden]".into()];
            }

            use std::io::Seek;
            let _ = file.rewind();

            let reader = BufReader::new(file);
            let mut lines: Vec<String> = reader
                .lines()
                .take(max_lines)
                .filter_map(Result::ok)
                .collect();

            if lines.is_empty() {
                lines.push("[Empty file]".into());
            }

            lines
        }
        Err(e) => vec![format!("[Error reading file: {}]", e)],
    }
}
