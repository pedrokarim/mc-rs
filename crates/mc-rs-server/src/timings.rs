//! Timings / performance tracking.

use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct TimingEntry {
    pub total_ns: u128,
    pub call_count: u64,
    pub max_ns: u128,
    pub min_ns: u128,
}

impl TimingEntry {
    pub fn new() -> Self {
        Self {
            total_ns: 0,
            call_count: 0,
            max_ns: 0,
            min_ns: u128::MAX,
        }
    }

    pub fn record(&mut self, duration: Duration) {
        let ns = duration.as_nanos();
        self.total_ns += ns;
        self.call_count += 1;
        self.max_ns = self.max_ns.max(ns);
        self.min_ns = self.min_ns.min(ns);
    }

    pub fn average_ns(&self) -> u128 {
        if self.call_count == 0 {
            return 0;
        }
        self.total_ns / self.call_count as u128
    }
}

impl Default for TimingEntry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct TimingsManager {
    pub entries: HashMap<String, TimingEntry>,
    pub enabled: bool,
}

impl TimingsManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self, name: &str) -> Option<TimingGuard<'_>> {
        if !self.enabled {
            return None;
        }
        Some(TimingGuard {
            manager: self,
            name: name.to_string(),
            start: Instant::now(),
        })
    }

    pub fn report(&mut self, name: &str, duration: Duration) {
        self.entries
            .entry(name.to_string())
            .or_default()
            .record(duration);
    }
}

pub struct TimingGuard<'a> {
    manager: &'a mut TimingsManager,
    name: String,
    start: Instant,
}

impl<'a> Drop for TimingGuard<'a> {
    fn drop(&mut self) {
        let d = self.start.elapsed();
        self.manager.report(&self.name, d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_duration() {
        let mut e = TimingEntry::new();
        e.record(Duration::from_micros(100));
        assert_eq!(e.call_count, 1);
        assert!(e.average_ns() > 0);
    }
}
