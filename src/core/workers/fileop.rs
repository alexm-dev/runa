//! File Operation worker module
//!
//! Implementation of the file operation worker,
//! which handles file operations such as delete, rename, create, and copy.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use crossbeam_channel::{Receiver, Sender};

use crate::core::fs;
use crate::core::workers::{FileOperation, WorkerResponse, WorkerTask};

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

pub(super) fn start(
    task_rx: Receiver<WorkerTask>,
    res_tx: Sender<WorkerResponse>,
    active_fileop: Arc<AtomicUsize>,
) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let _guard = ActiveOpGuard::new(Arc::clone(&active_fileop));

            let WorkerTask::FileOp { op } = task else {
                continue;
            };

            let modified_dirs = collect_modified_dirs(&op);
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
                FileOperation::Rename {
                    old,
                    new,
                    overwrite,
                } => {
                    let target = new;
                    let is_case_rename = old.to_string_lossy().to_lowercase()
                        == target.to_string_lossy().to_lowercase();

                    if target.exists() && !is_case_rename && !overwrite {
                        Err(format!(
                            "Rename failed: '{}' already exists",
                            target.file_name().unwrap_or_default().to_string_lossy()
                        ))
                    } else {
                        if target.exists() && !is_case_rename && overwrite {
                            if old.is_dir() && target.is_dir() {
                                match fs::merge_dir(&old, &target, true) {
                                    Ok(()) => {
                                        focus_target = target.file_name().map(|n| n.to_os_string());
                                        Ok(())
                                    }
                                    Err(e) => Err(format!(
                                        "Could not merge directories '{}' -> '{}': {}",
                                        old.display(),
                                        target.display(),
                                        e
                                    )),
                                }
                            } else {
                                let remove_res = if target.is_dir() {
                                    std::fs::remove_dir_all(&target)
                                } else {
                                    std::fs::remove_file(&target)
                                };

                                if let Err(e) = remove_res {
                                    Err(format!(
                                        "Could not remove existing target before rename: {}: {}",
                                        target.display(),
                                        e
                                    ))
                                } else {
                                    focus_target = target.file_name().map(|n| n.to_os_string());
                                    fs::rename_with_fallback(&old, &target, old.is_dir())
                                        .map_err(|e| e.to_string())
                                }
                            }
                        } else {
                            focus_target = target.file_name().map(|n| n.to_os_string());
                            fs::rename_with_fallback(&old, &target, old.is_dir())
                                .map_err(|e| e.to_string())
                        }
                    }
                }
                FileOperation::Create {
                    path,
                    is_dir,
                    overwrite,
                } => {
                    let target = if overwrite {
                        path
                    } else {
                        fs::get_unused_path(&path)
                    };

                    if target.exists() {
                        if target.is_dir() {
                            if is_dir {
                                focus_target = target.file_name().map(|n| n.to_os_string());
                                Ok(())
                            } else {
                                Err(format!(
                                    "Create failed: '{}' is an existing directory",
                                    target.display()
                                ))
                            }
                        } else {
                            let remove_res = std::fs::remove_file(&target);
                            if let Err(e) = remove_res {
                                Err(format!(
                                    "Could not remove existing file before create: {}: {}",
                                    target.display(),
                                    e
                                ))
                            } else {
                                focus_target = target.file_name().map(|n| n.to_os_string());
                                if is_dir {
                                    std::fs::create_dir_all(&target).map_err(|e| e.to_string())
                                } else {
                                    std::fs::OpenOptions::new()
                                        .write(true)
                                        .create_new(true)
                                        .open(&target)
                                        .map(|_| ())
                                        .map_err(|e| e.to_string())
                                }
                            }
                        }
                    } else {
                        focus_target = target.file_name().map(|n| n.to_os_string());
                        if is_dir {
                            std::fs::create_dir_all(&target).map_err(|e| e.to_string())
                        } else {
                            std::fs::OpenOptions::new()
                                .write(true)
                                .create_new(true)
                                .open(&target)
                                .map(|_| ())
                                .map_err(|e| e.to_string())
                        }
                    }
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
                            let target = fs::get_unused_path(&dest.join(name));

                            if let Some(ref ft) = focus_target
                                && ft == name
                            {
                                focus_target = target.file_name().map(|n| n.to_os_string());
                            }

                            if cut {
                                if std::fs::rename(&s, &target).is_err() {
                                    let copy_res = if s.is_dir() {
                                        fs::copy_recursive(&s, &target)
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
                                    fs::copy_recursive(&s, &target)
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
                        modified_dirs,
                    });
                }
                Err(e) => {
                    let _ = res_tx.send(WorkerResponse::Error(format!("Op Error: {}", e), None));
                }
            }
        }
    });
}

fn collect_modified_dirs(op: &FileOperation) -> Vec<PathBuf> {
    let add_parent = |dirs: &mut HashSet<PathBuf>, path: &Path| {
        if let Some(parent) = path.parent() {
            dirs.insert(parent.to_path_buf());
        }
    };

    let mut dirs = HashSet::new();

    match op {
        FileOperation::Delete(paths, _) => {
            for p in paths {
                add_parent(&mut dirs, p);
            }
        }
        FileOperation::Rename { old, new, .. } => {
            add_parent(&mut dirs, old);
            add_parent(&mut dirs, new);
        }
        FileOperation::Copy { src, dest, cut, .. } => {
            dirs.insert(dest.clone());
            if *cut {
                for s in src {
                    add_parent(&mut dirs, s);
                }
            }
        }
        FileOperation::Create { path, .. } => {
            add_parent(&mut dirs, path);
        }
    }

    dirs.into_iter().collect()
}
