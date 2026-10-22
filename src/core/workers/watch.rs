//! Config watcher thread.
//!
//! This thread watches for changes to the runa config file and any directories that are being listed in the UI.
//! When a change is detected, it sends a message to the main thread to handle the change.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, unbounded};
use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::ModifyKind,
};

use crate::core::workers::{WatchCommand, WorkerResponse};
use crate::utils::{os, timings::Timings};

/// Starts the filesystem watcher thread.
pub(super) fn start(cmd_rx: Receiver<WatchCommand>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        let (ev_tx, ev_rx) = unbounded::<notify::Result<Event>>();

        let mut watcher = match RecommendedWatcher::new(
            move |result| {
                let _ = ev_tx.send(result);
            },
            NotifyConfig::default(),
        ) {
            Ok(watcher) => watcher,
            Err(e) => {
                let _ = res_tx.send(WorkerResponse::Error(format!(
                    "Watch error: failed to create watcher: {}",
                    e
                )));
                return;
            }
        };

        let config_path = os::default_config_path();
        let config_name = config_path
            .file_name()
            .map(|n| n.to_os_string())
            .unwrap_or_default();

        let config_dir = config_path.parent().map(Path::to_path_buf);
        let mut config_watched: Vec<PathBuf> = Vec::new();
        if let Some(watched_dir) = config_dir.as_deref().and_then(resolve_config_watch_dir) {
            match watcher.watch(&watched_dir, RecursiveMode::NonRecursive) {
                Ok(()) => config_watched.push(watched_dir),
                Err(e) => {
                    let _ =
                        res_tx.send(WorkerResponse::Error(format!("Config watch error: {}", e)));
                }
            }
        }

        let mut watched: HashSet<PathBuf> = HashSet::new();
        let mut pending: HashSet<PathBuf> = HashSet::new();
        let mut deadline: Option<Instant> = None;
        let debounce = Duration::from_millis(Timings::FS_WATCH_DEBOUNCE_MS);

        loop {
            let timer = match deadline {
                Some(d) => crossbeam_channel::after(d.saturating_duration_since(Instant::now())),
                None => crossbeam_channel::never(),
            };

            crossbeam_channel::select! {
                recv(cmd_rx) -> msg => match msg {
                    Ok(WatchCommand::Retarget(dirs)) => {
                        retarget_listing_watch(
                            &mut watcher,
                            &mut watched,
                            dirs,
                            &config_watched,
                        );
                    }
                    // All senders dropped: runa is shutting down.
                    Err(_) => break,
                },
                recv(ev_rx) -> msg => {
                    if let Ok(Ok(event)) = msg {
                        if let Some(dir) = &config_dir
                            && !config_watched.contains(dir)
                            && appeared(&event, dir)
                            && watcher.watch(dir, RecursiveMode::NonRecursive).is_ok()
                        {
                            config_watched.push(dir.clone());
                            if config_path.is_file() {
                                let _ = res_tx.send(WorkerResponse::ConfigChanged);
                            }
                        }
                        classify_watch_event(
                            &event,
                            &config_path,
                            config_name.as_os_str(),
                            &watched,
                            &res_tx,
                            &mut pending,
                        );
                        if !pending.is_empty() && deadline.is_none() {
                            deadline = Some(Instant::now() + debounce);
                        }
                    }
                },
                recv(timer) -> _ => {
                    if !pending.is_empty() {
                        let dirs: Vec<PathBuf> = pending.drain().collect();
                        let _ = res_tx.send(WorkerResponse::DirsChanged { dirs });
                    }
                    deadline = None;
                },
            }
        }
    });
}

fn retarget_listing_watch(
    watcher: &mut RecommendedWatcher,
    watched: &mut HashSet<PathBuf>,
    dirs: Vec<PathBuf>,
    config_watched: &[PathBuf],
) {
    let new: HashSet<PathBuf> = dirs.into_iter().collect();

    for old in watched.iter() {
        if !new.contains(old) && !config_watched.contains(old) {
            let _ = watcher.unwatch(old);
        }
    }

    for dir in new.iter() {
        if !watched.contains(dir) && !config_watched.contains(dir) {
            let _ = watcher.watch(dir, RecursiveMode::NonRecursive);
        }
    }

    *watched = new;
}

fn classify_watch_event(
    event: &Event,
    config_path: &Path,
    config_name: &OsStr,
    watched: &HashSet<PathBuf>,
    res_tx: &Sender<WorkerResponse>,
    pending: &mut HashSet<PathBuf>,
) {
    if !matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) {
        return;
    }

    if is_config_changed(event, config_path, config_name) {
        let _ = res_tx.send(WorkerResponse::ConfigChanged);
    }

    for path in &event.paths {
        if watched.contains(path) {
            pending.insert(path.clone());
        } else if let Some(parent) = path.parent()
            && watched.contains(parent)
        {
            pending.insert(parent.to_path_buf());
        }
    }
}

fn is_config_changed(event: &Event, config_path: &Path, config_name: &OsStr) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) && event.paths.iter().any(|p| {
        p == config_path
            || (p.file_name().is_some_and(|name| name == config_name)
                && is_config_dir(p.parent(), config_path))
    })
}

fn is_config_dir(dir: Option<&Path>, config_path: &Path) -> bool {
    let (Some(dir), Some(config_dir)) = (dir, config_path.parent()) else {
        return false;
    };
    matches!((fs::canonicalize(dir), fs::canonicalize(config_dir)), (Ok(a), Ok(b)) if a == b)
}

fn appeared(event: &Event, path: &Path) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(ModifyKind::Name(_))
    ) && event.paths.iter().any(|p| p == path)
}

fn resolve_config_watch_dir(config_dir: &Path) -> Option<PathBuf> {
    if config_dir.is_dir() {
        return Some(config_dir.to_path_buf());
    }

    config_dir
        .parent()
        .filter(|parent| parent.is_dir())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    use notify::event::CreateKind;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn resolve_config_watch_dir_config() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let config_dir = temp.path().join("runa");
        fs::create_dir_all(&config_dir)?;

        let resolved = resolve_config_watch_dir(&config_dir).expect("Expected watch dir");
        assert_eq!(resolved, config_dir);
        Ok(())
    }

    #[test]
    fn resolve_config_watch_dir_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let config_dir = temp.path().join("runa");

        let resolved = resolve_config_watch_dir(&config_dir).expect("Expected watch dir");
        assert_eq!(resolved, temp.path());
        Ok(())
    }

    #[test]
    fn only_the_config_file_is_a_config_change() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let config_dir = temp.path().join("runa");
        let other_dir = temp.path().join("dotfiles");
        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(&other_dir)?;
        let config_path = config_dir.join("runa.toml");
        let name = OsStr::new("runa.toml");
        let created =
            |path: PathBuf| Event::new(EventKind::Create(CreateKind::File)).add_path(path);

        assert!(is_config_changed(
            &created(config_path.clone()),
            &config_path,
            name
        ));
        assert!(!is_config_changed(
            &created(other_dir.join("runa.toml")),
            &config_path,
            name
        ));
        Ok(())
    }
}
