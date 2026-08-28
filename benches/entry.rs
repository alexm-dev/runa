//! `FileEntry` construction cost and memory footprint.
//!
//! `cargo bench --features internal --bench entry`

mod common;

use std::hint::black_box;

use criterion::{Criterion, Throughput};
use runa::bench_api::EntryFootprint;

use common::{SIZES, bytes, memory_header};

fn main() {
    memory_report();

    let mut c = Criterion::default().configure_from_args();
    bench_build(&mut c);
    c.final_summary();
}

fn memory_report() {
    memory_header("FileEntry");

    println!(
        "  struct size: {} bytes, {} allocations per entry ({} with a symlink)",
        EntryFootprint::struct_size(),
        EntryFootprint::allocations_per_entry(false),
        EntryFootprint::allocations_per_entry(true),
    );
    println!();
    println!(
        "  {:>8}  {:>12}  {:>12}  {:>12}  {:>10}",
        "entries", "inline", "heap", "total", "per entry"
    );
    for n in SIZES {
        let f = EntryFootprint::measure(n);
        println!(
            "  {:>8}  {:>12}  {:>12}  {:>12}  {:>9.1}B",
            f.items,
            bytes(f.inline),
            bytes(f.heap),
            bytes(f.total()),
            f.per_item(),
        );
    }
    println!();
}

fn bench_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("entry");
    for n in SIZES {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(format!("build/{n}"), |b| {
            b.iter(|| black_box(EntryFootprint::measure(black_box(n))))
        });
    }
    group.finish();
}
