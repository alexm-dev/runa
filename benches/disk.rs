//! Disk benchmarks for runa.
//!
//! Measures directory loads and previews on real files with 10k entries.
//! Only runs when named directly, a plain cargo bench skips it.
//! The files live in a temporary directory which is deleted at the end.
//!
//! Commands:
//!     cargo bench --bench disk                              run all disk benches
//!     cargo bench --bench disk -- --save-baseline before    save a baseline named before
//!     cargo bench --bench disk -- --baseline before         compare against the baseline

mod common;

use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use criterion::{Criterion, SamplingMode, Throughput, criterion_group, criterion_main};
use runa::bench_api::{LoadFixture, PreviewFixture, SortBy};
use tempfile::TempDir;

use common::config;

const ENTRIES: usize = 10_000;
const LINES: usize = 20_000;

/// Temporary directory with the bench files, deleted when dropped.
struct Sandbox {
    dir: TempDir,
}

impl Sandbox {
    fn build() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("runa-bench-")
            .tempdir()
            .expect("create sandbox");
        let sandbox = Self { dir };

        let listing = sandbox.listing();
        fs::create_dir(&listing).expect("create listing dir");
        for i in 0..ENTRIES {
            // Every fifth entry is a subdirectory for dirs_first
            if i % 5 == 0 {
                fs::create_dir(listing.join(format!("subdir_{i}"))).expect("create subdir");
            } else {
                fs::write(listing.join(format!("entry_{i}.txt")), b"x").expect("write entry");
            }
        }

        let mut out = BufWriter::new(fs::File::create(sandbox.text()).expect("create text"));
        for i in 0..LINES {
            writeln!(
                out,
                "line {i:06} - the quick brown fox jumps over the lazy dog"
            )
            .expect("write line");
        }
        out.flush().expect("flush text");
        drop(out);

        sandbox.settle();
        sandbox
    }

    fn listing(&self) -> PathBuf {
        self.dir.path().join("listing")
    }

    fn text(&self) -> PathBuf {
        self.dir.path().join("preview.txt")
    }

    /// Reads every file once before measuring.
    /// Lets the virus scanner and the first cold reads finish beforehand.
    fn settle(&self) {
        let mut buf = Vec::new();
        for entry in fs::read_dir(self.listing())
            .expect("read listing")
            .flatten()
        {
            if let Ok(mut file) = fs::File::open(entry.path()) {
                buf.clear();
                let _ = file.read_to_end(&mut buf);
            }
        }
        let _ = fs::read(self.text());
    }
}

fn disk(c: &mut Criterion) {
    let sandbox = Sandbox::build();
    let listing = sandbox.listing();

    let mut group = c.benchmark_group("disk");
    // Fewer equal sized samples to keep the run short
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(ENTRIES as u64));

    let natural = LoadFixture::new(&listing, SortBy::Natural);
    group.bench_function("browse/10k", |b| b.iter(|| natural.browse()));
    group.bench_function("load-natural/10k", |b| b.iter(|| natural.run()));
    let modified = LoadFixture::new(&listing, SortBy::Modified);
    group.bench_function("load-modified/10k", |b| b.iter(|| modified.run()));

    group.throughput(Throughput::Elements(50));
    let text: &Path = &sandbox.text();
    let deep = PreviewFixture::new(text, 50, 80, 10_000);
    group.bench_function("preview-scrolled/10k", |b| b.iter(|| deep.run()));

    group.finish();
}

criterion_group! {
    name = benches;
    config = config();
    targets = disk
}
criterion_main!(benches);
