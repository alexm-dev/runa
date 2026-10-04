//! Find feature worker thread.
//!
//! Module holds the implementation of the find feature worker thread.
//! It listens for find tasks, executes them, and sends back the results.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;

use crossbeam_channel::{Receiver, Sender};

use crate::core::{
    proc,
    workers::{WorkerResponse, WorkerTask},
};

pub(super) fn start(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::FindRecursive {
                base_dir,
                query,
                max_results,
                cancel,
                show_hidden,
                request_id,
                tab_id,
            } = task
            else {
                continue;
            };

            let mut results = Vec::new();
            let _ = proc::find(
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
                tab_id,
            });
        }
    });
}
