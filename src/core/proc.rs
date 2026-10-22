//! The runa processes module.
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
//! Falls back to internal core/preview::safe_read_preview if bat is not available or throws and error.
//!
//! The module also includes [complete_dirs_with_fd] function to enable the move file function to have
//! auto-completion of paths via fd.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::io::{self, BufRead, Read};
use std::path::MAIN_SEPARATOR;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant};

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::utils::{os, path};

/// The size of the buffer reader used to read the output of fd
/// This value is set to 32KB to balance memory usage and performance.
/// Larger buffers may improve performance for large outputs,
/// but will also increase memory consumption.
const BUFREADER_SIZE: usize = 32768;

/// Most bytes read from a preview command, so a tool can never fill up the memory.
const PREVIEW_COMMAND_MAX_BYTES: u64 = 1024 * 1024;

/// A list of common directories and files to exclude from the search.
/// This helps to speed up the search and avoid irrelevant results.
#[rustfmt::skip]
const EXCLUDES: &[&str] = &[
    ".git", ".hg", ".svn", ".rustup", ".cargo", "target", "node_modules", "dist", ".local",
    "venv", ".venv", "__pycache__", ".DS_Store", "build", "out", "bin", "obj", ".cache",
];

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
        path::normalize_search_path(rel)
    }
}

/// An internal struct to hold raw results from the fuzzy matching process.
#[derive(Debug, Clone)]
struct RawResult {
    relative: String,
    score: i64,
}

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

    if max_results == 0 {
        return Ok(());
    }

    let max_res_str = max_results.to_string();

    let fd_bin = os::fd_binary()?;

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

    let norm_query = path::normalize_separators(query);
    let flat_query = path::flatten_separators(&norm_query);

    if let Some(stdout) = proc.stdout.take() {
        let mut reader = io::BufReader::with_capacity(BUFREADER_SIZE, stdout);
        let mut line_buffer = String::with_capacity(256);

        while reader.read_line(&mut line_buffer)? > 0 {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = proc.kill();
                let _ = proc.wait();
                break;
            }
            let rel = line_buffer.trim();
            let norm_rel = path::normalize_separators(rel);
            let flat_rel = path::flatten_separators(&norm_rel);
            if let Some(score) = matcher.fuzzy_match(&flat_rel, &flat_query) {
                raw_results.push(RawResult {
                    relative: norm_rel.into_owned(),
                    score,
                });
            }
            line_buffer.clear();
        }
        let _ = proc.wait();
    }

    if raw_results.len() > max_results {
        raw_results.select_nth_unstable_by_key(max_results - 1, |raw| std::cmp::Reverse(raw.score));
        raw_results.truncate(max_results);
    }
    raw_results.sort_unstable_by_key(|raw| std::cmp::Reverse(raw.score));

    out.reserve(raw_results.len());
    out.extend(raw_results.into_iter().map(|raw| {
        let path = base_dir.join(&raw.relative);
        FindResult {
            path,
            score: raw.score,
        }
    }));

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
    scroll: usize,
) -> Result<Vec<String>, std::io::Error> {
    if max_lines == 0 {
        return Ok(Vec::new());
    }

    let bat_bin = os::bat_binary()?;

    let start = scroll + 1;
    let end = scroll + max_lines;

    let mut args = Vec::with_capacity(bat_args.len() + 1);
    args.extend_from_slice(bat_args);
    args.push(format!("--line-range={}:{}", start, end).into());
    let mut cmd = Command::new(bat_bin)
        .args(args)
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut output = String::new();
    if let Some(mut stdout) = cmd.stdout.take() {
        stdout.read_to_string(&mut output)?;
    }

    let _ = cmd.wait();

    let mut lines = Vec::with_capacity(max_lines);
    for line in output.lines().take(max_lines) {
        lines.push(line.to_owned());
    }

    if lines.is_empty() {
        return Err(std::io::Error::other("bat produced no output"));
    }

    Ok(lines)
}

pub(crate) fn preview_command(
    programm: &OsStr,
    args: &[OsString],
    dir: Option<&Path>,
    max_lines: usize,
    timeout: Duration,
) -> io::Result<Vec<String>> {
    if max_lines == 0 {
        return Ok(Vec::new());
    }

    let mut cmd = Command::new(programm);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }

    let mut proc = cmd.spawn()?;
    let Some(stdout) = proc.stdout.take() else {
        let _ = proc.kill();
        let _ = proc.wait();
        return Err(io::Error::other("preview command has no output"));
    };

    let (line_tx, line_rx) = crossbeam_channel::bounded(max_lines);
    thread::spawn(move || {
        let mut reader = io::BufReader::new(stdout.take(PREVIEW_COMMAND_MAX_BYTES));
        let mut buf = Vec::new();

        while matches!(reader.read_until(b'\n', &mut buf), Ok(read) if read > 0) {
            let line = String::from_utf8_lossy(&buf);
            if line_tx
                .send(line.trim_end_matches(['\r', '\n']).to_owned())
                .is_err()
            {
                break;
            }

            buf.clear();
        }
    });

    let deadline = Instant::now() + timeout;
    let mut lines = Vec::with_capacity(max_lines);
    while lines.len() < max_lines {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let Ok(line) = line_rx.recv_timeout(remaining) else {
            break;
        };

        lines.push(line);
    }

    let _ = proc.kill();
    let _ = proc.wait();

    if lines.is_empty() {
        return Err(io::Error::other("preview command printed nothing"));
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
    let fd_bin = os::fd_binary()?;

    let mut cmd = Command::new(fd_bin);
    cmd.arg("--type").arg("d").arg("--max-depth").arg("1");
    if show_hidden {
        cmd.arg("--hidden");
    }

    let pattern = format!("^{}", prefix);
    cmd.arg(&pattern).arg(base_dir);
    let output = cmd.output()?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "fd exited with status: {:?}",
            output.status
        )));
    }

    let mut dirs: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim_end_matches(MAIN_SEPARATOR).to_string())
        .collect();
    dirs.sort_unstable_by_key(|a| a.to_lowercase());

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
    /// Uses runa's PATH resolver to check for the presence of `fd`.
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

        let preview = preview_bat(&file_path, 2, &[], 0)?;
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
    fn preview_bat_nonexistent_file() -> Result<(), Box<dyn std::error::Error>> {
        skip_if_no_bat!();

        let dir = tempfile::tempdir()?;
        let bad_path = dir.path().join("does_not_exist.txt");
        let result = preview_bat(&bad_path, 2, &[], 0);

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
}
