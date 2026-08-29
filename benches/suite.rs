//! runa benchmark suite.
//!
//! ```text
//! cargo bench
//! cargo bench -- "^cache"                       one area, Criterion filter
//! cargo bench -- --save-baseline before         then: -- --baseline before
//! ```
//!
//! Take before/after in one session - idle medians run 3-5% faster than loaded
//! ones. The report lands in `target/criterion/report/index.html`.
//!
//! Each area prints a computed memory table before its timings. Those figures
//! are deterministic; the timings are wall-clock, with a measured noise floor
//! under 7.3%. Treat a change under 5% as noise and one over 10% as real.

mod common;

mod cache;
mod entry;
mod listing;
mod preview;
mod sort;

use criterion::Criterion;

fn main() {
    let mut c = Criterion::default().configure_from_args();

    entry::register(&mut c);
    cache::register(&mut c);
    sort::register(&mut c);
    listing::register(&mut c);
    preview::register(&mut c);

    c.final_summary();
}
