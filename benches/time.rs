//! Timing benchmarks for runa.
//!
//! Measures entry creation, sorting, filtering, the directory cache, directory loads and previews.
//! Runs on generated input in memory. Only the load and preview benches read files,
//! the src directory and LICENSE-APACHE.
//!
//! Save a baseline before a change and compare against it after the change.
//! Both runs should happen in the same session.
//!
//! Commands:
//!     cargo bench --bench time                              run all timing benches
//!     cargo bench --bench time -- sort                      run only the sort benches
//!     cargo bench --bench time -- --save-baseline before    save a baseline named before
//!     cargo bench --bench time -- --baseline before         compare against the baseline
//!
//! A full check of a change:
//!     cargo bench --bench report
//!     cargo bench --bench time -- --save-baseline before
//!     (make the change)
//!     cargo bench --bench report
//!     cargo bench --bench time -- --baseline before

mod common;

use std::hint::black_box;
use std::path::Path;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use runa::bench_api::{
    CacheFixture, EntryFixture, FilterFixture, LoadFixture, PreviewFixture, SortBy, SortFixture,
};

use common::{BATCH, SIZES, config, label};

fn entry(c: &mut Criterion) {
    let mut group = c.benchmark_group("entry");
    for n in SIZES {
        let fixture = EntryFixture::new(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(format!("new/{}", label(n)), |b| {
            b.iter_batched(|| fixture.input(), |names| fixture.run(names), BATCH)
        });
    }
    group.finish();
}

fn sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort");
    for (name, by) in [
        ("name", SortBy::Name),
        ("natural", SortBy::Natural),
        ("extension", SortBy::Extension),
        ("size", SortBy::Size),
        ("modified", SortBy::Modified),
    ] {
        for n in SIZES {
            let fixture = SortFixture::new(by, n);
            group.throughput(Throughput::Elements(n as u64));
            group.bench_function(format!("{name}/{}", label(n)), |b| {
                b.iter_batched_ref(|| fixture.input(), |e| fixture.run(e), BATCH)
            });
        }
    }
    group.finish();
}

fn filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter");
    // Extra time for cloning the input
    group.measurement_time(std::time::Duration::from_secs(8));
    for n in SIZES {
        let fixture = FilterFixture::new(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(label(n), |b| {
            b.iter_batched_ref(|| fixture.input(), |e| fixture.run(e), BATCH)
        });
    }
    group.finish();
}

fn cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache");

    let mut below = CacheFixture::new(10, 1_000);
    group.bench_function("hit", |b| b.iter(|| black_box(below.hit())));
    group.bench_function("miss", |b| b.iter(|| black_box(below.miss())));
    group.bench_function("insert-below-cap", |b| b.iter(|| below.insert()));

    let mut full = CacheFixture::new(30, 1_000);
    group.bench_function("insert-at-cap", |b| b.iter(|| full.insert()));
    group.bench_function("invalidate", |b| b.iter(|| full.invalidate()));

    group.finish();
}

/// Measures loading the src directory of the repo.
/// Larger directories are measured by the entry, filter and sort benches and the disk bench.
fn load(c: &mut Criterion) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    let mut group = c.benchmark_group("load");
    let natural = LoadFixture::new(&src, SortBy::Natural);
    group.bench_function("src/natural", |b| b.iter(|| natural.run()));
    let modified = LoadFixture::new(&src, SortBy::Modified);
    group.bench_function("src/modified", |b| b.iter(|| modified.run()));
    group.finish();
}

fn preview(c: &mut Criterion) {
    let license = Path::new(env!("CARGO_MANIFEST_DIR")).join("LICENSE-APACHE");

    let mut group = c.benchmark_group("preview");
    let top = PreviewFixture::new(&license, 50, 80, 0);
    group.bench_function("top", |b| b.iter(|| top.run()));
    let scrolled = PreviewFixture::new(&license, 50, 80, 150);
    group.bench_function("scrolled-150", |b| b.iter(|| scrolled.run()));
    group.finish();
}

criterion_group! {
    name = benches;
    config = config();
    targets = entry, sort, filter, cache, load, preview
}
criterion_main!(benches);
