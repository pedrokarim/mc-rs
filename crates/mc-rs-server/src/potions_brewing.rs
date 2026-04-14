//! Potion brewing recipes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotionBase {
    Water,
    Awkward,
    Thick,
    Mundane,
    Regeneration,
    Swiftness,
    FireResistance,
    Poison,
    Healing,
    NightVision,
    Weakness,
    Strength,
    Slowness,
    Leaping,
    Harming,
    WaterBreathing,
    Invisibility,
    Luck,
    TurtleMaster,
    SlowFalling,
    LongRegeneration,
    StrongRegeneration,
    LongSwiftness,
    StrongSwiftness,
    LongFireResistance,
    LongPoison,
    StrongPoison,
    StrongHealing,
    LongNightVision,
    LongWeakness,
    LongStrength,
    StrongStrength,
    LongSlowness,
    StrongSlowness,
    LongLeaping,
    StrongLeaping,
    StrongHarming,
    LongWaterBreathing,
    LongInvisibility,
    LongTurtleMaster,
    StrongTurtleMaster,
    LongSlowFalling,
    OminousBad,
    OminousWind,
    Trial,
}

/// Ingredient + base → result.
pub fn brew(ingredient: &str, base: PotionBase) -> Option<PotionBase> {
    Some(match (ingredient, base) {
        ("minecraft:nether_wart", PotionBase::Water) => PotionBase::Awkward,
        ("minecraft:sugar", PotionBase::Awkward) => PotionBase::Swiftness,
        ("minecraft:golden_carrot", PotionBase::Awkward) => PotionBase::NightVision,
        ("minecraft:spider_eye", PotionBase::Awkward) => PotionBase::Poison,
        ("minecraft:rabbit_foot", PotionBase::Awkward) => PotionBase::Leaping,
        ("minecraft:glistering_melon_slice", PotionBase::Awkward) => PotionBase::Healing,
        ("minecraft:ghast_tear", PotionBase::Awkward) => PotionBase::Regeneration,
        ("minecraft:blaze_powder", PotionBase::Awkward) => PotionBase::Strength,
        ("minecraft:magma_cream", PotionBase::Awkward) => PotionBase::FireResistance,
        ("minecraft:pufferfish", PotionBase::Awkward) => PotionBase::WaterBreathing,
        ("minecraft:golden_melon_slice", PotionBase::Awkward) => PotionBase::Healing,
        ("minecraft:turtle_shell_helmet", PotionBase::Awkward) => PotionBase::TurtleMaster,
        ("minecraft:phantom_membrane", PotionBase::Awkward) => PotionBase::SlowFalling,
        // Fermented spider eye
        ("minecraft:fermented_spider_eye", PotionBase::NightVision) => PotionBase::Invisibility,
        ("minecraft:fermented_spider_eye", PotionBase::Swiftness) => PotionBase::Slowness,
        ("minecraft:fermented_spider_eye", PotionBase::Leaping) => PotionBase::Slowness,
        ("minecraft:fermented_spider_eye", PotionBase::Healing) => PotionBase::Harming,
        ("minecraft:fermented_spider_eye", PotionBase::Poison) => PotionBase::Harming,
        ("minecraft:fermented_spider_eye", PotionBase::Water) => PotionBase::Weakness,
        // Glowstone → strong
        ("minecraft:glowstone_dust", PotionBase::Regeneration) => PotionBase::StrongRegeneration,
        ("minecraft:glowstone_dust", PotionBase::Swiftness) => PotionBase::StrongSwiftness,
        ("minecraft:glowstone_dust", PotionBase::Poison) => PotionBase::StrongPoison,
        ("minecraft:glowstone_dust", PotionBase::Healing) => PotionBase::StrongHealing,
        ("minecraft:glowstone_dust", PotionBase::Strength) => PotionBase::StrongStrength,
        ("minecraft:glowstone_dust", PotionBase::Slowness) => PotionBase::StrongSlowness,
        ("minecraft:glowstone_dust", PotionBase::Leaping) => PotionBase::StrongLeaping,
        ("minecraft:glowstone_dust", PotionBase::Harming) => PotionBase::StrongHarming,
        ("minecraft:glowstone_dust", PotionBase::TurtleMaster) => PotionBase::StrongTurtleMaster,
        // Redstone → long
        ("minecraft:redstone", PotionBase::Regeneration) => PotionBase::LongRegeneration,
        ("minecraft:redstone", PotionBase::Swiftness) => PotionBase::LongSwiftness,
        ("minecraft:redstone", PotionBase::NightVision) => PotionBase::LongNightVision,
        ("minecraft:redstone", PotionBase::Poison) => PotionBase::LongPoison,
        ("minecraft:redstone", PotionBase::FireResistance) => PotionBase::LongFireResistance,
        ("minecraft:redstone", PotionBase::Strength) => PotionBase::LongStrength,
        ("minecraft:redstone", PotionBase::Slowness) => PotionBase::LongSlowness,
        ("minecraft:redstone", PotionBase::Leaping) => PotionBase::LongLeaping,
        ("minecraft:redstone", PotionBase::Invisibility) => PotionBase::LongInvisibility,
        ("minecraft:redstone", PotionBase::Weakness) => PotionBase::LongWeakness,
        ("minecraft:redstone", PotionBase::WaterBreathing) => PotionBase::LongWaterBreathing,
        ("minecraft:redstone", PotionBase::TurtleMaster) => PotionBase::LongTurtleMaster,
        ("minecraft:redstone", PotionBase::SlowFalling) => PotionBase::LongSlowFalling,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_wart_becomes_awkward() {
        assert_eq!(brew("minecraft:nether_wart", PotionBase::Water), Some(PotionBase::Awkward));
    }

    #[test]
    fn invalid_returns_none() {
        assert!(brew("minecraft:stone", PotionBase::Water).is_none());
    }
}
