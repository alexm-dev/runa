//! runa benchmark suite.
//!
//! `cargo bench --features internal`
//!
//! Select an area with Criterion's filter, e.g. `-- "^cache"` or `-- sort/natural`.
//! Compare a change with `-- --save-baseline before` then `-- --baseline before`,
//! taking both in one session - idle medians run 3-5% faster than loaded ones.
//!
//! To keep a readable log (bench-logs/ is gitignored):

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
