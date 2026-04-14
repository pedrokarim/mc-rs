//! Plant growth — port PMMP `Crops::onRandomTick()` + autres plantes.

use crate::block_behaviors::{can_crop_grow, crop_max_stage};
use rand::Rng;

/// Chance de croissance par random tick (0.0-1.0) selon la plante.
pub fn growth_chance_per_tick(plant_name: &str) -> f32 {
    match plant_name {
        "minecraft:wheat" => 0.01,
        "minecraft:carrots" | "minecraft:potatoes" => 0.01,
        "minecraft:beetroot" => 0.008,
        "minecraft:nether_wart" => 0.005,
        "minecraft:sugar_cane" => 0.01,
        "minecraft:cactus" => 0.01,
        "minecraft:sapling" => 0.06,
        "minecraft:bamboo" => 0.05,
        "minecraft:cocoa" => 0.005,
        "minecraft:melon_stem" | "minecraft:pumpkin_stem" => 0.01,
        "minecraft:sweet_berry_bush" => 0.02,
        _ => 0.0,
    }
}

/// Effet de bonemeal : force une croissance instantanée (1-N stages aléatoires).
pub fn bonemeal_growth_amount(plant_name: &str) -> u32 {
    let mut rng = rand::thread_rng();
    let max_stage = crop_max_stage(plant_name).unwrap_or(0);
    match plant_name {
        "minecraft:sugar_cane" | "minecraft:cactus" | "minecraft:bamboo" => 0, // bonemeal invalid
        _ => {
            if max_stage == 0 {
                0
            } else {
                rng.gen_range(2..=5).min(max_stage)
            }
        }
    }
}

/// Chance d'un arbre de grandir depuis un sapling.
/// Vanilla : pousse si (random_tick + light >= 9).
pub fn sapling_grows(light_level: u8) -> bool {
    use rand::Rng;
    if light_level < 9 {
        return false;
    }
    rand::thread_rng().gen::<f32>() < growth_chance_per_tick("minecraft:sapling")
}

/// Tree heights vanilla par espèce (base).
pub fn tree_default_height(species: &str) -> u32 {
    match species {
        "oak" => 5,
        "birch" => 7,
        "spruce" => 8,
        "jungle" => 10,
        "acacia" => 6,
        "dark_oak" => 6,
        "cherry" => 7,
        "mangrove" => 7,
        _ => 5,
    }
}

pub fn check_growable(plant_name: &str, light_level: u8, farmland: bool) -> bool {
    if let Some(_) = crop_max_stage(plant_name) {
        can_crop_grow(light_level, farmland)
    } else {
        light_level >= 9
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheat_has_growth_chance() {
        assert!(growth_chance_per_tick("minecraft:wheat") > 0.0);
    }

    #[test]
    fn stone_doesnt_grow() {
        assert_eq!(growth_chance_per_tick("minecraft:stone"), 0.0);
    }

    #[test]
    fn jungle_tree_is_tall() {
        assert_eq!(tree_default_height("jungle"), 10);
    }
}
