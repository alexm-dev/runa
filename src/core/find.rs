//! The (fuzzy) find module for the find function in runa
//!
//! This module implements the [find] function, the [FindResult] and the [RawResult] structs.
//!
//! The [FindResult] struct is used to correctly display the calculated results of the
//! find function. It is used mainly by ui/actions.
//!
//! The [RawResult] struct is an internal struct used to store intermediate results
//! during the find process.
//!
//! The [find] function uses the fd command-line tool to perform a file search
//! in the specified base directory. It then applies fuzzy matching using the
//! fuzzy_matcher crate to filter and score the results based on the provided query.
//! The results are returned as a vector of [FindResult] structs, sorted by their
//! fuzzy match scores.

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// The size of the buffer reader used to read the output of fd
const BUFREADER_SIZE: usize = 32768;

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
        let reader = io::BufReader::with_capacity(BUFREADER_SIZE, stdout);

        for line in reader.lines() {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = proc.kill();
                let _ = proc.wait();
                break;
            }
            let rel = line?;
            let rel = rel.trim();
            let norm_rel = normalize_separators(rel);
            if let Some(score) = matcher.fuzzy_match(&norm_rel, &norm_query) {
                raw_results.push(RawResult {
                    relative: norm_rel.into_owned(),
                    score,
                });
            }
        }
        let _ = proc.wait();
    }

    raw_results.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    raw_results.truncate(max_results);

    out.reserve(raw_results.len());
    for raw in raw_results {
        let path = base_dir.join(&raw.relative);
        let is_dir = path.is_dir();
        out.push(FindResult {
            path,
            is_dir,
            score: raw.score,
        });
    }
    Ok(())
}

/// Helpers:
///
/// Normalize a relative path to use forward slashes for consistency across platforms.
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

/// Normalize separators in a given string to use forward slashes.
fn normalize_separators<'a>(separator: &'a str) -> Cow<'a, str> {
    if separator.contains('\\') {
        Cow::Owned(separator.replace('\\', "/"))
    } else {
        Cow::Borrowed(separator)
    }
}
