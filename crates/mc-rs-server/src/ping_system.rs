//! Ping / latency tracking.

/// Exponential moving average for ping smoothing.
pub const PING_SMOOTHING: f32 = 0.1;

#[derive(Debug, Clone)]
pub struct PingTracker {
    pub current_ping_ms: u32,
    pub min_ping_ms: u32,
    pub max_ping_ms: u32,
    pub samples: u32,
}

impl PingTracker {
    pub fn new() -> Self {
        Self {
            current_ping_ms: 0,
            min_ping_ms: u32::MAX,
            max_ping_ms: 0,
            samples: 0,
        }
    }

    pub fn record(&mut self, ping_ms: u32) {
        if self.samples == 0 {
            self.current_ping_ms = ping_ms;
        } else {
            let new = (self.current_ping_ms as f32 * (1.0 - PING_SMOOTHING)
                + ping_ms as f32 * PING_SMOOTHING) as u32;
            self.current_ping_ms = new;
        }
        self.min_ping_ms = self.min_ping_ms.min(ping_ms);
        self.max_ping_ms = self.max_ping_ms.max(ping_ms);
        self.samples += 1;
    }

    pub fn rating(&self) -> PingRating {
        match self.current_ping_ms {
            0..=50 => PingRating::Excellent,
            51..=100 => PingRating::Good,
            101..=200 => PingRating::Fair,
            201..=500 => PingRating::Poor,
            _ => PingRating::Unusable,
        }
    }
}

impl Default for PingTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PingRating {
    Excellent,
    Good,
    Fair,
    Poor,
    Unusable,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_ping_poor() {
        let mut t = PingTracker::new();
        t.record(300);
        matches!(t.rating(), PingRating::Poor);
    }
}
