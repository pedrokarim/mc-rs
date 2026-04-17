//! Tool types + efficiency.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    Pickaxe,
    Axe,
    Shovel,
    Hoe,
    Shears,
    Sword,
    Bucket,
    FishingRod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolTier {
    Wood = 0,
    Stone = 1,
    Iron = 2,
    Gold = 3, // Fastest but least durable
    Diamond = 4,
    Netherite = 5,
}

impl ToolTier {
    /// Mining speed multiplier.
    pub fn speed(&self) -> f32 {
        match self {
            Self::Wood => 2.0,
            Self::Stone => 4.0,
            Self::Iron => 6.0,
            Self::Gold => 12.0,
            Self::Diamond => 8.0,
            Self::Netherite => 9.0,
        }
    }

    /// Durability.
    pub fn durability(&self) -> u16 {
        match self {
            Self::Wood => 59,
            Self::Stone => 131,
            Self::Iron => 250,
            Self::Gold => 32,
            Self::Diamond => 1561,
            Self::Netherite => 2031,
        }
    }

    /// Damage (attack).
    pub fn damage(&self) -> f32 {
        match self {
            Self::Wood => 4.0,
            Self::Stone => 5.0,
            Self::Iron => 6.0,
            Self::Gold => 4.0,
            Self::Diamond => 7.0,
            Self::Netherite => 8.0,
        }
    }

    /// Enchantability.
    pub fn enchantability(&self) -> u8 {
        match self {
            Self::Wood => 15,
            Self::Stone => 5,
            Self::Iron => 14,
            Self::Gold => 22,
            Self::Diamond => 10,
            Self::Netherite => 15,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gold_fastest() {
        assert!(ToolTier::Gold.speed() > ToolTier::Diamond.speed());
    }

    #[test]
    fn netherite_most_durable() {
        assert!(ToolTier::Netherite.durability() > ToolTier::Diamond.durability());
    }
}
