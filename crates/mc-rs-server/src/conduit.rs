//! Conduit — port PMMP `src/block/Conduit.php` (underwater beacon).
//!
//! Un Conduit entouré d'au moins 16 blocs de prismarine/sea_lantern valide
//! donne le status effect `Conduit Power` (water breathing + night vision +
//! haste mining + attack hostile marine mobs) aux joueurs dans la range.

#[derive(Debug, Clone)]
pub struct ConduitState {
    pub position: [i32; 3],
    pub ring_block_count: u32,
    pub is_active: bool,
    pub has_target: bool,
}

impl ConduitState {
    pub fn new(position: [i32; 3]) -> Self {
        Self {
            position,
            ring_block_count: 0,
            is_active: false,
            has_target: false,
        }
    }

    /// Le conduit s'active s'il a >= 16 blocs de prismarine dans son ring 3x3x3.
    pub fn compute_activation(&mut self, ring_blocks: u32) {
        self.ring_block_count = ring_blocks;
        self.is_active = ring_blocks >= 16;
    }

    /// Rayon d'effet (vanilla : 16 + 16 par tranche de 7 blocs ring).
    /// Max effect range = 96 blocks (avec 42 blocs).
    pub fn effect_range(&self) -> u32 {
        if !self.is_active {
            return 0;
        }
        let raw = (self.ring_block_count / 7) * 16 + 16;
        raw.min(96) // vanilla max range
    }

    /// Attack range hostile marine mobs (guardian, drowned).
    pub fn attack_range(&self) -> u32 {
        if self.ring_block_count >= 42 {
            8
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activates_at_16_blocks() {
        let mut c = ConduitState::new([0, 64, 0]);
        c.compute_activation(15);
        assert!(!c.is_active);
        c.compute_activation(16);
        assert!(c.is_active);
    }

    #[test]
    fn max_range_at_42_blocks() {
        let mut c = ConduitState::new([0, 64, 0]);
        c.compute_activation(42);
        assert_eq!(c.effect_range(), 96);
        assert_eq!(c.attack_range(), 8);
    }
}
