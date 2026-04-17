//! Crossbow — charge, multi-shot, piercing, fireworks + arrows.

#[derive(Debug, Clone)]
pub struct Crossbow {
    pub charged: bool,
    pub charge_ticks: u32,
    pub ammo: Vec<u16>, // 1-3 projectiles (with multishot)
    pub durability: u16,
}

/// Base charge time (25 ticks).
pub const BASE_CHARGE_TIME: u32 = 25;
/// Quick charge enchant reduces each level by 5 ticks.
pub const QUICK_CHARGE_REDUCTION: u32 = 5;
/// Max durability.
pub const MAX_DURABILITY: u16 = 465;

impl Crossbow {
    pub fn new() -> Self {
        Self {
            charged: false,
            charge_ticks: 0,
            ammo: Vec::new(),
            durability: MAX_DURABILITY,
        }
    }

    pub fn start_charging(&mut self, quick_charge_level: u8, ammo_type: u16) {
        self.charge_ticks =
            BASE_CHARGE_TIME.saturating_sub(QUICK_CHARGE_REDUCTION * quick_charge_level as u32);
        self.ammo = vec![ammo_type];
    }

    pub fn tick(&mut self) {
        if !self.charged && self.charge_ticks > 0 {
            self.charge_ticks -= 1;
            if self.charge_ticks == 0 {
                self.charged = true;
            }
        }
    }

    pub fn apply_multishot(&mut self, ammo_type: u16) {
        if self.ammo.len() < 3 {
            self.ammo.push(ammo_type);
            self.ammo.push(ammo_type);
        }
    }

    pub fn fire(&mut self) -> Vec<u16> {
        if !self.charged {
            return vec![];
        }
        let ammo = std::mem::take(&mut self.ammo);
        self.charged = false;
        self.durability = self.durability.saturating_sub(1);
        ammo
    }

    /// Piercing enchant = arrow goes through enemies.
    pub fn piercing_passes(level: u8) -> u8 {
        level + 1
    }
}

impl Default for Crossbow {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_takes_time() {
        let mut c = Crossbow::new();
        c.start_charging(0, 1);
        assert!(!c.charged);
    }

    #[test]
    fn quick_charge_reduces_time() {
        let mut c = Crossbow::new();
        c.start_charging(5, 1);
        assert_eq!(c.charge_ticks, 0);
    }

    #[test]
    fn multishot_adds_two() {
        let mut c = Crossbow::new();
        c.start_charging(0, 1);
        c.apply_multishot(1);
        assert_eq!(c.ammo.len(), 3);
    }
}
