//! The internal preview reader. The `bat` path is not benchmarked - it is a
//! process spawn runa does not control.
//!
//! Both cases render the same 50 lines, so the difference between them is
//! purely the cost of skipping to the scroll offset.

use std::hint::black_box;
use std::path::Path;

use criterion::Criterion;
use runa::bench_api::PreviewFixture;

use crate::common::{Scratch, bytes, memory_header};

const LINES: usize = 20_000;
const VISIBLE: usize = 50;
const WIDTH: usize = 80;
const DEEP: usize = 10_000;

pub fn register(c: &mut Criterion) {
    let scratch = Scratch::new();
    let file = scratch.text_file("preview.txt", LINES);

    memory_report(&file);

    let mut group = c.benchmark_group("preview");

    let head = PreviewFixture::new(&file, VISIBLE, WIDTH);
    group.bench_function(format!("head/{VISIBLE}lines"), |b| {
        b.iter(|| black_box(head.run()))
    });

    let deep = PreviewFixture::new(&file, VISIBLE, WIDTH).with_scroll(DEEP);
    group.bench_function(format!("scrolled-10k/{VISIBLE}lines"), |b| {
        b.iter(|| black_box(deep.run()))
    });

    group.finish();
}

fn memory_report(file: &Path) {
    memory_header("preview");
    println!("  {VISIBLE} lines at {WIDTH} columns");
    println!();
    println!(
        "  {:>10}  {:>8}  {:>12}  {:>12}  {:>12}",
        "scroll", "lines", "text", "inline", "total"
    );
    for scroll in [0, DEEP] {
        let f = PreviewFixture::new(file, VISIBLE, WIDTH)
            .with_scroll(scroll)
            .footprint();
        println!(
            "  {:>10}  {:>8}  {:>12}  {:>12}  {:>12}",
            scroll,
            f.lines,
            bytes(f.text_bytes),
            bytes(f.inline_bytes),
            bytes(f.total()),
        );
    }
    println!();
}
