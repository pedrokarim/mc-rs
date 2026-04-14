//! Cat — tamed avec cod/salmon, morning gifts, scare creepers/phantoms.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatVariant {
    Tabby,
    Black,
    Red,
    Siamese,
    BritishShorthair,
    Calico,
    Persian,
    Ragdoll,
    White,
    Jellie,
    AllBlack,
}

#[derive(Debug, Clone)]
pub struct Cat {
    pub variant: CatVariant,
    pub age: i32,
    pub tamed: bool,
    pub owner: Option<u64>,
    pub sitting: bool,
    pub collar_color: u8,
    pub morning_gift_pending: bool,
}

/// Cat scare range for creepers/phantoms (6 blocs).
pub const SCARE_RANGE: f64 = 6.0;
/// Taming items = raw cod/salmon.
pub fn taming_items() -> &'static [&'static str] {
    &["minecraft:raw_cod", "minecraft:raw_salmon"]
}
/// Tame chance per item (1/3).
pub const TAME_CHANCE: f32 = 1.0 / 3.0;

impl Cat {
    pub fn new(variant: CatVariant) -> Self {
        Self {
            variant,
            age: 0,
            tamed: false,
            owner: None,
            sitting: false,
            collar_color: 14, // red default
            morning_gift_pending: false,
        }
    }

    pub fn try_tame(&mut self, owner: u64) -> bool {
        if self.tamed {
            return false;
        }
        use rand::Rng;
        let mut rng = rand::thread_rng();
        if rng.gen::<f32>() < TAME_CHANCE {
            self.tamed = true;
            self.owner = Some(owner);
            self.sitting = true;
            true
        } else {
            false
        }
    }

    /// Schedule morning gift on player wake up.
    pub fn schedule_morning_gift(&mut self) {
        if self.tamed {
            self.morning_gift_pending = true;
        }
    }

    /// Scare away certain mobs (creeper, phantom).
    pub fn scares(entity_kind: &str) -> bool {
        matches!(entity_kind, "creeper" | "phantom")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creeper_scared() {
        assert!(Cat::scares("creeper"));
    }

    #[test]
    fn zombie_not_scared() {
        assert!(!Cat::scares("zombie"));
    }
}
