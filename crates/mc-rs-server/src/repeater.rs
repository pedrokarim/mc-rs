//! Redstone repeater — delay 1-4, lock state.

#[derive(Debug, Clone)]
pub struct Repeater {
    pub facing: u8,
    pub delay: u8,          // 1-4 ticks
    pub powered: bool,
    pub locked: bool,
    pub pending_tick: u32,
}

/// Max delay (4 ticks).
pub const MAX_DELAY: u8 = 4;

impl Repeater {
    pub fn new(facing: u8) -> Self {
        Self {
            facing,
            delay: 1,
            powered: false,
            locked: false,
            pending_tick: 0,
        }
    }

    pub fn increment_delay(&mut self) {
        self.delay = (self.delay % MAX_DELAY) + 1;
    }

    pub fn signal_input(&mut self, input: bool) {
        if self.locked {
            return;
        }
        if input != self.powered {
            self.pending_tick = (self.delay * 2) as u32;
        }
    }

    pub fn tick(&mut self) {
        if self.pending_tick > 0 {
            self.pending_tick -= 1;
            if self.pending_tick == 0 {
                self.powered = !self.powered;
            }
        }
    }

    pub fn lock(&mut self, locked: bool) {
        self.locked = locked;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_cycles() {
        let mut r = Repeater::new(0);
        for expected in 2..=4u8 {
            r.increment_delay();
            assert_eq!(r.delay, expected);
        }
        r.increment_delay();
        assert_eq!(r.delay, 1);
    }

    #[test]
    fn locked_prevents_change() {
        let mut r = Repeater::new(0);
        r.lock(true);
        r.signal_input(true);
        assert_eq!(r.pending_tick, 0);
    }
}
