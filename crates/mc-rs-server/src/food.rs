//! Food — restore hunger/saturation + effects.

#[derive(Debug, Clone, Copy)]
pub struct FoodProperties {
    pub hunger_restore: u8,
    pub saturation: f32,
    pub always_edible: bool,     // like golden apple
    pub eat_duration_ticks: u32, // 32 default
    pub cures_bad_food: bool,    // false for normal food
}

/// Vanilla food values per PMMP.
pub fn food_properties(item: &str) -> Option<FoodProperties> {
    Some(match item {
        "minecraft:apple" => FoodProperties {
            hunger_restore: 4,
            saturation: 2.4,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:baked_potato" => FoodProperties {
            hunger_restore: 5,
            saturation: 6.0,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:beef" | "minecraft:raw_beef" => FoodProperties {
            hunger_restore: 3,
            saturation: 1.8,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:cooked_beef" | "minecraft:steak" => FoodProperties {
            hunger_restore: 8,
            saturation: 12.8,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:bread" => FoodProperties {
            hunger_restore: 5,
            saturation: 6.0,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:carrot" => FoodProperties {
            hunger_restore: 3,
            saturation: 3.6,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:chicken" | "minecraft:raw_chicken" => FoodProperties {
            hunger_restore: 2,
            saturation: 1.2,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:cooked_chicken" => FoodProperties {
            hunger_restore: 6,
            saturation: 7.2,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:cookie" => FoodProperties {
            hunger_restore: 2,
            saturation: 0.4,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:golden_apple" => FoodProperties {
            hunger_restore: 4,
            saturation: 9.6,
            always_edible: true,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:enchanted_golden_apple" => FoodProperties {
            hunger_restore: 4,
            saturation: 9.6,
            always_edible: true,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:golden_carrot" => FoodProperties {
            hunger_restore: 6,
            saturation: 14.4,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:melon_slice" => FoodProperties {
            hunger_restore: 2,
            saturation: 1.2,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:mushroom_stew" => FoodProperties {
            hunger_restore: 6,
            saturation: 7.2,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:cooked_porkchop" => FoodProperties {
            hunger_restore: 8,
            saturation: 12.8,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:potato" => FoodProperties {
            hunger_restore: 1,
            saturation: 0.6,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:pumpkin_pie" => FoodProperties {
            hunger_restore: 8,
            saturation: 4.8,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:cooked_rabbit" => FoodProperties {
            hunger_restore: 5,
            saturation: 6.0,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:rabbit_stew" => FoodProperties {
            hunger_restore: 10,
            saturation: 12.0,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:cooked_salmon" => FoodProperties {
            hunger_restore: 6,
            saturation: 9.6,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:cooked_mutton" => FoodProperties {
            hunger_restore: 6,
            saturation: 9.6,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:sweet_berries" => FoodProperties {
            hunger_restore: 2,
            saturation: 0.4,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:beetroot" => FoodProperties {
            hunger_restore: 1,
            saturation: 1.2,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:beetroot_soup" => FoodProperties {
            hunger_restore: 6,
            saturation: 7.2,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:chorus_fruit" => FoodProperties {
            hunger_restore: 4,
            saturation: 2.4,
            always_edible: true,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:honey_bottle" => FoodProperties {
            hunger_restore: 6,
            saturation: 1.2,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: true,
        },
        "minecraft:glow_berries" => FoodProperties {
            hunger_restore: 2,
            saturation: 0.4,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        "minecraft:suspicious_stew" => FoodProperties {
            hunger_restore: 6,
            saturation: 7.2,
            always_edible: false,
            eat_duration_ticks: 32,
            cures_bad_food: false,
        },
        _ => return None,
    })
}

/// Raw chicken gives 30% hunger chance.
pub const RAW_CHICKEN_HUNGER_CHANCE: f32 = 0.3;
/// Rotten flesh gives 80% hunger chance.
pub const ROTTEN_FLESH_HUNGER_CHANCE: f32 = 0.8;
/// Pufferfish always gives poison + nausea + hunger.
pub fn pufferfish_effects() -> &'static [(&'static str, u8, u32)] {
    &[
        ("poison", 1, 60 * 20),
        ("hunger", 2, 15 * 20),
        ("nausea", 1, 15 * 20),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bread_gives_hunger() {
        let f = food_properties("minecraft:bread").unwrap();
        assert_eq!(f.hunger_restore, 5);
    }

    #[test]
    fn golden_apple_always_edible() {
        let f = food_properties("minecraft:golden_apple").unwrap();
        assert!(f.always_edible);
    }
}
