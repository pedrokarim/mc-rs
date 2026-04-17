//! Minecart variants — passenger/chest/hopper/tnt/furnace/spawner.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinecartKind {
    Empty,   // Passenger
    Chest,   // 27 slots storage
    Hopper,  // Auto-pickup items
    Tnt,     // Explodes on activator rail
    Furnace, // Boosts via fuel
    CommandBlock,
    Spawner,
}

#[derive(Debug, Clone)]
pub struct Minecart {
    pub kind: MinecartKind,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub velocity_z: f64,
    pub fuel_ticks: u32,
    pub tnt_primed: bool,
    pub tnt_fuse: u32,
    pub inventory: Vec<Option<(u16, u16)>>,
    pub passenger: Option<u64>,
}

/// Chest minecart slots.
pub const CHEST_SLOTS: usize = 27;
/// Hopper minecart slots.
pub const HOPPER_SLOTS: usize = 5;
/// Max speed (8.0 blocks/sec empty).
pub const MAX_SPEED: f64 = 0.4;
/// Furnace boost duration per coal (180 seconds).
pub const FURNACE_FUEL_PER_COAL: u32 = 3600;

impl Minecart {
    pub fn new(kind: MinecartKind) -> Self {
        let slots = match kind {
            MinecartKind::Chest => CHEST_SLOTS,
            MinecartKind::Hopper => HOPPER_SLOTS,
            _ => 0,
        };
        Self {
            kind,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
            fuel_ticks: 0,
            tnt_primed: false,
            tnt_fuse: 0,
            inventory: vec![None; slots],
            passenger: None,
        }
    }

    pub fn tick(&mut self) -> bool {
        if self.fuel_ticks > 0 {
            self.fuel_ticks -= 1;
        }
        if self.tnt_primed {
            self.tnt_fuse = self.tnt_fuse.saturating_sub(1);
            if self.tnt_fuse == 0 {
                return true; // explode
            }
        }
        false
    }

    pub fn prime_tnt(&mut self) {
        if self.kind == MinecartKind::Tnt {
            self.tnt_primed = true;
            self.tnt_fuse = 80;
        }
    }

    pub fn refuel_furnace(&mut self) {
        if self.kind == MinecartKind::Furnace {
            self.fuel_ticks += FURNACE_FUEL_PER_COAL;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chest_has_27_slots() {
        let m = Minecart::new(MinecartKind::Chest);
        assert_eq!(m.inventory.len(), CHEST_SLOTS);
    }

    #[test]
    fn tnt_primes() {
        let mut m = Minecart::new(MinecartKind::Tnt);
        m.prime_tnt();
        assert!(m.tnt_primed);
    }
}
