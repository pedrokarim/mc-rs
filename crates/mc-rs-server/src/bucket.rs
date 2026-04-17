//! Bucket — port PMMP `src/item/LiquidBucket.php` + `Bucket.php`.

use crate::item_registry::network_id;
use mc_rs_proto::packets::player::ItemStack;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketContent {
    Empty,
    Water,
    Lava,
    Milk,
    Cod,
    Salmon,
    TropicalFish,
    Pufferfish,
    Axolotl,
    Tadpole,
    PowderedSnow,
}

impl BucketContent {
    pub fn item_name(&self) -> &'static str {
        match self {
            Self::Empty => "minecraft:bucket",
            Self::Water => "minecraft:water_bucket",
            Self::Lava => "minecraft:lava_bucket",
            Self::Milk => "minecraft:milk_bucket",
            Self::Cod => "minecraft:cod_bucket",
            Self::Salmon => "minecraft:salmon_bucket",
            Self::TropicalFish => "minecraft:tropical_fish_bucket",
            Self::Pufferfish => "minecraft:pufferfish_bucket",
            Self::Axolotl => "minecraft:axolotl_bucket",
            Self::Tadpole => "minecraft:tadpole_bucket",
            Self::PowderedSnow => "minecraft:powder_snow_bucket",
        }
    }

    /// Bucket's item ID if registry knows it.
    pub fn to_item(&self) -> Option<ItemStack> {
        network_id(self.item_name()).map(|id| ItemStack::new(id, 1, 0))
    }
}

/// Using milk bucket removes all effects. PMMP `MilkBucket::onConsume`.
pub fn milk_removes_effects() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bucket_name() {
        assert_eq!(BucketContent::Empty.item_name(), "minecraft:bucket");
    }

    #[test]
    fn milk_effect() {
        assert!(milk_removes_effects());
    }
}
