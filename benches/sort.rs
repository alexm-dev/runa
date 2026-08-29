//! Sorting and filtering a directory listing.

use std::hint::black_box;
use std::path::Path;
use std::time::{Duration, Instant};

use criterion::{Criterion, Throughput};
use runa::bench_api::{FilterFixture, SortFixture};

use crate::common::{SIZES, Scratch, size};

type Build = fn(&Path, usize) -> SortFixture;

const MODES: [(&str, Build); 3] = [
    ("by-name", SortFixture::by_name),
    ("by-natural", SortFixture::by_natural),
    ("by-extension", SortFixture::by_extension),
];

pub fn register(c: &mut Criterion) {
    let scratch = Scratch::new();

    let mut group = c.benchmark_group("sort");
    for (label, build) in MODES {
        for n in SIZES {
            group.throughput(Throughput::Elements(n as u64));
            group.sample_size(if n >= 10_000 { 30 } else { 100 });
            let mut fixture = build(scratch.path(), n);
            group.bench_function(format!("{label}/{}", size(n)), |b| {
                b.iter_custom(|iters| {
                    let mut total = Duration::ZERO;
                    for _ in 0..iters {
                        fixture.reset();
                        let start = Instant::now();
                        black_box(fixture.run());
                        total += start.elapsed();
                    }
                    total
                })
            });
        }
    }
    group.finish();

    let mut group = c.benchmark_group("filter");
    for n in SIZES {
        group.throughput(Throughput::Elements(n as u64));
        group.sample_size(if n >= 10_000 { 30 } else { 100 });
        let mut fixture = FilterFixture::new(n);
        group.bench_function(size(n), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    fixture.reset();
                    let start = Instant::now();
                    black_box(fixture.run());
                    total += start.elapsed();
                }
                total
            })
        });
    }
    group.finish();
}
