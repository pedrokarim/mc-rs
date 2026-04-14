//! Comportements de blocs — ports sélectifs de `.reference/PocketMine-MP/src/block/*`.
//!
//! Regroupe :
//! - **Gravity blocks** (sand, gravel, concrete_powder) qui tombent quand rien
//!   dessous. Port `Fallable.php`.
//! - **Crop growth** (wheat, carrots, potatoes, beetroot) avec tick random.
//! - **Ice/snow melting** selon température.
//! - **Fire spread** simplifié.
//! - **Leaves decay** quand séparé du log.

/// Blocs affectés par la gravité. IDs runtime à matcher.
/// Port PMMP `Fallable.php::isFallable`.
pub fn is_fallable(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:sand"
            | "minecraft:red_sand"
            | "minecraft:gravel"
            | "minecraft:anvil"
            | "minecraft:chipped_anvil"
            | "minecraft:damaged_anvil"
            | "minecraft:white_concrete_powder"
            | "minecraft:orange_concrete_powder"
            | "minecraft:magenta_concrete_powder"
            | "minecraft:light_blue_concrete_powder"
            | "minecraft:yellow_concrete_powder"
            | "minecraft:lime_concrete_powder"
            | "minecraft:pink_concrete_powder"
            | "minecraft:gray_concrete_powder"
            | "minecraft:light_gray_concrete_powder"
            | "minecraft:cyan_concrete_powder"
            | "minecraft:purple_concrete_powder"
            | "minecraft:blue_concrete_powder"
            | "minecraft:brown_concrete_powder"
            | "minecraft:green_concrete_powder"
            | "minecraft:red_concrete_powder"
            | "minecraft:black_concrete_powder"
            | "minecraft:pointed_dripstone"
            | "minecraft:scaffolding"
    )
}

/// Crops avec stades de croissance PMMP.
/// Retourne le nombre de stades max (par ex. wheat = 7).
pub fn crop_max_stage(block_name: &str) -> Option<u32> {
    match block_name {
        "minecraft:wheat" | "minecraft:carrots" | "minecraft:potatoes" => Some(7),
        "minecraft:beetroot" => Some(3),
        "minecraft:nether_wart" => Some(3),
        "minecraft:sweet_berry_bush" => Some(3),
        "minecraft:cocoa" => Some(2),
        _ => None,
    }
}

/// Conditions de pousse d'une crop.
/// PMMP `Crops::onRandomTick()` : nécessite light >= 9 + farmland.
pub fn can_crop_grow(light_level: u8, has_farmland_below: bool) -> bool {
    light_level >= 9 && has_farmland_below
}

/// Feuilles qui décèrent quand séparées d'un log.
/// PMMP `Leaves::onNearbyBlockChange()`.
pub fn leaves_need_log_within(block_name: &str) -> Option<u32> {
    if block_name.ends_with("_leaves") {
        Some(4) // distance max = 4 blocs
    } else {
        None
    }
}

/// Valeur de température d'un biome (pour snow / ice).
/// PMMP `Biome::getTemperature()`.
pub fn biome_temperature(biome_name: &str) -> f32 {
    match biome_name {
        "snowy_plains" | "snowy_tundra" | "ice_spikes" => 0.0,
        "taiga" | "snowy_taiga" | "old_growth_pine_taiga" => 0.25,
        "plains" | "forest" | "birch_forest" => 0.7,
        "jungle" | "savanna" | "desert" => 1.2,
        "nether" => 2.0,
        _ => 0.5,
    }
}

pub fn snow_can_form(temperature: f32) -> bool {
    temperature <= 0.15
}

pub fn water_can_freeze(temperature: f32) -> bool {
    temperature <= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sand_is_fallable() {
        assert!(is_fallable("minecraft:sand"));
        assert!(is_fallable("minecraft:gravel"));
        assert!(!is_fallable("minecraft:stone"));
    }

    #[test]
    fn wheat_has_7_stages() {
        assert_eq!(crop_max_stage("minecraft:wheat"), Some(7));
        assert_eq!(crop_max_stage("minecraft:beetroot"), Some(3));
        assert_eq!(crop_max_stage("minecraft:stone"), None);
    }

    #[test]
    fn crop_needs_light_and_farmland() {
        assert!(can_crop_grow(9, true));
        assert!(!can_crop_grow(8, true));
        assert!(!can_crop_grow(15, false));
    }

    #[test]
    fn leaves_decay_distance() {
        assert_eq!(leaves_need_log_within("minecraft:oak_leaves"), Some(4));
        assert_eq!(leaves_need_log_within("minecraft:stone"), None);
    }

    #[test]
    fn frozen_biomes() {
        assert!(snow_can_form(biome_temperature("snowy_plains")));
        assert!(water_can_freeze(biome_temperature("ice_spikes")));
        assert!(!snow_can_form(biome_temperature("plains")));
    }
}
