//! Preview worker thread module.
//!
//! This module holds the implementation of the preview worker thread.

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};

use crate::core::{
    fs,
    preview::{self, ListingRow},
    proc,
    workers::{PreviewLines, PreviewMode, PreviewTask, WorkerResponse},
};
use crate::utils::{os, timings::Timings};

pub(super) fn start(task_rx: Receiver<PreviewTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(PreviewTask {
            path,
            max_lines,
            pane_width,
            scroll,
            preview_mode,
            request_id,
            tab_id,
        }) = task_rx.recv()
        {
            let read_lines = max_lines + 1;

            let mut lines = if fs::is_temp_file(&path) {
                PreviewLines::Text(vec![preview::sanitize_to_exact_width(
                    "[Temporary file - preview skipped]",
                    pane_width,
                )])
            } else if let PreviewMode::Archive { program, args, dir } = &preview_mode
                && let Some(rows) =
                    archive_rows(&path, program, args, dir.as_deref(), scroll, read_lines)
            {
                PreviewLines::Listing(rows)
            } else {
                PreviewLines::Text(match preview_mode {
                    PreviewMode::Internal | PreviewMode::Archive { .. } => {
                        preview::safe_read_preview(&path, read_lines, pane_width, scroll)
                    }
                    PreviewMode::Bat { args } => {
                        if !os::is_regular_file(&path) || fs::is_preview_deny(&path) {
                            preview::safe_read_preview(&path, read_lines, pane_width, scroll)
                        } else {
                            match proc::preview_bat(&path, read_lines, args.as_slice(), scroll) {
                                // Bat preview succeeded
                                // If bat fails, fallback to internal preview
                                // If bat is not installed or returns error, we fallback to internal preview
                                Ok(lines) => lines,
                                Err(_) => preview::safe_read_preview(
                                    &path, read_lines, pane_width, scroll,
                                ),
                            }
                        }
                    }
                })
            };

            let is_eof = match &mut lines {
                PreviewLines::Text(lines) => split_eof(lines, max_lines),
                PreviewLines::Listing(rows) => split_eof(rows, max_lines),
            };
            let _ = res_tx.send(WorkerResponse::PreviewLoaded {
                path,
                lines,
                scroll,
                is_eof,
                request_id,
                tab_id,
            });
        }
    });
}

fn archive_rows(
    path: &Path,
    program: &OsStr,
    args: &[OsString],
    dir: Option<&Path>,
    scroll: usize,
    read_lines: usize,
) -> Option<Vec<ListingRow>> {
    if !os::is_regular_file(path) {
        return None;
    }

    let timeout = Duration::from_millis(Timings::PREVIEW_COMMAND_TIMEOUT_MS);
    let paths = proc::preview_command(program, args, dir, scroll + read_lines, timeout).ok()?;
    let mut rows = preview::tree_rows(&paths);
    rows.drain(..scroll.min(rows.len()));
    Some(rows)
}

fn split_eof<T>(lines: &mut Vec<T>, max_lines: usize) -> bool {
    let is_eof = lines.len() <= max_lines;
    lines.truncate(max_lines);
    is_eof
}

#[cfg(test)]
mod tests {
    use crate::core::workers::test_support::{TEST_TAB_ID, TEST_TIMEOUT, bat_available};
    use crate::core::workers::{PreviewLines, PreviewMode, PreviewTask, WorkerResponse, Workers};

    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn preview_worker_internal() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let preview_file = temp.path().join("preview.txt");
        std::fs::write(&preview_file, "A\nB\nC\nD\n")?;
        let workers = Workers::spawn();
        workers.preview_file_tx().send(PreviewTask {
            path: preview_file.clone(),
            max_lines: 2,
            pane_width: 40,
            scroll: 0,
            preview_mode: PreviewMode::Internal,
            request_id: 3,
            tab_id: TEST_TAB_ID,
        })?;

        match workers.response_rx().recv_timeout(TEST_TIMEOUT)? {
            WorkerResponse::PreviewLoaded {
                lines: PreviewLines::Text(lines),
                ..
            } => {
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
    fn preview_fallback_on_failure() -> Result<(), Box<dyn std::error::Error>> {
        if !bat_available() {
            return Ok(());
        }

        let temp = tempdir()?;
        let file_path = temp.path().join("fallback.txt");
        fs::write(&file_path, "Standard Text Content")?;

        let workers = Workers::spawn();

        workers.preview_file_tx().send(PreviewTask {
            path: file_path,
            max_lines: 5,
            pane_width: 40,
            scroll: 0,
            preview_mode: PreviewMode::Bat { args: vec![] },
            request_id: 99,
            tab_id: TEST_TAB_ID,
        })?;

        let resp = workers.response_rx().recv_timeout(TEST_TIMEOUT)?;
        if let WorkerResponse::PreviewLoaded {
            lines: PreviewLines::Text(lines),
            ..
        } = resp
        {
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
}
