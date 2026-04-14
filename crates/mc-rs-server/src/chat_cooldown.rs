//! Chat cooldown / rate limit.

/// Min ticks between messages (anti-spam).
pub const MIN_MESSAGE_INTERVAL_TICKS: u32 = 10;
/// Max messages per second.
pub const MAX_PER_SEC: u32 = 20;
/// Mute duration for spam (60s).
pub const MUTE_DURATION: u32 = 1200;

#[derive(Debug, Clone, Default)]
pub struct ChatCooldown {
    pub last_message_tick: u64,
    pub muted_until: Option<u64>,
    pub recent_message_count: u32,
}

impl ChatCooldown {
    pub fn can_send(&self, now: u64) -> bool {
        if let Some(until) = self.muted_until {
            if now < until {
                return false;
            }
        }
        now - self.last_message_tick >= MIN_MESSAGE_INTERVAL_TICKS as u64
    }

    pub fn record(&mut self, now: u64) {
        self.last_message_tick = now;
        self.recent_message_count += 1;
    }

    pub fn mute(&mut self, now: u64, duration: u32) {
        self.muted_until = Some(now + duration as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn muted_cannot_send() {
        let mut c = ChatCooldown::default();
        c.mute(0, 100);
        assert!(!c.can_send(50));
    }
}
