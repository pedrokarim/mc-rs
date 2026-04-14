//! Packet loss tracker.

#[derive(Debug, Clone)]
pub struct PacketLossTracker {
    pub total_sent: u64,
    pub total_acked: u64,
    pub total_lost: u64,
    pub recent_window: Vec<bool>, // true = received, false = lost
    pub window_size: usize,
}

impl PacketLossTracker {
    pub fn new(window: usize) -> Self {
        Self {
            total_sent: 0,
            total_acked: 0,
            total_lost: 0,
            recent_window: Vec::with_capacity(window),
            window_size: window,
        }
    }

    pub fn record_sent(&mut self) {
        self.total_sent += 1;
    }

    pub fn record_ack(&mut self) {
        self.total_acked += 1;
        self.push_window(true);
    }

    pub fn record_loss(&mut self) {
        self.total_lost += 1;
        self.push_window(false);
    }

    fn push_window(&mut self, received: bool) {
        if self.recent_window.len() >= self.window_size {
            self.recent_window.remove(0);
        }
        self.recent_window.push(received);
    }

    pub fn recent_loss_rate(&self) -> f32 {
        if self.recent_window.is_empty() {
            return 0.0;
        }
        let lost = self.recent_window.iter().filter(|&&r| !r).count();
        lost as f32 / self.recent_window.len() as f32
    }
}

impl Default for PacketLossTracker {
    fn default() -> Self { Self::new(100) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_zero_loss() {
        let t = PacketLossTracker::default();
        assert_eq!(t.recent_loss_rate(), 0.0);
    }

    #[test]
    fn all_lost_100() {
        let mut t = PacketLossTracker::new(5);
        for _ in 0..5 {
            t.record_loss();
        }
        assert_eq!(t.recent_loss_rate(), 1.0);
    }
}
