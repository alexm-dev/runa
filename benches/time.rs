//! Timings.
//!
//! ```text
//! cargo bench --bench time -- --save-baseline before    record
//! cargo bench --bench time -- --baseline before         compare
//! cargo bench --bench time -- sort                      one area
//! ```
//!
//! In-memory input, except `load` and `preview`, which read `src/` and
//! `LICENSE-APACHE`. Compare baselines taken in the same session.

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
    // Cloning the input dominates the wall time.
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

/// The real load path on `src/`. Large sizes are covered by `entry`,
/// `filter` and `sort`, and by the `disk` bench.
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
