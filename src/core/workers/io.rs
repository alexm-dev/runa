//! IO worker thread module.
//!
//! Handles the nav, parent, preview I/O operations.

use std::sync::Arc;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use dashmap::DashMap;

use crate::core::{
    cache::DirCache,
    fm::{self, FileEntry},
    formatter::Formatter,
    workers::{WorkerResponse, WorkerTask},
};
use crate::utils::text::StrBuffer;

/// Starts the io worker thread, wich listens to [WorkerTask] and sends back to [WorkerResponse]
pub(super) fn start(
    task_rx: Receiver<WorkerTask>,
    res_tx: Sender<WorkerResponse>,
    cache: Arc<DirCache>,
) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::LoadDirectory {
                path,
                focus,
                list,
                sort_config,
                sort_date_format,
                always_show,
                request_id,
                tab_id,
            } = task
            else {
                continue;
            };
            match fm::browse_dir(&path) {
                Ok(mut entries) => {
                    let meta_cache = DashMap::with_capacity(entries.len());
                    let formatter = Formatter::new(list.clone(), sort_config, always_show);
                    formatter.filter_entries(&mut entries);
                    let sort_column =
                        formatter.sort_entries(&path, &mut entries, &sort_date_format, &meta_cache);

                    let entries_arc: Arc<[FileEntry]> = Arc::from(entries);
                    let sort_column_arc: Option<Arc<StrBuffer>> =
                        sort_column.map(|v| Arc::new(StrBuffer::from_iter(v)));

                    cache.insert_if_newer(
                        &path,
                        sort_config,
                        &list,
                        Arc::clone(&entries_arc),
                        sort_column_arc.clone(),
                        request_id,
                    );

                    let _ = res_tx.send(WorkerResponse::DirectoryLoaded {
                        path,
                        entries: entries_arc,
                        focus,
                        sort_column: sort_column_arc,
                        request_id,
                        tab_id,
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
