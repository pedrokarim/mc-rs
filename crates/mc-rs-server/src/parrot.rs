//! Parrot — apprivoisement avec seeds, imite sons de mobs.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParrotVariant {
    Red,
    Blue,
    Green,
    Cyan,
    Gray,
}

#[derive(Debug, Clone)]
pub struct Parrot {
    pub variant: ParrotVariant,
    pub tamed: bool,
    pub owner: Option<u64>,
    pub sitting: bool,
    pub shoulder_position: Option<u64>, // Player on whose shoulder
}

/// Tame chance per seed (1/3).
pub const TAME_CHANCE: f32 = 1.0 / 3.0;
/// Cookie is LETHAL to parrots.
pub const COOKIE_DAMAGE: f32 = 1000.0;
/// Seeds items qui tame.
pub fn seeds_items() -> &'static [&'static str] {
    &[
        "minecraft:wheat_seeds",
        "minecraft:beetroot_seeds",
        "minecraft:melon_seeds",
        "minecraft:pumpkin_seeds",
        "minecraft:torchflower_seeds",
        "minecraft:pitcher_pod",
    ]
}

impl Parrot {
    pub fn new(variant: ParrotVariant) -> Self {
        Self {
            variant,
            tamed: false,
            owner: None,
            sitting: false,
            shoulder_position: None,
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
            true
        } else {
            false
        }
    }

    /// Death on cookie (poisoned).
    pub fn is_poisoned_by(item: &str) -> bool {
        item == "minecraft:cookie"
    }

    pub fn mount_shoulder(&mut self, player_id: u64) -> bool {
        if !self.tamed {
            return false;
        }
        self.shoulder_position = Some(player_id);
        true
    }

    pub fn dismount_shoulder(&mut self) {
        self.shoulder_position = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_is_poison() {
        assert!(Parrot::is_poisoned_by("minecraft:cookie"));
        assert!(!Parrot::is_poisoned_by("minecraft:wheat_seeds"));
    }

    #[test]
    fn shoulder_requires_tame() {
        let mut p = Parrot::new(ParrotVariant::Red);
        assert!(!p.mount_shoulder(1));
    }
}
