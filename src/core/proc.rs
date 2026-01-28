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

use crate::utils::{flatten_separators, normalize_relative_path, normalize_separators};

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use std::borrow::Cow;
use std::cmp::Ordering;
use std::ffi::OsString;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};

/// The size of the buffer reader used to read the output of fd
/// This value is set to 32KB to balance memory usage and performance.
/// Larger buffers may improve performance for large outputs,
/// but will also increase memory consumption.
const BUFREADER_SIZE: usize = 32768;

/// A list of common directories and files to exclude from the search.
/// This helps to speed up the search and avoid irrelevant results.
#[rustfmt::skip]
const EXCLUDES: &[&str] = &[
    ".git", ".hg", ".svn", ".rustup", ".cargo", "target", "node_modules", "dist", ".local",
    "venv", ".venv", "__pycache__", ".DS_Store", "build", "out", "bin", "obj", ".cache",
];

/// A OnceLock to store the fd binary name.
/// This is used to avoid checking for the binary multiple times.
static FD_BIN: OnceLock<Option<&'static str>> = OnceLock::new();

/// OnceLock cache to store the bat binary name.
/// This is used to avoid checking for the binary multiple times.
/// If bat is not found, the value will be None.
static BAT_BIN: OnceLock<Option<&'static str>> = OnceLock::new();

fn cached_binary(
    cache: &'static OnceLock<Option<&'static str>>,
    binaries: &[&'static str],
    err_msg: &'static str,
) -> io::Result<&'static str> {
    cache
        .get_or_init(|| {
            binaries
                .iter()
                .find(|&&bin| which::which(bin).is_ok())
                .copied()
        })
        .as_ref()
        .copied()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, err_msg))
}

#[inline]
pub(crate) fn fd_binary() -> io::Result<&'static str> {
    cached_binary(&FD_BIN, &["fd", "fd-find"], "fd/fd-find not found")
}

#[inline]
fn bat_binary() -> io::Result<&'static str> {
    cached_binary(&BAT_BIN, &["bat"], "bat not found")
}

/// A single result from the find function.
/// It contains the path and the score of the fuzzy match.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct FindResult {
    path: PathBuf,
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
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn relative(&self, base: &Path) -> Cow<'_, str> {
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
pub(crate) fn find(
    base_dir: &Path,
    query: &str,
    out: &mut Vec<FindResult>,
    cancel: Arc<AtomicBool>,
    max_results: usize,
    show_hidden: bool,
) -> io::Result<()> {
    out.clear();
    if query.is_empty() {
        return Ok(());
    }

    let max_res_str = max_results.to_string();

    let fd_bin = fd_binary()?;

    let mut cmd = Command::new(fd_bin);
    cmd.arg(".")
        .arg(base_dir)
        .arg("--type")
        .arg("f")
        .arg("--type")
        .arg("d");

    if show_hidden {
        cmd.arg("--hidden");
    }
    for excl in EXCLUDES {
        cmd.arg("--exclude").arg(excl);
    }
    cmd.arg("--color")
        .arg("never")
        .arg("--max-results")
        .arg(&max_res_str)
        .stdout(Stdio::piped());

    let mut proc = cmd
        .spawn()
        .map_err(|_| io::Error::other("fd/fd-find exectuion failed"))?;

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
/// Returns a vector of strings, each representing a line from the file preview.
pub(crate) fn preview_bat(
    path: &Path,
    max_lines: usize,
    bat_args: &[OsString],
) -> Result<Vec<String>, std::io::Error> {
    let bat_bin = bat_binary()?;

    let mut cmd = Command::new(bat_bin)
        .args(bat_args)
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut lines = Vec::with_capacity(max_lines);

    if let Some(stdout) = cmd.stdout.take() {
        let reader = io::BufReader::new(stdout);
        for line in reader.lines().take(max_lines) {
            match line {
                Ok(l) => lines.push(l),
                Err(_) => break,
            }
        }
    }

    let _ = cmd.kill();
    let _ = cmd.wait();

    if lines.is_empty() {
        return Err(std::io::Error::other("bat produced no output"));
    }

    Ok(lines)
}

/// Auto completion of directories
/// Used by MoveFile inputmode to autocomplete the directory paths.
pub(crate) fn complete_dirs_with_fd(
    base_dir: &Path,
    prefix: &str,
    show_hidden: bool,
) -> Result<Vec<String>, std::io::Error> {
    let fd_bin = fd_binary()?;

    let mut cmd = Command::new(fd_bin);
    cmd.arg("--type").arg("d").arg("--max-depth").arg("1");
    if show_hidden {
        cmd.arg("--hidden");
    }
    cmd.arg(prefix).arg(base_dir);
    let output = cmd.output()?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "fd exited with status: {:?}",
            output.status
        )));
    }

    let dirs = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim_end_matches(std::path::MAIN_SEPARATOR).to_string())
        .collect();

    Ok(dirs)
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
        which::which("fd").is_ok() || which::which("fd-find").is_ok()
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
    fn finds_fd_binary_if_available() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_fd!();
        let bin = fd_binary()?;
        assert!(bin == "fd" || bin == "fd-find");
        assert!(which::which(bin).is_ok());
        Ok(())
    }

    #[test]
    fn finds_bat_binary_if_available() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_bat!();
        let bin = bat_binary()?;
        assert_eq!(bin, "bat");
        assert!(which::which(bin).is_ok());
        Ok(())
    }

    #[test]
    fn find_recursive_unit() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_fd!();

        let dir = tempdir()?;
        std::fs::File::create(dir.path().join("crab.txt"))?;
        std::fs::File::create(dir.path().join("other.txt"))?;
        let cancel = Arc::new(AtomicBool::new(false));
        let mut out = Vec::new();
        find(dir.path(), "crab", &mut out, cancel, 11, false)?;
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
    fn find_recursive_empty_query() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_fd!();
        let dir = tempdir()?;
        fs::File::create(dir.path().join("something.txt"))?;
        let cancel = Arc::new(AtomicBool::new(false));
        let mut out = Vec::new();
        find(dir.path(), "", &mut out, cancel, 10, false)?;
        assert!(out.is_empty());
        Ok(())
    }

    #[test]
    fn find_recursive_subdirectory() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_fd!();
        let dir = tempdir()?;
        let subdir = dir.path().join("nested");
        std::fs::create_dir(&subdir)?;
        std::fs::File::create(subdir.join("crabby.rs"))?;
        let cancel = Arc::new(AtomicBool::new(false));
        let mut out = Vec::new();
        find(dir.path(), "crabby", &mut out, cancel, 10, false)?;
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
    fn preview_bat_basic() -> Result<(), Box<dyn std::error::Error>> {
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
    fn preview_bat_with_args() -> Result<(), Box<dyn std::error::Error>> {
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
    fn preview_bat_nonexistent_file() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_bat!();

        let dir = tempfile::tempdir()?;
        let bad_path = dir.path().join("does_not_exist.txt");
        let result = preview_bat(&bad_path, 2, &[]);

        assert!(result.is_err(), "Expected error for missing file");
        Ok(())
    }

    #[test]
    fn complete_dirs_with_fd_sandboxed() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_fd!();

        let dir = tempdir()?;
        let base_path = dir.path();

        let dirs_to_create = ["photos", "documents", ".hidden_dir"];
        for dir_name in &dirs_to_create {
            std::fs::create_dir(base_path.join(dir_name))?;
        }

        std::fs::write(base_path.join("not_a_dir.txt"), "hello")?;

        let results = complete_dirs_with_fd(base_path, "p", true)?;

        assert!(
            results.iter().any(|r| r.contains("photos")),
            "Should find 'photos'"
        );
        assert!(
            !results.iter().any(|r| r.contains("documents")),
            "Should not find 'documents'"
        );
        assert!(
            !results.iter().any(|r| r.contains("not_a_dir")),
            "Should not find files"
        );

        let all_results = complete_dirs_with_fd(base_path, "", true)?;
        assert!(all_results.len() >= 2);

        Ok(())
    }

    #[test]
    fn fd_missing_or_invalid_path() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_fd!();

        let sandbox = tempdir()?;
        let non_existent = sandbox.path().join("ghost_zone");

        let result = complete_dirs_with_fd(&non_existent, "", true);
        assert!(result.is_err(), "Expected error for non-existent path");

        Ok(())
    }
}
