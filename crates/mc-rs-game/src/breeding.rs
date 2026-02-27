//! Breeding and temptation data for passive mobs.
//!
//! Maps mob types to the food items that attract them and allow breeding.

/// Returns the list of items that tempt / breed a given mob type.
///
/// Returns an empty slice for non-breedable mobs.
pub fn tempt_items(mob_type: &str) -> &'static [&'static str] {
    match mob_type {
        "minecraft:cow" | "minecraft:mooshroom" => &["minecraft:wheat"],
        "minecraft:sheep" => &["minecraft:wheat"],
        "minecraft:pig" => &["minecraft:carrot", "minecraft:potato", "minecraft:beetroot"],
        "minecraft:chicken" => &[
            "minecraft:wheat_seeds",
            "minecraft:beetroot_seeds",
            "minecraft:melon_seeds",
            "minecraft:pumpkin_seeds",
        ],
        "minecraft:rabbit" => &[
            "minecraft:carrot",
            "minecraft:golden_carrot",
            "minecraft:dandelion",
        ],
        "minecraft:horse" | "minecraft:donkey" => {
            &["minecraft:golden_carrot", "minecraft:golden_apple"]
        }
        "minecraft:wolf" => &[
            "minecraft:beef",
            "minecraft:cooked_beef",
            "minecraft:chicken",
            "minecraft:cooked_chicken",
            "minecraft:porkchop",
            "minecraft:cooked_porkchop",
            "minecraft:mutton",
            "minecraft:cooked_mutton",
            "minecraft:rabbit",
            "minecraft:cooked_rabbit",
            "minecraft:rotten_flesh",
        ],
        "minecraft:cat" | "minecraft:ocelot" => &["minecraft:cod", "minecraft:salmon"],
        _ => &[],
    }
}

/// Whether a mob type supports breeding at all.
pub fn is_breedable(mob_type: &str) -> bool {
    !tempt_items(mob_type).is_empty()
}

/// Check if a specific item tempts/breeds a specific mob type.
pub fn is_tempt_item(mob_type: &str, item_name: &str) -> bool {
    tempt_items(mob_type).contains(&item_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cow_tempted_by_wheat() {
        assert!(is_tempt_item("minecraft:cow", "minecraft:wheat"));
        assert!(!is_tempt_item("minecraft:cow", "minecraft:carrot"));
    }

    #[test]
    fn chicken_tempted_by_seeds() {
        assert!(is_tempt_item("minecraft:chicken", "minecraft:wheat_seeds"));
        assert!(is_tempt_item(
            "minecraft:chicken",
            "minecraft:pumpkin_seeds"
        ));
        assert!(!is_tempt_item("minecraft:chicken", "minecraft:wheat"));
    }

    #[test]
    fn pig_tempted_by_veggies() {
        assert!(is_tempt_item("minecraft:pig", "minecraft:carrot"));
        assert!(is_tempt_item("minecraft:pig", "minecraft:potato"));
        assert!(is_tempt_item("minecraft:pig", "minecraft:beetroot"));
    }

    #[test]
    fn non_breedable_returns_empty() {
        assert!(!is_breedable("minecraft:zombie"));
        assert!(!is_breedable("minecraft:skeleton"));
        assert!(tempt_items("minecraft:zombie").is_empty());
    }

    #[test]
    fn all_breedable_mobs() {
        assert!(is_breedable("minecraft:cow"));
        assert!(is_breedable("minecraft:pig"));
        assert!(is_breedable("minecraft:chicken"));
        assert!(is_breedable("minecraft:sheep"));
        assert!(is_breedable("minecraft:wolf"));
    }
}
