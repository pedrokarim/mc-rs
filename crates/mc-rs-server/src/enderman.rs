//! Enderman — teleport sous l'eau/dans le feu, angry on eye contact.

#[derive(Debug, Clone)]
pub struct Enderman {
    pub held_block: Option<u16>,
    pub anger_ticks: u32,
    pub target_entity: Option<u64>,
    pub teleport_cooldown: u32,
}

/// Can pick up ces blocks (vanilla).
pub fn carriable_blocks() -> &'static [u16] {
    &[
        12,  // sand
        13,  // gravel
        2,   // grass block
        3,   // dirt
        37,  // dandelion
        38,  // poppy
        39,  // brown mushroom
        40,  // red mushroom
        110, // mycelium
        81,  // cactus
        82,  // clay
        86,  // pumpkin
        103, // melon
        172, // hardened clay
    ]
}

/// Max teleport range (64 blocs).
pub const TELEPORT_RANGE: f64 = 64.0;
/// Teleport cooldown when attacked.
pub const TELEPORT_COOLDOWN: u32 = 10;
/// Anger duration on eye contact.
pub const ANGER_DURATION: u32 = 400;

impl Enderman {
    pub fn new() -> Self {
        Self {
            held_block: None,
            anger_ticks: 0,
            target_entity: None,
            teleport_cooldown: 0,
        }
    }

    pub fn can_carry(block_id: u16) -> bool {
        carriable_blocks().contains(&block_id)
    }

    pub fn provoke(&mut self, target: u64) {
        self.anger_ticks = ANGER_DURATION;
        self.target_entity = Some(target);
    }

    pub fn tick(&mut self) {
        if self.anger_ticks > 0 {
            self.anger_ticks -= 1;
            if self.anger_ticks == 0 {
                self.target_entity = None;
            }
        }
        if self.teleport_cooldown > 0 {
            self.teleport_cooldown -= 1;
        }
    }

    pub fn is_hostile(&self) -> bool {
        self.anger_ticks > 0
    }

    /// Teleport on damage (vanilla 64% chance).
    pub fn teleport_chance_on_damage() -> f32 {
        0.64
    }

    /// Damaged by water/rain — tries teleport.
    pub fn damaged_by_water(&self) -> bool {
        true
    }
}

impl Default for Enderman {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sand_carriable() {
        assert!(Enderman::can_carry(12));
    }

    #[test]
    fn stone_not_carriable() {
        assert!(!Enderman::can_carry(1));
    }

    #[test]
    fn provoke_sets_anger() {
        let mut e = Enderman::new();
        e.provoke(42);
        assert!(e.is_hostile());
    }
}
