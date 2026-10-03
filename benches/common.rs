//! Shared Criterion settings for the time and disk benches of runa.

#![allow(dead_code)]

use std::time::Duration;

use criterion::{BatchSize, Criterion};

/// Number of inputs prepared for each timed batch.
/// Keeps the memory usage small and the same on every run.
pub const BATCH: BatchSize = BatchSize::NumIterations(8);

/// Entry counts used by the sized benches.
pub const SIZES: [usize; 2] = [1_000, 10_000];

/// Returns the Criterion config used by all timing benches.
pub fn config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(4))
        // Changes below 5% are normal machine noise
        .noise_threshold(0.05)
        .significance_level(0.01)
        .confidence_level(0.99)
}

/// Returns a short label for a bench id, like 1k for 1000.
pub fn label(n: usize) -> String {
    if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}
