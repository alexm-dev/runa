//! Sorting and filtering a directory listing.
//!
//! Both mutate in place, so `reset()` runs outside the timer. With it inside,
//! `filter/10000` measured 2.75 ms instead of 31 us - a deep clone, not a
//! filter. `Size` and `Modified` are excluded; they stat entries that do not
//! exist on disk.

use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::{Criterion, Throughput};
use runa::bench_api::{FilterFixture, Sort, SortFixture};

use crate::common::{SIZES, Scratch};

const SORTS: [Sort; 3] = [Sort::Name, Sort::Natural, Sort::Extension];

pub fn register(c: &mut Criterion) {
    let scratch = Scratch::new();

    let mut group = c.benchmark_group("sort");
    for sort in SORTS {
        for n in SIZES {
            group.throughput(Throughput::Elements(n as u64));
            group.sample_size(if n >= 10_000 { 30 } else { 100 });
            let mut fixture = SortFixture::new(scratch.path(), n, sort);
            group.bench_function(format!("{}/{n}", sort.label()), |b| {
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
        group.bench_function(format!("{n}"), |b| {
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
