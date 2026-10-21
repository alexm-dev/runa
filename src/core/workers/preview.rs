//! Preview worker thread module.
//!
//! This module holds the implementation of the preview worker thread.

use std::thread;

use crossbeam_channel::{Receiver, Sender};

use crate::core::{
    formatter, fs, proc,
    workers::{PreviewMode, WorkerResponse, WorkerTask},
};
use crate::utils::os;

pub(super) fn start(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::LoadPreview {
                path,
                max_lines,
                pane_width,
                scroll,
                preview_mode,
                request_id,
                tab_id,
            } = task
            else {
                continue;
            };

            let read_lines = max_lines + 1;

            let lines = if fs::is_temp_file(&path) {
                vec![formatter::sanitize_to_exact_width(
                    "[Temporary file - preview skipped]",
                    pane_width,
                )]
            } else {
                match preview_mode {
                    PreviewMode::Internal => {
                        formatter::safe_read_preview(&path, read_lines, pane_width, scroll)
                    }
                    PreviewMode::Bat { args } => {
                        if !os::is_regular_file(&path) || fs::is_preview_deny(&path) {
                            formatter::safe_read_preview(&path, read_lines, pane_width, scroll)
                        } else {
                            match proc::preview_bat(&path, read_lines, args.as_slice(), scroll) {
                                // Bat preview succeeded
                                // If bat fails, fallback to internal preview
                                // If bat is not installed or returns error, we fallback to internal preview
                                Ok(lines) => lines,
                                Err(_) => formatter::safe_read_preview(
                                    &path, read_lines, pane_width, scroll,
                                ),
                            }
                        }
                    }
                }
            };

            let (lines, is_eof) = split_eof(lines, max_lines);
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

fn split_eof(mut lines: Vec<String>, max_lines: usize) -> (Vec<String>, bool) {
    let is_eof = lines.len() <= max_lines;
    lines.truncate(max_lines);
    (lines, is_eof)
}
