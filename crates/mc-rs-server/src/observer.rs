//! Observer — emits redstone pulse on neighbor update in front.

#[derive(Debug, Clone)]
pub struct Observer {
    pub facing: u8,
    pub pulse_ticks: u32,
}

/// Observer pulse duration (2 ticks / 1 tick in PMMP).
pub const PULSE_DURATION: u32 = 2;

impl Observer {
    pub fn new(facing: u8) -> Self {
        Self { facing, pulse_ticks: 0 }
    }

    /// Called when block in front changes.
    pub fn on_neighbor_update(&mut self) {
        self.pulse_ticks = PULSE_DURATION;
    }

    pub fn tick(&mut self) {
        if self.pulse_ticks > 0 {
            self.pulse_ticks -= 1;
        }
    }

    pub fn is_emitting(&self) -> bool {
        self.pulse_ticks > 0
    }

    pub fn signal_strength(&self) -> u8 {
        if self.is_emitting() { 15 } else { 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_after_update() {
        let mut o = Observer::new(0);
        o.on_neighbor_update();
        assert!(o.is_emitting());
    }

    #[test]
    fn pulse_expires() {
        let mut o = Observer::new(0);
        o.on_neighbor_update();
        for _ in 0..=PULSE_DURATION {
            o.tick();
        }
        assert!(!o.is_emitting());
    }
}
