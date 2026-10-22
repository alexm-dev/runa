//! File and archive previews for runa.
//!
//! Reads the lines of a file preview and builds the tree rows of an archive listing.

use std::fs::File;
use std::io::{BufReader, ErrorKind, Read, Seek};
use std::path::Path;

use unicode_width::UnicodeWidthChar;

use crate::utils::os;

/// Minimum number of lines shown in any preview
const MIN_PREVIEW_LINES: usize = 3;
/// Maximum file size allowed for preview (5gb)
const MAX_PREVIEW_SIZE: u64 = 5_000 * 1024 * 1024;
/// Number of bytes to peek from file start for header checks (eg. PNG, ZIP, etc..)
const HEADER_PEEK_BYTES: usize = 8;
/// Bytes to peek for null bytes in binary detections
const BINARY_PEEK_BYTES: usize = 1024;
/// Size of the buffer used to read files for previews.
const READ_BUFFER_SIZE: usize = 64 * 1024;
/// Maximum bytes kept per preview lines.
/// Longer lines are cut to keep previews for minified files fast.
const MAX_LINE_BYTES: usize = 1024;

/// Message shown instead of a preview for anything that is not a regular file.
const NOT_REGULAR_FILE_MSG: &str = "[Not a regular file - preview skipped]";
/// Message shown instead of a preview for binary files.
const BINARY_FILE_PREVIEW_MSG: &str = "[Binary file - preview hidden]";

/// Calculating the pane widht and clean the output to the widht of the pane
/// by removing control characters, expanding tabs to 4 spaces,
/// and truncating or padding the string to fit exactly.
/// # Returns
/// A sanitized string that fits exactly within the specified pane width.
pub(crate) fn sanitize_to_exact_width(line: &str, pane_width: usize) -> String {
    let mut out = String::with_capacity(pane_width);
    let mut current_w = 0;

    for ch in line.chars() {
        if ch == '\t' {
            let space_count = 4 - (current_w % 4);
            if current_w + space_count > pane_width {
                break;
            }
            for _ in 0..space_count {
                out.push(' ');
            }
            current_w += space_count;
            continue;
        }

        if ch.is_control() {
            continue;
        }

        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);

        if current_w + ch_width > pane_width {
            break;
        }

        out.push(ch);
        current_w += ch_width;
    }

    while current_w < pane_width {
        out.push(' ');
        current_w += 1;
    }

    out
}

/// Loads a preview for any path (directory or file), returning an error or a padded lines for
/// display.
/// large binaries/unreadable and unsupported files are replaced with a notice.
///
/// # Returns
/// A vector of strings, each representing a line from the file or directory preview.
pub(crate) fn safe_read_preview(
    path: &Path,
    max_lines: usize,
    pane_width: usize,
    scroll: usize,
) -> Vec<String> {
    let max_lines = std::cmp::max(max_lines, MIN_PREVIEW_LINES);

    if !os::is_regular_file(path) {
        return vec![sanitize_to_exact_width(NOT_REGULAR_FILE_MSG, pane_width)];
    }

    // File Read and binary Check
    match File::open(path) {
        Ok(mut file) => {
            if let Ok(metadata) = file.metadata() {
                if !metadata.is_file() {
                    return vec![sanitize_to_exact_width(NOT_REGULAR_FILE_MSG, pane_width)];
                }

                if metadata.len() > MAX_PREVIEW_SIZE {
                    return vec![sanitize_to_exact_width(
                        "[File too large for preview]",
                        pane_width,
                    )];
                }
            }

            // Peek for null bytes to detect binary files
            let mut buffer = [0u8; BINARY_PEEK_BYTES];
            let n = file.read(&mut buffer).unwrap_or(0);

            let header_len = std::cmp::min(n, HEADER_PEEK_BYTES);
            let header = &buffer[..header_len];

            if header.len() >= 5 && &header[..5] == b"%PDF-" {
                return vec![sanitize_to_exact_width(BINARY_FILE_PREVIEW_MSG, pane_width)];
            }

            if buffer[..n].contains(&0) {
                return vec![sanitize_to_exact_width(BINARY_FILE_PREVIEW_MSG, pane_width)];
            }

            // Rewind to start for full read
            let _ = file.rewind();

            let mut reader = BufReader::with_capacity(READ_BUFFER_SIZE, file);
            let mut buf = [0u8; READ_BUFFER_SIZE];

            let mut preview_lines = Vec::with_capacity(max_lines);
            let mut current_line = Vec::with_capacity(256);

            let mut line_idx = 0usize;
            let mut collected = 0usize;

            loop {
                let n = match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };

                for &b in &buf[..n] {
                    if b == b'\n' {
                        if line_idx >= scroll {
                            if let Some(&last) = current_line.last()
                                && last == b'\r'
                            {
                                current_line.pop();
                            }

                            let line = String::from_utf8_lossy(&current_line);
                            preview_lines.push(sanitize_to_exact_width(&line, pane_width));
                            collected += 1;

                            if collected >= max_lines {
                                return preview_lines;
                            }
                        }

                        current_line.clear();
                        line_idx += 1;
                    } else {
                        if current_line.len() < MAX_LINE_BYTES {
                            current_line.push(b);
                        }
                    }
                }
            }

            if !current_line.is_empty() && line_idx >= scroll && collected < max_lines {
                if current_line.last() == Some(&b'\r') {
                    current_line.pop();
                }
                let line = String::from_utf8_lossy(&current_line);
                preview_lines.push(sanitize_to_exact_width(&line, pane_width));
            }

            if preview_lines.is_empty() {
                let msg = if scroll == 0 {
                    "[Empty file]"
                } else {
                    "[End of file]"
                };
                preview_lines.push(sanitize_to_exact_width(msg, pane_width));
            }

            preview_lines
        }
        Err(e) => {
            let msg = match e.kind() {
                ErrorKind::PermissionDenied => "[Error: Permission Denied]",
                ErrorKind::NotFound => "[Error: File Not Found]",
                _ => {
                    return vec![sanitize_to_exact_width(
                        &format!("[Error reading file: {}]", e),
                        pane_width,
                    )];
                }
            };
            vec![sanitize_to_exact_width(msg, pane_width)]
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ListingRow {
    buf: Box<str>,
    name_len: usize,
    depth: usize,
    is_dir: bool,
}

impl ListingRow {
    fn new(name: &str, depth: usize, is_dir: bool) -> Self {
        let ext = name
            .rsplit_once('.')
            .filter(|(base, ext)| !is_dir && !base.is_empty() && !ext.is_empty())
            .map_or("", |(_, ext)| ext);

        let mut buf = String::with_capacity(name.len() + ext.len());
        buf.push_str(name);
        buf.extend(ext.chars().flat_map(char::to_lowercase));

        Self {
            buf: buf.into_boxed_str(),
            name_len: name.len(),
            depth,
            is_dir,
        }
    }

    crate::getters! {
        depth: usize,
        is_dir: bool,
    }

    pub(crate) fn name(&self) -> &str {
        &self.buf[..self.name_len]
    }

    pub(crate) fn ext(&self) -> Option<&str> {
        let ext = &self.buf[self.name_len..];
        (!ext.is_empty()).then_some(ext)
    }
}

pub(crate) fn tree_rows(paths: &[String]) -> Vec<ListingRow> {
    let mut rows = Vec::with_capacity(paths.len());
    let mut open: Vec<&str> = Vec::new();

    for path in paths {
        let path = path.trim_start_matches("./").trim_start_matches('/');
        if path.is_empty() || path.ends_with('/') {
            continue;
        }

        let (dirs, file) = path.rsplit_once('/').unwrap_or(("", path));
        let parts = || {
            dirs.split('/')
                .filter(|part| !part.is_empty() && *part != ".")
        };

        let shared = open
            .iter()
            .zip(parts())
            .take_while(|(open, part)| *open == part)
            .count();
        open.truncate(shared);

        for dir in parts().skip(shared) {
            rows.push(ListingRow::new(dir, open.len(), true));
            open.push(dir);
        }
        rows.push(ListingRow::new(file, open.len(), false));
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_sanitization_and_exact_width() {
        let pane_width = 10;

        let cases = vec![
            ("short.txt", 10),
            ("very_long_filename.txt", 10),
            ("🦀_crab.rs", 10),
            ("\t_tab", 10),
        ];

        for (input, expected_width) in cases {
            let result = sanitize_to_exact_width(input, pane_width);

            let actual_width = unicode_width::UnicodeWidthStr::width(result.as_str());

            assert_eq!(
                actual_width, expected_width,
                "Failed to produce exact width for input: '{}'. Result was: '{}' (width: {})",
                input, result, actual_width
            );

            assert!(
                !result.chars().any(|c| c.is_control() && c != ' '),
                "Result contains control characters: {:?}",
                result
            );
        }
    }
}
