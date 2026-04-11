use std::time::{Duration, Instant};

pub(super) struct Timings;

impl Timings {
    pub(super) const PREVIEW_REQUEST_MS: u64 = 30;
    pub(super) const PREVIEW_DEBOUNCE_MS: u64 = 35;
    pub(super) const NAV_THROTTLE_MS: u64 = 15;
    pub(super) const FILE_INFO_DEBOUNCE_MS: u64 = 60;
}

#[derive(Default, Debug, Clone)]
pub(super) struct Throttler {
    last_timing: Option<Instant>,
}

impl Throttler {
    pub(super) fn new() -> Self {
        Self { last_timing: None }
    }

    pub(super) fn can_trigger(&self, ms: u64) -> bool {
        let now = Instant::now();
        match self.last_timing {
            Some(prev) => now.duration_since(prev) >= Duration::from_millis(ms),
            None => true,
        }
    }

    pub(super) fn touch(&mut self) {
        self.last_timing = Some(Instant::now());
    }
}
