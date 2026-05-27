//! Composter — port PMMP `src/block/Composter.php`. Items compostables + chance.

use std::collections::HashMap;

/// Liste des items qu'on peut composter avec la probabilité d'avancement
/// (0.0 - 1.0). Basée sur les valeurs vanilla Bedrock.
pub fn compostable_items() -> HashMap<&'static str, f32> {
    let mut m = HashMap::new();
    // Chance 30%
    for item in &[
        "minecraft:beetroot_seeds",
        "minecraft:dried_kelp",
        "minecraft:grass",
        "minecraft:kelp",
        "minecraft:leaves",
        "minecraft:melon_seeds",
        "minecraft:pumpkin_seeds",
        "minecraft:sapling",
        "minecraft:seagrass",
        "minecraft:sweet_berries",
        "minecraft:wheat_seeds",
        "minecraft:moss_carpet",
        "minecraft:pink_petals",
        "minecraft:small_dripleaf",
    ] {
        m.insert(*item, 0.3);
    }
    // Chance 50%
    for item in &[
        "minecraft:cactus",
        "minecraft:dried_kelp_block",
        "minecraft:flower",
        "minecraft:melon_slice",
        "minecraft:sugar_cane",
        "minecraft:tall_grass",
        "minecraft:vine",
        "minecraft:wheat",
        "minecraft:sea_pickle",
    ] {
        m.insert(*item, 0.5);
    }
    // Chance 65%
    for item in &[
        "minecraft:apple",
        "minecraft:beetroot",
        "minecraft:carrot",
        "minecraft:cocoa_beans",
        "minecraft:fern",
        "minecraft:lily_pad",
        "minecraft:melon_block",
        "minecraft:mushroom",
        "minecraft:mushroom_block",
        "minecraft:nether_wart",
        "minecraft:potato",
        "minecraft:pumpkin",
        "minecraft:sunflower",
        "minecraft:lily_of_the_valley",
        "minecraft:tall_grass_top",
    ] {
        m.insert(*item, 0.65);
    }
    // Chance 85%
    for item in &[
        "minecraft:baked_potato",
        "minecraft:bread",
        "minecraft:cookie",
        "minecraft:hay_block",
        "minecraft:brown_mushroom_block",
        "minecraft:nether_wart_block",
        "minecraft:warped_wart_block",
        "minecraft:pumpkin_pie",
    ] {
        m.insert(*item, 0.85);
    }
    // Chance 100%
    for item in &[
        "minecraft:cake",
        "minecraft:pumpkin_pie",
        "minecraft:cocoa_beans_plant",
    ] {
        m.insert(*item, 1.0);
    }
    m
}

/// État d'un composter (block entity). Level 0-7 ; 8 = réussite = bonemeal.
#[derive(Debug, Clone, Default)]
pub struct ComposterState {
    pub level: u8,
}

impl ComposterState {
    /// Essaye de composter un item. Retourne `true` si ça a avancé.
    pub fn try_compost(&mut self, item_name: &str) -> bool {
        if self.level >= 7 {
            return false;
        }
        let chance = compostable_items().get(item_name).copied().unwrap_or(0.0);
        if chance == 0.0 {
            return false;
        }
        use rand::Rng;
        if rand::thread_rng().gen::<f32>() < chance {
            self.level += 1;
            true
        } else {
            false
        }
    }

    pub fn is_full(&self) -> bool {
        self.level >= 7
    }

    /// Extraire le bonemeal (= passe level 7→0).
    pub fn take_bonemeal(&mut self) -> bool {
        if self.is_full() {
            self.level = 0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compost_bread_85_pct() {
        let items = compostable_items();
        assert_eq!(items.get("minecraft:bread"), Some(&0.85));
    }

    #[test]
    fn take_bonemeal_only_when_full() {
        let mut c = ComposterState { level: 3 };
        assert!(!c.take_bonemeal());
        c.level = 7;
        assert!(c.take_bonemeal());
        assert_eq!(c.level, 0);
    }
}
