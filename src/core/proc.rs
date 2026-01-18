//! The runa processes module.
//!
//! This module implements the [find] and the [preview_bat] function, the [FindResult] and the RawResult structs.
//!
//! The [FindResult] struct is used to correctly display the calculated results of the
//! find function. It is used mainly by ui/actions.
//!
//! The RawResult struct is an internal struct used to store intermediate results
//! during the find process.
//!
//! The [find] function uses the fd command-line tool to perform a file search
//! in the specified base directory. It then applies fuzzy matching using the
//! fuzzy_matcher crate to filter and score the results based on the provided query.
//! The results are returned as a vector of [FindResult] structs, sorted by their
//! fuzzy match scores.
//!
//! The module also includes a [preview_bat] function that uses the bat command-line tool
//! to preview the contents of a file, returning a specified number of lines from the file.
//! This function is used by core/workers.rs to provide file previews in the UI.
//! Falls back to internal core/formatter::safe_read_preview if bat is not available or throws and error.

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use std::borrow::Cow;
use std::cmp::Ordering;
use std::ffi::OsString;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// The size of the buffer reader used to read the output of fd
/// This value is set to 32KB to balance memory usage and performance.
/// Larger buffers may improve performance for large outputs,
/// but will also increase memory consumption.
const BUFREADER_SIZE: usize = 32768;

/// A list of common directories and files to exclude from the search.
/// This helps to speed up the search and avoid irrelevant results.
#[rustfmt::skip]
const EXCLUDES: &[&str] = &[
    ".git", ".hg", ".svn", ".rustup", ".cargo", "target", "node_modules", "dist",
    "venv", ".venv", "__pycache__", ".DS_Store", "build", "out", "bin", "obj"
];

/// A single result from the find function.
/// It contains the path and the score of the fuzzy match.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FindResult {
    path: PathBuf,
    score: i64,
}

/// Implement ordering for FindResult based on score (higher is better).
/// This allows sorting of FindResult instances.
impl Ord for FindResult {
    fn cmp(&self, other: &Self) -> Ordering {
        other.score.cmp(&self.score)
    }
}

/// Implement partial ordering for FindResult.
/// This is required because we implemented Ord.
impl PartialOrd for FindResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl FindResult {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn score(&self) -> i64 {
        self.score
    }
    pub fn relative(&self, base: &Path) -> String {
        let rel = self.path.strip_prefix(base).unwrap_or(&self.path);
        normalize_relative_path(rel)
    }
}

/// An internal struct to hold raw results from the fuzzy matching process.
/// It contains the relative path and the score.
#[derive(Debug, Clone)]
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

    let mut args: Vec<OsString> = vec![
        OsString::from("."),
        OsString::from(base_dir),
        OsString::from("--type"),
        OsString::from("f"),
        OsString::from("--type"),
        OsString::from("d"),
        OsString::from("--hidden"),
    ];

    for excl in EXCLUDES {
        args.push(OsString::from("--exclude"));
        args.push(OsString::from(excl));
    }

    args.push(OsString::from("--color"));
    args.push(OsString::from("never"));
    args.push(OsString::from("--max-results"));
    args.push(OsString::from(max_results.to_string()));

    let mut cmd = Command::new("fd");
    cmd.args(&args).stdout(Stdio::piped());

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
    let flat_query = flatten_separators(&norm_query);

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
            let flat_rel = flatten_separators(&norm_rel);
            if let Some(score) = matcher.fuzzy_match(&flat_rel, &flat_query) {
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
        out.push(FindResult {
            path,
            score: raw.score,
        });
    }
    Ok(())
}

/// Use bat to preview a file at the given path, returning up to max_lines of output
/// Uses the provided bat_args for customization.
///
/// # Returns
/// A vector of strings, each representing a line from the file preview.
pub fn preview_bat(
    path: &Path,
    max_lines: usize,
    bat_args: &[OsString],
) -> Result<Vec<String>, std::io::Error> {
    let mut args = bat_args.to_vec();
    args.push(path.as_os_str().to_os_string());

    let output = Command::new("bat")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;

    if !output.status.success() {
        return Err(std::io::Error::other("bat command failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().take(max_lines).map(str::to_owned).collect())
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

/// Flatten separators by removing all '/' and '\' characters from the string.
/// This is used to create a simplified version of the path for fuzzy matching.
///
/// # Examples
/// let flat = flatten_separators("src/core/proc.rs");
/// flat = "srccoreprocrs";
fn flatten_separators(separator: &str) -> String {
    let mut buf = String::with_capacity(separator.len());
    for char in separator.chars() {
        if char != '/' && char != '\\' {
            buf.push(char);
        }
    }
    buf
}

/// Integration tests for proc
#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::io::Write;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    /// Checks if the `fd` command-line tool is available in the system.
    /// Returns true if `fd` is found, otherwise false.
    /// Uses which crate to check for the presence of `fd`.
    fn fd_available() -> bool {
        which::which("fd").is_ok()
    }

    fn bat_available() -> bool {
        which::which("bat").is_ok()
    }

    /// Macro to skip tests if `fd` is not available.
    macro_rules! skip_if_no_fd {
        () => {
            if !fd_available() {
                return Ok(());
            }
        };
    }

    /// Macro to skip tests if `bat` is not available.
    macro_rules! skip_if_no_bat {
        () => {
            if !bat_available() {
                return Ok(());
            }
        };
    }

    #[test]
    fn test_find_recursive_unit() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_fd!();

        let dir = tempdir()?;
        std::fs::File::create(dir.path().join("crab.txt"))?;
        std::fs::File::create(dir.path().join("other.txt"))?;
        let cancel = Arc::new(AtomicBool::new(false));
        let mut out = Vec::new();
        find(dir.path(), "crab", &mut out, cancel, 11)?;
        let candidate = out
            .iter()
            .find(|r| r.path().file_name().unwrap() == "crab.txt");
        assert!(
            candidate.is_some(),
            "Expected 'crab.txt' in find results. Got: {:?}",
            out.iter()
                .map(|r| r.path().display().to_string())
                .collect::<Vec<_>>()
        );

        let filename = out[0]
            .path()
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("Could not extract valid UTF-8 file name")?;
        assert!(
            filename.contains("crab"),
            "Filename does not contain 'crab': got '{}'",
            filename
        );
        Ok(())
    }

    #[test]
    fn test_find_recursive_empty_query() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_fd!();
        let dir = tempdir()?;
        fs::File::create(dir.path().join("something.txt"))?;
        let cancel = Arc::new(AtomicBool::new(false));
        let mut out = Vec::new();
        find(dir.path(), "", &mut out, cancel, 10)?;
        assert!(out.is_empty());
        Ok(())
    }

    #[test]
    fn test_find_recursive_subdirectory() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_fd!();
        let dir = tempdir()?;
        let subdir = dir.path().join("nested");
        std::fs::create_dir(&subdir)?;
        std::fs::File::create(subdir.join("crabby.rs"))?;
        let cancel = Arc::new(AtomicBool::new(false));
        let mut out = Vec::new();
        find(dir.path(), "crabby", &mut out, cancel, 10)?;
        let candidate = out
            .iter()
            .find(|r| r.path().file_name().unwrap() == "crabby.rs");
        assert!(
            candidate.is_some(),
            "Expected 'crabby.rs' in find results. Got: {:?}",
            out.iter()
                .map(|r| r.path().display().to_string())
                .collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_preview_bat_basic() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_bat!();

        let dir = tempdir()?;
        let file_path = dir.path().join("hello.txt");
        let mut file = fs::File::create(&file_path)?;
        writeln!(file, "line one")?;
        writeln!(file, "line two")?;
        writeln!(file, "line three")?;

        let preview = preview_bat(&file_path, 2, &[])?;
        assert_eq!(preview.len(), 2);
        assert!(
            preview.iter().any(|line| line.contains("line one")),
            "Preview missing expected content"
        );
        assert!(
            preview.iter().any(|line| line.contains("line two")),
            "Preview missing expected content"
        );

        Ok(())
    }

    #[test]
    fn test_preview_bat_with_args() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_bat!();

        let dir = tempdir()?;
        let file_path = dir.path().join("colors.rs");
        let mut file = fs::File::create(&file_path)?;
        writeln!(file, "fn main() {{}}")?;

        let preview = preview_bat(&file_path, 1, &[std::ffi::OsString::from("--plain")])?;
        assert_eq!(preview.len(), 1);
        assert!(preview[0].contains("fn main"));

        Ok(())
    }

    #[test]
    fn test_preview_bat_nonexistent_file() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_bat!();

        let dir = tempfile::tempdir()?;
        let bad_path = dir.path().join("does_not_exist.txt");
        let result = preview_bat(&bad_path, 2, &[]);

        assert!(result.is_err(), "Expected error for missing file");
        Ok(())
    }
}
