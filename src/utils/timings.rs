//! Timings module for AppState, handling the throttling
//! and or debounce for relevant actions.
//!
//! Throttler to wrap each timing into a check first before debouncing a request.

use std::time::{Duration, Instant};

/// Timings constants for various actions.
pub(crate) struct Timings;

impl Timings {
    pub(crate) const PREVIEW_REQUEST_MS: u64 = 30;
    pub(crate) const PREVIEW_DEBOUNCE_MS: u64 = 35;
    pub(crate) const NAV_THROTTLE_MS: u64 = 15;
    pub(crate) const FILE_INFO_DEBOUNCE_MS: u64 = 60;
    pub(crate) const CONFIG_RELOAD_MS: u64 = 1000;
    pub(crate) const UI_RELOAD_MS: u64 = 200;
    pub(crate) const FS_WATCH_DEBOUNCE_MS: u64 = 150;
    pub(crate) const WORKER_INDICATOR_MS: u64 = 200;

    /// How long short confirmations stay visible.
    pub(crate) const MESSAGE_SHORT: Duration = Duration::from_secs(2);
    /// Default message and errors duration.
    pub(crate) const MESSAGE: Duration = Duration::from_secs(3);
    /// How long important messages stay visible.
    pub(crate) const MESSAGE_LONG: Duration = Duration::from_secs(5);
}

/// Throttler to handle timings and debounce for relevant actions.
/// Tracks latest timings and whether a request is queued for later.
#[derive(Default, Debug, Clone)]
pub(crate) struct Throttler {
    last_timing: Option<Instant>,
    queued: bool,
}

impl Throttler {
    pub(crate) fn new() -> Self {
        Self {
            last_timing: None,
            queued: false,
        }
    }

    /// Check if the throttler can trigger an action based on the last timing
    pub(crate) fn can_trigger(&self, ms: u64) -> bool {
        let now = Instant::now();
        match self.last_timing {
            Some(prev) => now.duration_since(prev) >= Duration::from_millis(ms),
            None => true,
        }
    }

    /// Leading Edge: Try to trigger an action, returning whether it was allowed or not.
    /// true if a call is allowed, false if it is queued for later.
    /// A refused call is remembered and can be taken later with `take_queued`.
    pub(crate) fn try_trigger(&mut self, ms: u64) -> bool {
        let allowed = self.can_trigger(ms);
        if !allowed {
            self.queued = true;
        }
        allowed
    }

    /// Trailing Edge: Check if a queued action can be taken based on the last timing.
    /// Take a queued action if it is allowed based on the last timing.
    pub(crate) fn take_queued(&mut self, ms: u64) -> bool {
        if self.queued && self.can_trigger(ms) {
            self.queued = false;
            return true;
        }
        false
    }

    /// Update the last timings to now and reset queued state.
    pub(crate) fn touch(&mut self) {
        self.last_timing = Some(Instant::now());
        self.queued = false;
    }
}
