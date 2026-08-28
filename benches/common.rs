//! Shared helpers for the bench areas.

#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// Scratch directory, removed when it drops. Benches touch nothing outside it.
pub struct Scratch {
    dir: TempDir,
}

impl Scratch {
    pub fn new() -> Self {
        Self {
            dir: TempDir::new().expect("failed to create scratch directory"),
        }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// One entry in five is a subdirectory, to exercise `dirs_first`.
    pub fn listing_dir(&self, files: usize) -> PathBuf {
        let dir = self.dir.path().join("listing");
        fs::create_dir_all(&dir).expect("create listing dir");
        for i in 0..files {
            if i % 5 == 0 {
                let _ = fs::create_dir(dir.join(format!("subdir_{i}")));
            } else {
                let _ = fs::write(dir.join(format!("entry_{i}.txt")), b"x");
            }
        }
        dir
    }

    pub fn text_file(&self, name: &str, lines: usize) -> PathBuf {
        let path = self.dir.path().join(name);
        let file = fs::File::create(&path).expect("create text file");
        let mut out = std::io::BufWriter::new(file);
        for i in 0..lines {
            writeln!(
                out,
                "line {i:06} - the quick brown fox jumps over the lazy dog"
            )
            .expect("write line");
        }
        out.flush().expect("flush");
        path
    }
}

impl Default for Scratch {
    fn default() -> Self {
        Self::new()
    }
}

pub const SIZES: [usize; 3] = [100, 1_000, 10_000];

pub fn bytes(n: usize) -> String {
    const KIB: f64 = 1024.0;
    let n = n as f64;
    if n < KIB {
        format!("{n:.0} B")
    } else if n < KIB * KIB {
        format!("{:.1} KiB", n / KIB)
    } else {
        format!("{:.2} MiB", n / (KIB * KIB))
    }
}

pub fn memory_header(title: &str) {
    println!("\n=== {title}: memory (computed, deterministic) ===\n");
}
