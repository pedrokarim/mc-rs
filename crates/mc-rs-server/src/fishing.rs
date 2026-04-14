//! Fishing — port PMMP `src/entity/projectile/FishingHook.php` (partiel).
//!
//! La ligne de pêche se lance, reste immobile sur l'eau, bob aléatoirement,
//! et peut attraper un poisson/loot treasure/junk avec des probabilités.

use mc_rs_proto::packets::player::ItemStack;
use rand::Rng;

/// Catégorie de loot pêche vanilla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishLootCategory {
    Fish,
    Treasure,
    Junk,
}

/// Chance de chaque catégorie (sur 100) — valeurs vanilla.
pub fn base_category_chances() -> [(FishLootCategory, u32); 3] {
    [
        (FishLootCategory::Fish, 85),
        (FishLootCategory::Treasure, 5),
        (FishLootCategory::Junk, 10),
    ]
}

/// Chances modifiées par enchants :
/// - Luck of the Sea : + treasure, - junk
/// - Lure : réduit le wait time (appliqué ailleurs, pas aux chances).
pub fn modified_category_chances(luck_of_the_sea: u8) -> [(FishLootCategory, u32); 3] {
    let luck = luck_of_the_sea.min(3) as u32;
    // Conservation : +treasure = -junk (fish inchangé).
    [
        (FishLootCategory::Fish, 85),
        (FishLootCategory::Treasure, 5 + 2 * luck),
        (FishLootCategory::Junk, 10 - 2 * luck),
    ]
}

/// Retourne un loot aléatoire en pondérant par chance.
pub fn roll_fish_loot(luck_of_the_sea: u8) -> FishLootCategory {
    let chances = modified_category_chances(luck_of_the_sea);
    let total: u32 = chances.iter().map(|(_, w)| w).sum();
    let mut rng = rand::thread_rng();
    let mut roll = rng.gen_range(0..total);
    for (cat, weight) in chances.iter() {
        if roll < *weight {
            return *cat;
        }
        roll -= *weight;
    }
    FishLootCategory::Fish
}

/// Liste des fish items vanilla avec poids (pour générer un loot).
/// PMMP `FishingHook::catchFish()`.
pub fn fish_items() -> &'static [(&'static str, u32)] {
    &[
        ("minecraft:cod", 60),
        ("minecraft:raw_salmon", 25),
        ("minecraft:tropical_fish", 2),
        ("minecraft:pufferfish", 13),
    ]
}

pub fn junk_items() -> &'static [(&'static str, u32)] {
    &[
        ("minecraft:leather_boots", 10),
        ("minecraft:leather", 10),
        ("minecraft:bone", 10),
        ("minecraft:bowl", 10),
        ("minecraft:stick", 5),
        ("minecraft:string", 5),
        ("minecraft:ink_sac", 1),
        ("minecraft:water_bottle", 10),
        ("minecraft:rotten_flesh", 10),
        ("minecraft:tripwire_hook", 10),
    ]
}

pub fn treasure_items() -> &'static [(&'static str, u32)] {
    &[
        ("minecraft:bow", 16),
        ("minecraft:enchanted_book", 16),
        ("minecraft:fishing_rod", 16),
        ("minecraft:name_tag", 16),
        ("minecraft:nautilus_shell", 16),
        ("minecraft:saddle", 16),
        ("minecraft:lily_pad", 4),
    ]
}

/// Génère un ItemStack loot à partir d'une catégorie.
pub fn generate_loot(category: FishLootCategory) -> Option<ItemStack> {
    use crate::item_registry::network_id;
    let pool = match category {
        FishLootCategory::Fish => fish_items(),
        FishLootCategory::Junk => junk_items(),
        FishLootCategory::Treasure => treasure_items(),
    };
    let total: u32 = pool.iter().map(|(_, w)| w).sum();
    let mut rng = rand::thread_rng();
    let mut roll = rng.gen_range(0..total);
    for (name, w) in pool {
        if roll < *w {
            return network_id(name).map(|id| ItemStack::new(id, 1, 0));
        }
        roll -= *w;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modified_chances_sum_to_100() {
        for luck in 0..=3 {
            let ch = modified_category_chances(luck);
            let total: u32 = ch.iter().map(|(_, w)| w).sum();
            assert_eq!(total, 100, "luck={}", luck);
        }
    }

    #[test]
    fn luck_increases_treasure() {
        let none = modified_category_chances(0);
        let max = modified_category_chances(3);
        let treasure_none = none.iter().find(|(c, _)| *c == FishLootCategory::Treasure).unwrap().1;
        let treasure_max = max.iter().find(|(c, _)| *c == FishLootCategory::Treasure).unwrap().1;
        assert!(treasure_max > treasure_none);
    }
}
