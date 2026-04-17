//! Piglin Brute — neutral but very aggressive variant of piglin.

#[derive(Debug, Clone)]
pub struct PiglinBrute {
    pub attack_cooldown: u32,
    pub is_aggressive: bool,
    pub target_entity: Option<u64>,
}

/// Melee damage.
pub const DAMAGE: f32 = 13.0;
/// HP.
pub const HP: f32 = 50.0;
/// Attack cooldown.
pub const ATTACK_COOLDOWN: u32 = 20;

impl PiglinBrute {
    pub fn new() -> Self {
        Self {
            attack_cooldown: 0,
            is_aggressive: false,
            target_entity: None,
        }
    }

    /// Piglin brutes ignore gold.
    pub fn distracted_by_gold() -> bool {
        false
    }

    /// Cannot be bartered with.
    pub fn can_barter() -> bool {
        false
    }

    /// Never zombifies in Overworld.
    pub fn zombifies() -> bool {
        false
    }
}

impl Default for PiglinBrute {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn no_zombification() {
        assert!(!super::PiglinBrute::zombifies());
    }
}
