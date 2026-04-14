//! Saddles — for various mounts.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaddleableMount {
    Pig,
    Horse,
    Donkey,
    Mule,
    Llama,        // Carpet, not saddle
    Strider,
    Camel,
    SkeletonHorse,
    ZombieHorse,
}

impl SaddleableMount {
    /// Returns the saddle item (or None if doesn't use a saddle).
    pub fn saddle_item(&self) -> Option<&'static str> {
        match self {
            Self::Llama => None, // Uses carpet
            _ => Some("minecraft:saddle"),
        }
    }

    pub fn can_be_ridden_without_saddle(&self) -> bool {
        false
    }
}

/// Saddle item rarity (uncommon, only via fishing/loot/villager).
pub const RARITY: &str = "uncommon";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pig_uses_saddle() {
        assert_eq!(SaddleableMount::Pig.saddle_item(), Some("minecraft:saddle"));
    }

    #[test]
    fn llama_no_saddle() {
        assert!(SaddleableMount::Llama.saddle_item().is_none());
    }
}
