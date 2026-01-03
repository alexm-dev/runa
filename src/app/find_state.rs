use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::{
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

use crate::core::find::FindResult;

#[derive(Default)]
pub struct FindState {
    cache: Vec<FindResult>,
    request_id: u64,
    debounce: Option<Instant>,
    last_query: String,
    cancel: Option<Arc<AtomicBool>>,
}

impl FindState {
    // Getters / Accessors

    pub fn results(&self) -> &[FindResult] {
        &self.cache
    }

    pub fn request_id(&self) -> u64 {
        self.request_id
    }

    pub fn debounce(&self) -> &Option<Instant> {
        &self.debounce
    }

    pub fn last_query(&self) -> &str {
        &self.last_query
    }

    pub fn cancel_current(&mut self) {
        if let Some(token) = self.cancel.take() {
            token.store(true, Ordering::Relaxed);
        }
    }

    pub fn set_results(&mut self, results: Vec<FindResult>) {
        self.cache = results;
    }

    pub fn set_cancel(&mut self, token: Arc<AtomicBool>) {
        self.cancel = Some(token);
    }

    pub fn clear_results(&mut self) {
        self.cache.clear();
    }

    pub fn next_request_id(&mut self) -> u64 {
        self.request_id = self.request_id.wrapping_add(1);
        self.request_id
    }

    pub fn set_debounce(&mut self, delay: Duration) {
        self.debounce = Some(Instant::now() + delay);
    }

    pub fn take_query(&mut self, current_query: &str) -> Option<String> {
        let Some(until) = self.debounce else {
            return None;
        };
        if Instant::now() < until {
            return None;
        }

        self.debounce = None;
        if current_query == self.last_query {
            self.last_query.clear();
            return None;
        }

        self.last_query.clear();
        self.last_query.push_str(current_query);
        Some(current_query.to_string())
    }

    pub fn reset(&mut self) {
        self.cancel_current();
        self.cache.clear();
        self.debounce = None;
        self.last_query.clear();
    }
}
