//! Timings module for AppState, handling the throttling
//! and or debounce for relevant actions.
//!
//! Throttler to wrap each timing into a check first before debouncing a request.

use std::time::{Duration, Instant};

pub(crate) struct Timings;

impl Timings {
    pub(crate) const PREVIEW_REQUEST_MS: u64 = 30;
    pub(crate) const PREVIEW_DEBOUNCE_MS: u64 = 35;
    pub(crate) const NAV_THROTTLE_MS: u64 = 15;
    pub(crate) const FILE_INFO_DEBOUNCE_MS: u64 = 60;
    pub(crate) const CONFIG_RELOAD_MS: u64 = 1000;
}

#[derive(Default, Debug, Clone)]
pub(crate) struct Throttler {
    last_timing: Option<Instant>,
}

impl Throttler {
    pub(crate) fn new() -> Self {
        Self { last_timing: None }
    }

    pub(crate) fn can_trigger(&self, ms: u64) -> bool {
        let now = Instant::now();
        match self.last_timing {
            Some(prev) => now.duration_since(prev) >= Duration::from_millis(ms),
            None => true,
        }
    }

    pub(crate) fn touch(&mut self) {
        self.last_timing = Some(Instant::now());
    }
}
