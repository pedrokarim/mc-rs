//! Redstone wire — power propagation.

/// Max signal strength.
pub const MAX_POWER: u8 = 15;

#[derive(Debug, Clone)]
pub struct RedstoneWire {
    pub power: u8,
    pub connects_north: bool,
    pub connects_south: bool,
    pub connects_east: bool,
    pub connects_west: bool,
    pub connects_up: bool,
}

impl RedstoneWire {
    pub fn new() -> Self {
        Self {
            power: 0,
            connects_north: false,
            connects_south: false,
            connects_east: false,
            connects_west: false,
            connects_up: false,
        }
    }

    /// Power decays by 1 per block.
    pub fn propagate(&self, neighbor_target_power: u8) -> u8 {
        if self.power == 0 {
            return neighbor_target_power;
        }
        self.power.saturating_sub(1).max(neighbor_target_power)
    }
}

impl Default for RedstoneWire {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_decays() {
        let mut w = RedstoneWire::new();
        w.power = 15;
        assert_eq!(w.propagate(0), 14);
    }
}
