//! Brewing / Potions — port PMMP `src/data/bedrock/PotionTypeIdMap.php` + `Potion.php`.
//!
//! Liste des types de potion vanilla + recettes de brewing stand.

use crate::effects::EffectKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotionType {
    Water,
    Mundane,
    LongMundane,
    Thick,
    Awkward,
    NightVision,
    LongNightVision,
    Invisibility,
    LongInvisibility,
    Leaping,
    LongLeaping,
    StrongLeaping,
    FireResistance,
    LongFireResistance,
    Swiftness,
    LongSwiftness,
    StrongSwiftness,
    Slowness,
    LongSlowness,
    StrongSlowness,
    WaterBreathing,
    LongWaterBreathing,
    Healing,
    StrongHealing,
    Harming,
    StrongHarming,
    Poison,
    LongPoison,
    StrongPoison,
    Regeneration,
    LongRegeneration,
    StrongRegeneration,
    Strength,
    LongStrength,
    StrongStrength,
    Weakness,
    LongWeakness,
    TurtleMaster,
    LongTurtleMaster,
    StrongTurtleMaster,
    SlowFalling,
    LongSlowFalling,
}

impl PotionType {
    /// Effet principal + durée (ticks) + amplifier.
    pub fn primary_effect(&self) -> Option<(EffectKind, i32, u8)> {
        use EffectKind::*;
        match self {
            Self::Water | Self::Mundane | Self::LongMundane | Self::Thick | Self::Awkward => None,
            Self::NightVision => Some((NightVision, 3600, 0)),
            Self::LongNightVision => Some((NightVision, 9600, 0)),
            Self::Invisibility => Some((Invisibility, 3600, 0)),
            Self::LongInvisibility => Some((Invisibility, 9600, 0)),
            Self::Leaping => Some((JumpBoost, 3600, 0)),
            Self::LongLeaping => Some((JumpBoost, 9600, 0)),
            Self::StrongLeaping => Some((JumpBoost, 1800, 1)),
            Self::FireResistance => Some((FireResistance, 3600, 0)),
            Self::LongFireResistance => Some((FireResistance, 9600, 0)),
            Self::Swiftness => Some((Speed, 3600, 0)),
            Self::LongSwiftness => Some((Speed, 9600, 0)),
            Self::StrongSwiftness => Some((Speed, 1800, 1)),
            Self::Slowness => Some((Slowness, 1800, 0)),
            Self::LongSlowness => Some((Slowness, 4800, 0)),
            Self::StrongSlowness => Some((Slowness, 400, 3)),
            Self::WaterBreathing => Some((WaterBreathing, 3600, 0)),
            Self::LongWaterBreathing => Some((WaterBreathing, 9600, 0)),
            Self::Healing => Some((InstantHealth, 1, 0)),
            Self::StrongHealing => Some((InstantHealth, 1, 1)),
            Self::Harming => Some((InstantDamage, 1, 0)),
            Self::StrongHarming => Some((InstantDamage, 1, 1)),
            Self::Poison => Some((Poison, 900, 0)),
            Self::LongPoison => Some((Poison, 1800, 0)),
            Self::StrongPoison => Some((Poison, 432, 1)),
            Self::Regeneration => Some((Regeneration, 900, 0)),
            Self::LongRegeneration => Some((Regeneration, 1800, 0)),
            Self::StrongRegeneration => Some((Regeneration, 450, 1)),
            Self::Strength => Some((Strength, 3600, 0)),
            Self::LongStrength => Some((Strength, 9600, 0)),
            Self::StrongStrength => Some((Strength, 1800, 1)),
            Self::Weakness => Some((Weakness, 1800, 0)),
            Self::LongWeakness => Some((Weakness, 4800, 0)),
            Self::TurtleMaster => Some((Slowness, 400, 3)),
            Self::LongTurtleMaster => Some((Slowness, 800, 3)),
            Self::StrongTurtleMaster => Some((Slowness, 400, 5)),
            Self::SlowFalling => Some((SlowFalling, 1800, 0)),
            Self::LongSlowFalling => Some((SlowFalling, 4800, 0)),
        }
    }
}

/// Recette de brewing : (input potion, ingredient item name, output potion).
#[derive(Debug, Clone, Copy)]
pub struct BrewingRecipe {
    pub input: PotionType,
    pub ingredient_item: &'static str,
    pub output: PotionType,
}

/// Recettes vanilla PMMP `BrewingRecipeHelper`.
pub fn vanilla_brewing_recipes() -> Vec<BrewingRecipe> {
    vec![
        BrewingRecipe { input: PotionType::Water, ingredient_item: "minecraft:nether_wart", output: PotionType::Awkward },
        BrewingRecipe { input: PotionType::Awkward, ingredient_item: "minecraft:golden_carrot", output: PotionType::NightVision },
        BrewingRecipe { input: PotionType::Awkward, ingredient_item: "minecraft:fermented_spider_eye", output: PotionType::Invisibility },
        BrewingRecipe { input: PotionType::Awkward, ingredient_item: "minecraft:rabbit_foot", output: PotionType::Leaping },
        BrewingRecipe { input: PotionType::Awkward, ingredient_item: "minecraft:magma_cream", output: PotionType::FireResistance },
        BrewingRecipe { input: PotionType::Awkward, ingredient_item: "minecraft:sugar", output: PotionType::Swiftness },
        BrewingRecipe { input: PotionType::Awkward, ingredient_item: "minecraft:pufferfish", output: PotionType::WaterBreathing },
        BrewingRecipe { input: PotionType::Awkward, ingredient_item: "minecraft:glistering_melon_slice", output: PotionType::Healing },
        BrewingRecipe { input: PotionType::Awkward, ingredient_item: "minecraft:spider_eye", output: PotionType::Poison },
        BrewingRecipe { input: PotionType::Awkward, ingredient_item: "minecraft:ghast_tear", output: PotionType::Regeneration },
        BrewingRecipe { input: PotionType::Awkward, ingredient_item: "minecraft:blaze_powder", output: PotionType::Strength },
        BrewingRecipe { input: PotionType::Awkward, ingredient_item: "minecraft:phantom_membrane", output: PotionType::SlowFalling },
        // Redstone : extend duration (Long versions).
        BrewingRecipe { input: PotionType::Swiftness, ingredient_item: "minecraft:redstone", output: PotionType::LongSwiftness },
        BrewingRecipe { input: PotionType::Leaping, ingredient_item: "minecraft:redstone", output: PotionType::LongLeaping },
        BrewingRecipe { input: PotionType::FireResistance, ingredient_item: "minecraft:redstone", output: PotionType::LongFireResistance },
        BrewingRecipe { input: PotionType::NightVision, ingredient_item: "minecraft:redstone", output: PotionType::LongNightVision },
        BrewingRecipe { input: PotionType::Invisibility, ingredient_item: "minecraft:redstone", output: PotionType::LongInvisibility },
        BrewingRecipe { input: PotionType::Regeneration, ingredient_item: "minecraft:redstone", output: PotionType::LongRegeneration },
        BrewingRecipe { input: PotionType::Strength, ingredient_item: "minecraft:redstone", output: PotionType::LongStrength },
        BrewingRecipe { input: PotionType::Poison, ingredient_item: "minecraft:redstone", output: PotionType::LongPoison },
        BrewingRecipe { input: PotionType::Weakness, ingredient_item: "minecraft:redstone", output: PotionType::LongWeakness },
        BrewingRecipe { input: PotionType::Slowness, ingredient_item: "minecraft:redstone", output: PotionType::LongSlowness },
        BrewingRecipe { input: PotionType::WaterBreathing, ingredient_item: "minecraft:redstone", output: PotionType::LongWaterBreathing },
        BrewingRecipe { input: PotionType::SlowFalling, ingredient_item: "minecraft:redstone", output: PotionType::LongSlowFalling },
        // Glowstone : amplifier (Strong).
        BrewingRecipe { input: PotionType::Swiftness, ingredient_item: "minecraft:glowstone_dust", output: PotionType::StrongSwiftness },
        BrewingRecipe { input: PotionType::Leaping, ingredient_item: "minecraft:glowstone_dust", output: PotionType::StrongLeaping },
        BrewingRecipe { input: PotionType::Healing, ingredient_item: "minecraft:glowstone_dust", output: PotionType::StrongHealing },
        BrewingRecipe { input: PotionType::Harming, ingredient_item: "minecraft:glowstone_dust", output: PotionType::StrongHarming },
        BrewingRecipe { input: PotionType::Regeneration, ingredient_item: "minecraft:glowstone_dust", output: PotionType::StrongRegeneration },
        BrewingRecipe { input: PotionType::Strength, ingredient_item: "minecraft:glowstone_dust", output: PotionType::StrongStrength },
        BrewingRecipe { input: PotionType::Poison, ingredient_item: "minecraft:glowstone_dust", output: PotionType::StrongPoison },
        BrewingRecipe { input: PotionType::Slowness, ingredient_item: "minecraft:glowstone_dust", output: PotionType::StrongSlowness },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awkward_no_effect() {
        assert!(PotionType::Awkward.primary_effect().is_none());
    }

    #[test]
    fn strong_healing_amplifier_1() {
        let (_, _, amp) = PotionType::StrongHealing.primary_effect().unwrap();
        assert_eq!(amp, 1);
    }

    #[test]
    fn vanilla_recipes_not_empty() {
        let r = vanilla_brewing_recipes();
        assert!(r.len() > 20);
    }
}
