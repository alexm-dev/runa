//! Directory listing - what a directory change costs.
//!
//! I/O bound, so these numbers are comparable run-to-run on one machine only.
//! The gap between `browse` and `browse+filter+sort` is what `Formatter` adds.

use std::hint::black_box;

use criterion::{Criterion, Throughput};
use runa::bench_api::{ListingFixture, Sort};

use crate::common::{Scratch, bytes, memory_header};

const ENTRIES: usize = 2_000;

pub fn register(c: &mut Criterion) {
    let scratch = Scratch::new();
    let dir = scratch.listing_dir(ENTRIES);
    let fixture = ListingFixture::new(&dir, Sort::Natural);

    memory_report(&fixture);

    let mut group = c.benchmark_group("listing");
    group.sample_size(50);
    group.throughput(Throughput::Elements(ENTRIES as u64));

    group.bench_function(format!("browse/{ENTRIES}"), |b| {
        b.iter(|| black_box(fixture.run_browse()))
    });
    group.bench_function(format!("browse+filter+sort/{ENTRIES}"), |b| {
        b.iter(|| black_box(fixture.run_full()))
    });

    group.finish();
}

fn memory_report(fixture: &ListingFixture) {
    memory_header("directory listing");

    let f = fixture.footprint();
    println!("  entries:   {}", f.items);
    println!("  inline:    {}", bytes(f.inline));
    println!("  heap:      {}", bytes(f.heap));
    println!("  total:     {}", bytes(f.total()));
    println!("  per entry: {:.1} B", f.per_item());
    println!();
    println!("  Held for as long as the directory is open. Multiply by tabs plus");
    println!("  the parent and preview panes for the resident total.");
    println!();
}
