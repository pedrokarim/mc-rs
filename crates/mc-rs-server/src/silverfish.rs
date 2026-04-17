//! Silverfish — mob qui se cache dans infested blocks.

#[derive(Debug, Clone)]
pub struct Silverfish {
    pub call_cooldown: u32,
    pub target_entity: Option<u64>,
}

/// Call nearby silverfish from infested blocks (80-100 ticks).
pub const CALL_COOLDOWN: u32 = 100;
/// Infested blocks.
pub fn infested_blocks() -> &'static [(u16, u16)] {
    &[
        (97, 1), // Infested stone
        (97, 2), // Infested cobblestone
        (97, 3), // Infested stone bricks
        (97, 4), // Infested mossy stone bricks
        (97, 5), // Infested cracked stone bricks
        (97, 6), // Infested chiseled stone bricks
    ]
}

impl Silverfish {
    pub fn new() -> Self {
        Self {
            call_cooldown: 0,
            target_entity: None,
        }
    }

    pub fn tick(&mut self) {
        if self.call_cooldown > 0 {
            self.call_cooldown -= 1;
        }
    }

    pub fn damaged_by_player(&mut self) -> bool {
        if self.call_cooldown > 0 {
            return false;
        }
        self.call_cooldown = CALL_COOLDOWN;
        true
    }

    /// Call range (21×11×21 cuboid vanilla).
    pub fn call_range() -> (i32, i32, i32) {
        (10, 5, 10)
    }
}

impl Default for Silverfish {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_damage_triggers_call() {
        let mut s = Silverfish::new();
        assert!(s.damaged_by_player());
    }

    #[test]
    fn second_damage_silent() {
        let mut s = Silverfish::new();
        s.damaged_by_player();
        assert!(!s.damaged_by_player());
    }
}
