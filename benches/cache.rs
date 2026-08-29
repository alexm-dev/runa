//! `DirCache` lookup, insertion and eviction.
//!
//! The cache holds 30 directories. Comparing `insert/10dirs` against
//! `insert-at-cap/30dirs` isolates the cost of the eviction scan.

use std::hint::black_box;

use criterion::Criterion;
use runa::bench_api::CacheFixture;

use crate::common::{bytes, memory_header};

pub fn register(c: &mut Criterion) {
    memory_report();

    let mut group = c.benchmark_group("cache");

    let mut warm = CacheFixture::new(10, 1_000);
    group.bench_function("hit-below-cap", |b| b.iter(|| black_box(warm.run_hit())));
    group.bench_function("miss", |b| b.iter(|| black_box(warm.run_miss())));
    group.bench_function("insert-below-cap", |b| b.iter(|| warm.run_insert()));

    let mut full = CacheFixture::new(30, 1_000);
    group.bench_function("hit-at-cap", |b| b.iter(|| black_box(full.run_hit())));
    group.bench_function("insert-at-cap", |b| b.iter(|| full.run_insert()));
    group.bench_function("invalidate", |b| b.iter(|| full.run_invalidate()));

    group.finish();
}

fn memory_report() {
    memory_header("DirCache");
    println!(
        "  {:>24}  {:>10}  {:>12}  {:>12}  {:>12}",
        "shape", "keys", "entries", "sort column", "total"
    );
    for (dirs, per_dir) in [(10, 1_000), (30, 1_000), (30, 10_000)] {
        let f = CacheFixture::measure_footprint(dirs, per_dir);
        println!(
            "  {:>24}  {:>10}  {:>12}  {:>12}  {:>12}",
            format!("{dirs} dirs x {per_dir} entries"),
            bytes(f.key_bytes),
            bytes(f.shared_entry_bytes),
            bytes(f.shared_column_bytes),
            bytes(f.total()),
        );
    }
    println!();
}
