//! The (fuzzy) find module for the find function in runa
//!
//! This module implements the [find] function and the [FindResult].
//!
//! The [FindResult] struct is used to correctly display the calculated results of the
//! find function. It is used mainly by ui/actions.

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::cmp::Ordering;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// A single result from the find function.
/// It contains the path, whether it is a directory, and the score of the fuzzy match.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FindResult {
    path: PathBuf,
    is_dir: bool,
    score: i64,
}

/// Implement ordering for FindResult based on score (higher is better).
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

struct RawResult {
    relative: String,
    score: i64,
}

/// Perform a fuzzy find using the fd command-line tool and the fuzzy_matcher crate.
pub fn find(
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

    let mut cmd = Command::new("fd");
    cmd.arg(".")
        .arg(base_dir)
        .args([
            "--type",
            "f",
            "--type",
            "d",
            "--hidden",
            "--color",
            "never",
            "--max-results",
            &max_results.to_string(),
        ])
        .stdout(Stdio::piped());

    let mut proc = match cmd.spawn() {
        Ok(proc) => proc,
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                return Err(io::Error::other(
                    "fd was not found in PATH. Please install fd-find",
                ));
            } else {
                return Err(io::Error::other(format!("Failed to spawn fd: {}", e)));
            }
        }
    };

    let matcher = SkimMatcherV2::default();
    let mut raw_results: Vec<RawResult> = Vec::with_capacity(max_results * 2);

    let norm_query = normalize_separators(query);

    if let Some(stdout) = proc.stdout.take() {
        let reader = io::BufReader::new(stdout);

        for line in reader.lines() {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = proc.kill();
                break;
            }
            let rel = line?;
            let rel = rel.trim();
            let norm_rel = normalize_separators(rel);
            if let Some(score) = matcher.fuzzy_match(&norm_rel, &norm_query) {
                raw_results.push(RawResult {
                    relative: rel.to_owned(),
                    score,
                });
            }
        }
    }

    raw_results.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    raw_results.truncate(max_results);

    let mut results = Vec::with_capacity(raw_results.len());
    for raw in raw_results {
        let path = base_dir.join(&raw.relative);
        let is_dir = path.is_dir();
        results.push(FindResult {
            path,
            is_dir,
            score: raw.score,
        });
    }

    out.extend(results);

    Ok(())
}

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

fn normalize_separators(separator: &str) -> String {
    separator.replace('\\', "/")
}
