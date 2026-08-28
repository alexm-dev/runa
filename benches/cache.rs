//! `DirCache` lookup, insertion and eviction.
//!
//! `cargo bench --features internal --bench cache`
//!
//! The cache holds 30 directories. Comparing `insert/10dirs` against
//! `insert-at-cap/30dirs` isolates the cost of the eviction scan.

mod common;

use std::hint::black_box;

use criterion::Criterion;
use runa::bench_api::CacheFixture;

use common::{bytes, memory_header};

fn main() {
    memory_report();

    let mut c = Criterion::default().configure_from_args();
    bench_cache(&mut c);
    c.final_summary();
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

fn bench_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache");

    let mut warm = CacheFixture::new(10, 1_000);
    group.bench_function("hit/10dirs", |b| b.iter(|| black_box(warm.run_hit())));
    group.bench_function("miss/10dirs", |b| b.iter(|| black_box(warm.run_miss())));
    group.bench_function("insert/10dirs", |b| b.iter(|| warm.run_insert()));

    let mut full = CacheFixture::new(30, 1_000);
    group.bench_function("hit/30dirs", |b| b.iter(|| black_box(full.run_hit())));
    group.bench_function("insert-at-cap/30dirs", |b| b.iter(|| full.run_insert()));
    group.bench_function("invalidate/30dirs", |b| b.iter(|| full.run_invalidate()));

    group.finish();
}
