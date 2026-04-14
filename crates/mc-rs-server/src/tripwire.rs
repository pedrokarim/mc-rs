//! Tripwire — hook + wire. Triggered by entity.

#[derive(Debug, Clone)]
pub struct TripwireHook {
    pub facing: u8,
    pub attached: bool,  // Has wire connected
    pub powered: bool,
}

#[derive(Debug, Clone)]
pub struct Tripwire {
    pub connected: bool, // Has hooks on both ends
    pub powered: bool,
    pub disarmed: bool,  // Cut with shears without triggering
}

impl TripwireHook {
    pub fn new(facing: u8) -> Self {
        Self { facing, attached: false, powered: false }
    }

    pub fn trigger(&mut self) {
        if self.attached {
            self.powered = true;
        }
    }

    pub fn release(&mut self) {
        self.powered = false;
    }
}

impl Tripwire {
    pub fn new() -> Self {
        Self { connected: false, powered: false, disarmed: false }
    }

    pub fn entity_steps_on(&mut self) -> bool {
        if self.disarmed {
            return false;
        }
        self.powered = true;
        self.connected
    }
}

impl Default for Tripwire {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disarmed_no_trigger() {
        let mut t = Tripwire::new();
        t.connected = true;
        t.disarmed = true;
        assert!(!t.entity_steps_on());
    }
}
