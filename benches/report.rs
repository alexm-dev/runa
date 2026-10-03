//! Allocation and memory report for runa.
//!
//! Counts the allocations and retained memory of each operation instead of timing it.
//! The output is the same on every run of the same code.
//! Only reads the src directory and LICENSE-APACHE.
//!
//! Commands:
//!     cargo bench --bench report    print the report

use std::alloc::System;
use std::path::Path;
use std::time::Duration;

use runa::bench_api::{
    self, CacheFixture, EntryFixture, FilterFixture, IdleFixture, LoadFixture, PreviewFixture,
    SortBy, SortFixture,
};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (size, allocs, allocs_symlink) = bench_api::entry_layout();
    println!(
        "\nFileEntry: {size} bytes inline, {allocs} heap allocations ({allocs_symlink} for a symlink)\n"
    );
    println!(
        "{:<26} {:>8} {:>8} {:>11} {:>11} {:>10}",
        "operation", "items", "allocs", "allocated", "retained", "per item"
    );

    for n in [1_000, 10_000] {
        let fixture = EntryFixture::new(n);
        let names = fixture.input();
        measure(&format!("entry new {}k", n / 1_000), n, || {
            fixture.run(names)
        });
    }

    for (name, by) in [
        ("name", SortBy::Name),
        ("natural", SortBy::Natural),
        ("extension", SortBy::Extension),
        ("size", SortBy::Size),
        ("modified", SortBy::Modified),
    ] {
        let fixture = SortFixture::new(by, 10_000);
        let mut input = fixture.input();
        measure(&format!("sort {name} 10k"), 10_000, || {
            fixture.run(&mut input)
        });
    }

    let fixture = FilterFixture::new(10_000);
    let mut input = fixture.input();
    measure("filter 10k", 10_000, || fixture.run(&mut input));

    let mut cache = CacheFixture::new(30, 1_000);
    measure("cache hit", 1, || cache.hit());
    measure("cache insert (at cap)", 1, || cache.insert());

    let src = root.join("src");
    let load = LoadFixture::new(&src, SortBy::Natural);
    let entries = load.browse();
    measure("load src/ natural", entries, || load.run());

    let license = root.join("LICENSE-APACHE");
    let preview = PreviewFixture::new(&license, 50, 80, 0);
    measure("preview 50 lines", 50, || preview.run());

    println!();
    match IdleFixture::new(&src) {
        Ok(mut idle) => {
            let region = Region::new(GLOBAL);
            let redraws = idle.run(Duration::from_secs(1));
            let stats = region.change();
            println!(
                "idle 1 s: {redraws} redraws, {} allocations   (both should be 0)",
                stats.allocations + stats.reallocations
            );
        }
        Err(e) => println!("idle: could not open src/: {e}"),
    }
    println!();
}

/// Runs the operation once and prints its allocation counts.
/// Retained is the memory the result still holds.
fn measure<T>(name: &str, items: usize, op: impl FnOnce() -> T) {
    let region = Region::new(GLOBAL);
    let out = op();
    let s = region.change();
    drop(out);

    let allocs = s.allocations + s.reallocations;
    let allocated = s.bytes_allocated as isize + s.bytes_reallocated.max(0);
    let retained = s.bytes_allocated as isize + s.bytes_reallocated - s.bytes_deallocated as isize;
    let per_item = if items == 0 {
        0.0
    } else {
        retained as f64 / items as f64
    };
    println!(
        "{:<26} {:>8} {:>8} {:>11} {:>11} {:>9.1}B",
        name,
        items,
        allocs,
        bytes(allocated),
        bytes(retained),
        per_item
    );
}

fn bytes(n: isize) -> String {
    const KIB: f64 = 1024.0;
    let v = n as f64;
    if v.abs() < KIB {
        format!("{n} B")
    } else if v.abs() < KIB * KIB {
        format!("{:.1} KiB", v / KIB)
    } else {
        format!("{:.2} MiB", v / (KIB * KIB))
    }
}
