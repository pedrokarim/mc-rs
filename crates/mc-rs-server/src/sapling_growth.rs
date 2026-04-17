//! Sapling growth — oak, birch, spruce, jungle, acacia, dark oak, mangrove, cherry.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeSpecies {
    Oak,
    Birch,
    Spruce,
    Jungle,
    Acacia,
    DarkOak,
    Mangrove,
    Cherry,
    Azalea,
    BambooShoot,
}

impl TreeSpecies {
    /// DarkOak needs 2x2 of saplings to grow.
    pub fn requires_2x2(&self) -> bool {
        matches!(self, Self::DarkOak)
    }

    /// Jungle can grow 2x2 into giant tree.
    pub fn can_grow_giant(&self) -> bool {
        matches!(self, Self::Jungle | Self::Spruce)
    }

    /// Needs sky access.
    pub fn needs_sky() -> bool {
        true
    }

    /// Growth stages (saplings have 2 phases).
    pub fn max_stage() -> u8 {
        1
    }

    /// Growth chance per random tick (~5%).
    pub fn growth_chance() -> f32 {
        0.05
    }

    /// Bone meal guarantees growth or next stage.
    pub fn bone_meal_chance() -> f32 {
        0.45
    }

    pub fn log_block(&self) -> &'static str {
        match self {
            Self::Oak => "minecraft:oak_log",
            Self::Birch => "minecraft:birch_log",
            Self::Spruce => "minecraft:spruce_log",
            Self::Jungle => "minecraft:jungle_log",
            Self::Acacia => "minecraft:acacia_log",
            Self::DarkOak => "minecraft:dark_oak_log",
            Self::Mangrove => "minecraft:mangrove_log",
            Self::Cherry => "minecraft:cherry_log",
            Self::Azalea => "minecraft:oak_log",
            Self::BambooShoot => "minecraft:bamboo",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_oak_requires_2x2() {
        assert!(TreeSpecies::DarkOak.requires_2x2());
        assert!(!TreeSpecies::Oak.requires_2x2());
    }
}
