use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ignore::WalkBuilder;
use num_cpus;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FindResult {
    path: PathBuf,
    relative: String,
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

    pub fn relative(&self) -> &str {
        &self.relative
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn score(&self) -> i64 {
        self.score
    }
}

pub fn find_recursive(
    root: &Path,
    query: &str,
    out: &mut Vec<FindResult>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
) -> io::Result<()> {
    out.clear();
    if query.is_empty() {
        return Ok(());
    }

    const MAX_FIND_RESULTS: usize = 15;

    let results: Arc<Mutex<BinaryHeap<(i64, PathBuf, bool)>>> =
        Arc::new(Mutex::new(BinaryHeap::with_capacity(MAX_FIND_RESULTS + 1)));

    let query_str = query.to_owned();
    let root_buf = root.to_path_buf();

    WalkBuilder::new(root)
        .standard_filters(true)
        .threads(num_cpus::get())
        .build_parallel()
        .run(|| {
            let results = Arc::clone(&results);
            let query = query_str.clone();
            let root_ref = root_buf.clone();
            let matcher = SkimMatcherV2::default();
            let cancel = Arc::clone(&cancel);

            Box::new(move |entry| {
                if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    return ignore::WalkState::Quit;
                }

                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => return ignore::WalkState::Continue,
                };

                let rel_path = entry.path().strip_prefix(&root_ref).unwrap_or(entry.path());
                let rel_str = rel_path.to_string_lossy();
                let match_target = rel_str.replace(['/', '\\'], "");

                if let Some(score) = matcher.fuzzy_match(&match_target, &query) {
                    if let Ok(mut guard) = results.lock() {
                        if guard.len() < MAX_FIND_RESULTS
                            || score > guard.peek().map(|(s, _, _)| *s).unwrap_or(0)
                        {
                            guard.push((
                                score,
                                entry.path().to_path_buf(),
                                entry.file_type().map(|f| f.is_dir()).unwrap_or(false),
                            ));

                            if guard.len() > MAX_FIND_RESULTS {
                                guard.pop();
                            }
                        }
                    }
                }
                ignore::WalkState::Continue
            })
        });

    let heap = Arc::try_unwrap(results)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Thread synchronization failed"))?
        .into_inner()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Mutex poisoned by a panicked thread"))?;

    let mut raw_results: Vec<_> = heap.into_vec();

    raw_results.sort_by(|a, b| b.0.cmp(&a.0));

    for (score, path, is_dir) in raw_results {
        let rel = path.strip_prefix(&root_buf).unwrap_or(&path);
        let mut relative = rel.to_string_lossy().into_owned();

        #[cfg(windows)]
        {
            relative = relative.replace('\\', "/");
        }

        out.push(FindResult {
            path,
            relative,
            is_dir,
            score,
        });
    }

    Ok(())
}
