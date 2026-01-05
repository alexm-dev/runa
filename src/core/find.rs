//! The (fuzzy) find module for the find function in runa
//!
//! This module implements the [find_recursive()] function and the [FindResult].
//!
//! The [FindResult] struct is used to correctly dispaly the calculated results of the
//! find_recursive function. It is used mainly by ui/actions

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ignore::WalkBuilder;
use num_cpus;
use parking_lot::Mutex;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FindResult {
    path: PathBuf,
    is_dir: bool,
    score: i64,
}

impl Ord for FindResult {
    fn cmp(&self, other: &Self) -> Ordering {
        other.score.cmp(&self.score)
    }
}

impl PartialOrd for FindResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl FindResult {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn score(&self) -> i64 {
        self.score
    }

    pub fn relative(&self, base: &Path) -> String {
        let rel = self.path.strip_prefix(base).unwrap_or(&self.path);
        normalize_relative_path(rel)
    }
}

/// Performs a fuzzy, parallel filesystem search at root.
///
/// Returns up to max_results best matches in out which is sorted by a score.
pub fn find_recursive(
    base_dir: &Path,
    query: &str,
    out: &mut Vec<FindResult>,
    cancel: Arc<AtomicBool>,
    max_results: usize,
) -> io::Result<()> {
    out.clear();
    if query.is_empty() {
        return Ok(());
    }

    let thread_heaps: Arc<Mutex<Vec<BinaryHeap<(Reverse<i64>, PathBuf, bool)>>>> =
        Arc::new(Mutex::new(Vec::new()));

    let query = Arc::new(query.to_owned());
    let root_buf = Arc::new(base_dir.to_path_buf());

    WalkBuilder::new(base_dir)
        .standard_filters(true)
        .hidden(false)
        .threads(num_cpus::get().saturating_sub(1).max(1))
        .build_parallel()
        .run(|| {
            let thread_heaps = Arc::clone(&thread_heaps);
            let query = Arc::clone(&query);
            let root_ref = Arc::clone(&root_buf);
            let matcher = SkimMatcherV2::default();
            let cancel = Arc::clone(&cancel);

            struct HeapGuard {
                local_heap: BinaryHeap<(Reverse<i64>, PathBuf, bool)>,
                thread_heaps: Arc<Mutex<Vec<BinaryHeap<(Reverse<i64>, PathBuf, bool)>>>>,
            }

            impl Drop for HeapGuard {
                fn drop(&mut self) {
                    self.thread_heaps
                        .lock()
                        .push(std::mem::take(&mut self.local_heap));
                }
            }

            let mut guard = HeapGuard {
                local_heap: BinaryHeap::with_capacity(max_results + 1),
                thread_heaps,
            };

            Box::new(move |entry| {
                if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    return ignore::WalkState::Quit;
                }

                let entry: ignore::DirEntry = match entry {
                    Ok(e) => e,
                    Err(_) => return ignore::WalkState::Continue,
                };

                let path = entry.path();

                let rel_path = path.strip_prefix(&*root_ref).unwrap_or(path);
                let rel_str = match rel_path.to_str() {
                    Some(s) => s,
                    None => return ignore::WalkState::Continue,
                };

                if let Some(score) = matcher.fuzzy_match(rel_str, &query) {
                    if guard.local_heap.len() < max_results
                        || score > (guard.local_heap.peek().map(|(s, _, _)| s.0).unwrap_or(0))
                    {
                        let is_dir = entry.file_type().map(|file| file.is_dir()).unwrap_or(false);
                        guard
                            .local_heap
                            .push((Reverse(score), path.to_path_buf(), is_dir));
                        if guard.local_heap.len() > max_results {
                            guard.local_heap.pop();
                        }
                    }
                }
                ignore::WalkState::Continue
            })
        });

    let mut final_heap = BinaryHeap::with_capacity(max_results);
    let mut locked_heaps = thread_heaps.lock();

    for mut heap in locked_heaps.drain(..) {
        for (rev_score, path, is_dir) in heap.drain() {
            let score = rev_score.0;

            if final_heap.len() < max_results
                || score
                    > final_heap
                        .peek()
                        .map(|(s, _, _): &(Reverse<i64>, PathBuf, bool)| s.0)
                        .unwrap_or(0)
            {
                final_heap.push((Reverse(score), path, is_dir));
                if final_heap.len() > max_results {
                    final_heap.pop();
                }
            }
        }
    }

    let mut raw_results: Vec<_> = final_heap.into_vec();
    raw_results.sort_unstable_by(|a, b| b.0.cmp(&a.0));

    for (score, path, is_dir) in raw_results {
        out.push(FindResult {
            path,
            is_dir,
            score: score.0,
        });
    }

    Ok(())
}

/// Helper to normalize the relative paths for each OS used for the find functions.
fn normalize_relative_path(path: &Path) -> String {
    let rel = path.to_string_lossy().into_owned();
    #[cfg(windows)]
    {
        rel.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        rel
    }
}
