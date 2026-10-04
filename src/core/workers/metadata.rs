//! File metadata worker thread.
//!
//! Module contains the implementation for the file metadata worker thread,
//! which is responsible for loading file metadata in a separate thread.
//! Used by the FileInfo UI component to load file metadata asynchronously.

use std::sync::Arc;
use std::thread;

use chrono::Local;
use crossbeam_channel::{Receiver, Sender};

use crate::core::{
    metadata::{FileMetadata, FileMetadataCache},
    workers::{WorkerResponse, WorkerTask},
};

/// Starts the file metadata worker thread.
pub(super) fn start(task_rx: Receiver<WorkerTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        #[cfg(unix)]
        let mut ug_cache = crate::core::metadata::unix_meta::UserGroupCache::new();

        while let Ok(task) = task_rx.recv() {
            let now = Local::now();
            if let WorkerTask::GetFileMetadata {
                path,
                date_format,
                needs,
                request_id,
                tab_id,
            } = task
            {
                match FileMetadata::new(&path) {
                    Ok(meta) => {
                        let cache = FileMetadataCache::from(
                            &meta,
                            &date_format,
                            &needs,
                            now,
                            #[cfg(unix)]
                            &mut ug_cache,
                        );
                        let _ = res_tx.send(WorkerResponse::FileMetadataLoaded {
                            metadata: Arc::new(cache),
                            path,
                            request_id,
                            tab_id,
                        });
                    }
                    Err(e) => {
                        let vanished = e.kind() == std::io::ErrorKind::NotFound || !path.exists();
                        if !vanished {
                            let _ = res_tx.send(WorkerResponse::Error(
                                format!("Metadata Error: {}", e),
                                Some(request_id),
                            ));
                        }
                    }
                }
            }
        }
    });
}
