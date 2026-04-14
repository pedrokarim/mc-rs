//! Item cooldown system — ender pearl, chorus fruit.

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ItemCooldowns {
    /// Item id → ticks remaining.
    pub cooldowns: HashMap<u16, u32>,
}

/// Default cooldowns (ticks).
pub const ENDER_PEARL_COOLDOWN: u32 = 20;
pub const CHORUS_FRUIT_COOLDOWN: u32 = 20;
pub const TRIDENT_RIPTIDE_COOLDOWN: u32 = 20;
pub const WIND_CHARGE_COOLDOWN: u32 = 10;

impl ItemCooldowns {
    pub fn set(&mut self, item_id: u16, ticks: u32) {
        self.cooldowns.insert(item_id, ticks);
    }

    pub fn has_cooldown(&self, item_id: u16) -> bool {
        self.cooldowns.get(&item_id).copied().unwrap_or(0) > 0
    }

    pub fn tick(&mut self) {
        self.cooldowns.retain(|_, t| {
            if *t > 0 {
                *t -= 1;
                true
            } else {
                false
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_ticks_down() {
        let mut c = ItemCooldowns::default();
        c.set(368, 5); // ender pearl
        c.tick();
        assert_eq!(c.cooldowns.get(&368), Some(&4));
    }
}
