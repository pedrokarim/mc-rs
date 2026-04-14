//! Beacon effects + tier / pyramid.

use crate::effects::EffectKind;

/// Pyramid size (1 to 4).
pub fn pyramid_size(full_bases: u32) -> u32 {
    full_bases.min(4)
}

/// Required block count per tier.
pub fn blocks_required(tier: u32) -> u32 {
    match tier {
        1 => 9,
        2 => 34,
        3 => 83,
        4 => 164,
        _ => 0,
    }
}

/// Effects available per tier.
pub fn tier_1_effects() -> &'static [EffectKind] {
    &[EffectKind::Speed, EffectKind::Haste]
}

pub fn tier_2_effects() -> &'static [EffectKind] {
    &[EffectKind::Speed, EffectKind::Haste, EffectKind::Resistance, EffectKind::JumpBoost]
}

pub fn tier_3_effects() -> &'static [EffectKind] {
    &[
        EffectKind::Speed,
        EffectKind::Haste,
        EffectKind::Resistance,
        EffectKind::JumpBoost,
        EffectKind::Strength,
    ]
}

pub fn tier_4_effects() -> &'static [EffectKind] {
    &[
        EffectKind::Speed,
        EffectKind::Haste,
        EffectKind::Resistance,
        EffectKind::JumpBoost,
        EffectKind::Strength,
        EffectKind::Regeneration,
    ]
}

/// Payment items.
pub fn payment_items() -> &'static [&'static str] {
    &[
        "minecraft:iron_ingot",
        "minecraft:gold_ingot",
        "minecraft:emerald",
        "minecraft:diamond",
        "minecraft:netherite_ingot",
    ]
}

/// Range based on tier (20, 30, 40, 50 blocks).
pub fn range(tier: u32) -> u32 {
    10 + tier * 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_4_needs_164() {
        assert_eq!(blocks_required(4), 164);
    }

    #[test]
    fn tier_4_has_regeneration() {
        assert!(tier_4_effects().contains(&EffectKind::Regeneration));
    }
}
