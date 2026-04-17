//! Mob variants — colors / biomes / species. Axolotl, Frog, Wolf, Cat, etc.

use crate::biomes_registry::BiomeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxolotlVariant {
    Lucy = 0,
    Wild = 1,
    Gold = 2,
    Cyan = 3,
    Blue = 4, // 1/1200 chance
}

impl AxolotlVariant {
    pub fn random() -> Self {
        use rand::Rng;
        let r = rand::thread_rng().gen_range(0..1200);
        if r == 0 {
            Self::Blue
        } else {
            match r % 4 {
                0 => Self::Lucy,
                1 => Self::Wild,
                2 => Self::Gold,
                _ => Self::Cyan,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrogVariant {
    Temperate, // default green
    Warm,      // white
    Cold,      // green-brown
}

impl FrogVariant {
    pub fn from_biome(biome: BiomeKind) -> Self {
        if biome.is_nether() {
            return Self::Temperate;
        }
        match biome {
            BiomeKind::Jungle
            | BiomeKind::SparseJungle
            | BiomeKind::BambooJungle
            | BiomeKind::MangroveSwamp
            | BiomeKind::Desert => Self::Warm,
            BiomeKind::SnowyPlains
            | BiomeKind::SnowyTaiga
            | BiomeKind::IceSpikes
            | BiomeKind::FrozenPeaks
            | BiomeKind::SnowySlopes
            | BiomeKind::Grove
            | BiomeKind::JaggedPeaks => Self::Cold,
            _ => Self::Temperate,
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WolfVariant {
    Pale, // default
    Woods,
    Ashen,
    Black,
    Chestnut,
    Rusty,
    Snowy,
    Spotted,
    Striped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParrotVariant {
    Red,
    Blue,
    Green,
    Cyan,
    Silver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaVariant {
    Creamy,
    White,
    Brown,
    Gray,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TropicalFishColor {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frog_warm_in_desert() {
        assert_eq!(
            FrogVariant::from_biome(BiomeKind::Desert),
            FrogVariant::Warm
        );
    }

    #[test]
    fn frog_cold_in_snowy() {
        assert_eq!(
            FrogVariant::from_biome(BiomeKind::SnowyPlains),
            FrogVariant::Cold
        );
    }

    #[test]
    fn frog_temperate_default() {
        assert_eq!(
            FrogVariant::from_biome(BiomeKind::Plains),
            FrogVariant::Temperate
        );
    }
}
