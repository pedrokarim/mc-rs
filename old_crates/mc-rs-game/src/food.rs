//! Food nutrition data for Minecraft Bedrock items.

/// Nutrition data for a food item.
#[derive(Debug, Clone, Copy)]
pub struct FoodData {
    /// Hunger points restored (0-20 scale).
    pub hunger: i32,
    /// Saturation points restored.
    pub saturation: f32,
}

/// Returns the nutrition data for a food item, or `None` if the item is not food.
///
/// Item names should be in the format `minecraft:item_name`.
pub fn food_data(item_name: &str) -> Option<FoodData> {
    let name = item_name.strip_prefix("minecraft:").unwrap_or(item_name);
    let (hunger, saturation) = match name {
        "apple" => (4, 2.4),
        "baked_potato" => (5, 6.0),
        "beef" => (3, 1.8),
        "bread" => (5, 6.0),
        "carrot" => (3, 3.6),
        "chicken" => (2, 1.2),
        "cooked_beef" => (8, 12.8),
        "cooked_chicken" => (6, 7.2),
        "cooked_cod" => (5, 6.0),
        "cooked_mutton" => (6, 9.6),
        "cooked_porkchop" => (8, 12.8),
        "cooked_rabbit" => (5, 6.0),
        "cooked_salmon" => (6, 9.6),
        "cookie" => (2, 0.4),
        "dried_kelp" => (1, 0.6),
        "enchanted_golden_apple" => (4, 9.6),
        "golden_apple" => (4, 9.6),
        "golden_carrot" => (6, 14.4),
        "melon_slice" => (2, 1.2),
        "mushroom_stew" => (6, 7.2),
        "mutton" => (2, 1.2),
        "porkchop" => (3, 1.8),
        "potato" => (1, 0.6),
        "pumpkin_pie" => (8, 4.8),
        "rabbit" => (3, 1.8),
        "rabbit_stew" => (10, 12.0),
        "cod" => (2, 0.4),
        "salmon" => (2, 0.4),
        "sweet_berries" => (2, 0.4),
        "glow_berries" => (2, 0.4),
        "beetroot" => (1, 1.2),
        "beetroot_soup" => (6, 7.2),
        "honey_bottle" => (6, 1.2),
        "rotten_flesh" => (4, 0.8),
        "spider_eye" => (2, 3.2),
        "poisonous_potato" => (2, 1.2),
        "pufferfish" => (1, 0.2),
        "tropical_fish" => (1, 0.2),
        _ => return None,
    };
    Some(FoodData { hunger, saturation })
}

/// Returns `true` if the item is a food item.
pub fn is_food(item_name: &str) -> bool {
    food_data(item_name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_food_values() {
        let data = food_data("minecraft:cooked_beef").unwrap();
        assert_eq!(data.hunger, 8);
        assert!((data.saturation - 12.8).abs() < 0.01);

        let data = food_data("minecraft:golden_carrot").unwrap();
        assert_eq!(data.hunger, 6);
        assert!((data.saturation - 14.4).abs() < 0.01);

        let data = food_data("minecraft:dried_kelp").unwrap();
        assert_eq!(data.hunger, 1);
        assert!((data.saturation - 0.6).abs() < 0.01);
    }

    #[test]
    fn non_food_returns_none() {
        assert!(food_data("minecraft:diamond").is_none());
        assert!(food_data("minecraft:stone").is_none());
        assert!(food_data("minecraft:wooden_sword").is_none());
    }

    #[test]
    fn is_food_check() {
        assert!(is_food("minecraft:bread"));
        assert!(is_food("minecraft:apple"));
        assert!(!is_food("minecraft:iron_ingot"));
        assert!(!is_food("minecraft:stick"));
    }
}
