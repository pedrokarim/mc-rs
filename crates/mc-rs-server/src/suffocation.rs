//! Suffocation damage from blocks.

/// Damage per tick when head inside solid block (1 HP every 2s).
pub const SUFFOCATION_DAMAGE: f32 = 1.0;
pub const SUFFOCATION_INTERVAL: u32 = 10; // ticks

/// Only non-transparent solid blocks suffocate.
pub fn causes_suffocation(block_id: u16) -> bool {
    !matches!(block_id,
        0  // air
        | 20 | 95 // glass
        | 92 // cake (partial)
        | 85 | 107 // fence
        | 44 // slab (half)
        | 88 // soul sand
        | 78 // snow layer
        | 97 // fully occlude? infested
    ) && block_id != 0 // air check again for safety
}

/// Water/lava don't suffocate.
pub fn water_suffocates() -> bool { false }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn air_no_suffocation() {
        assert!(!causes_suffocation(0));
    }

    #[test]
    fn stone_suffocates() {
        assert!(causes_suffocation(1));
    }
}
