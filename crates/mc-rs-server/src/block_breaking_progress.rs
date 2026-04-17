//! Block breaking progress (digging animation).

#[derive(Debug, Clone)]
pub struct DigProgress {
    pub start_tick: u64,
    pub block_pos: (i32, i32, i32),
    pub block_id: u16,
    pub total_ticks: u32,
    pub last_animation_stage: i8, // -1 to 9
}

impl DigProgress {
    pub fn new(now: u64, pos: (i32, i32, i32), block_id: u16, total_ticks: u32) -> Self {
        Self {
            start_tick: now,
            block_pos: pos,
            block_id,
            total_ticks,
            last_animation_stage: -1,
        }
    }

    pub fn progress(&self, now: u64) -> f32 {
        if self.total_ticks == 0 {
            return 1.0;
        }
        ((now - self.start_tick) as f32 / self.total_ticks as f32).clamp(0.0, 1.0)
    }

    pub fn animation_stage(&self, now: u64) -> i8 {
        let prog = self.progress(now);
        ((prog * 10.0) as i8).min(9)
    }

    pub fn is_complete(&self, now: u64) -> bool {
        self.progress(now) >= 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_at_total() {
        let d = DigProgress::new(0, (0, 0, 0), 1, 100);
        assert!(d.is_complete(100));
    }
}
