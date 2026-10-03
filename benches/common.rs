//! Shared Criterion settings for the `time` and `disk` benches.

#![allow(dead_code)]

use std::time::Duration;

use criterion::{BatchSize, Criterion};

/// Inputs per timed batch. Fixed, so memory stays small (~10 MiB at 10k
/// entries) however long Criterion measures.
pub const BATCH: BatchSize = BatchSize::NumIterations(8);

pub const SIZES: [usize; 2] = [1_000, 10_000];

pub fn config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(4))
        // Machine noise is a few percent; smaller changes count as noise.
        .noise_threshold(0.05)
        .significance_level(0.01)
        .confidence_level(0.99)
}

/// Compact size for benchmark ids: 1_000 -> "1k".
pub fn label(n: usize) -> String {
    if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}
