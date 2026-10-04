use std::sync::Arc;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use dashmap::DashMap;

use crate::core::{
    FileEntry, Formatter,
    cache::DirCache,
    workers::{WorkerResponse, WorkerTask},
};
use crate::utils::text::StrBuffer;

pub(super) fn start(
    task_rx: Receiver<WorkerTask>,
    res_tx: Sender<WorkerResponse>,
    cache: Arc<DirCache>,
) {
    thread::spawn(move || {
        while let Ok(task) = task_rx.recv() {
            let WorkerTask::SortDirectory {
                path,
                entries,
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

            let mut entries_vec = entries.to_vec();
            let meta_cache = DashMap::with_capacity(entries.len());

            let formatter = Formatter::new(list.clone(), sort_config, always_show);
            formatter.filter_entries(&mut entries_vec);
            let sort_column =
                formatter.sort_entries(&path, &mut entries_vec, &sort_date_format, &meta_cache);

            let entries_arc: Arc<[FileEntry]> = Arc::from(entries_vec);
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
    });
}
