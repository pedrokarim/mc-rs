//! Keepalive ping/pong — detect dead connections.

/// Interval between keepalive pings (200 ticks = 10s).
pub const KEEPALIVE_INTERVAL: u32 = 200;
/// Timeout for disconnection (60s without pong = 1200 ticks).
pub const TIMEOUT_TICKS: u32 = 1200;

#[derive(Debug, Clone)]
pub struct KeepaliveState {
    pub last_ping_sent_tick: u64,
    pub last_pong_received_tick: u64,
    pub outstanding_ping_id: Option<i64>,
    pub rtt_ms: u32,
}

impl KeepaliveState {
    pub fn new(now: u64) -> Self {
        Self {
            last_ping_sent_tick: now,
            last_pong_received_tick: now,
            outstanding_ping_id: None,
            rtt_ms: 0,
        }
    }

    pub fn should_send_ping(&self, now: u64) -> bool {
        now - self.last_ping_sent_tick >= KEEPALIVE_INTERVAL as u64
    }

    pub fn is_timed_out(&self, now: u64) -> bool {
        now - self.last_pong_received_tick >= TIMEOUT_TICKS as u64
    }

    pub fn on_pong(&mut self, now: u64, rtt: u32) {
        self.last_pong_received_tick = now;
        self.outstanding_ping_id = None;
        self.rtt_ms = rtt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_after_1200_ticks() {
        let ka = KeepaliveState::new(0);
        assert!(!ka.is_timed_out(100));
        assert!(ka.is_timed_out(TIMEOUT_TICKS as u64 + 1));
    }
}
