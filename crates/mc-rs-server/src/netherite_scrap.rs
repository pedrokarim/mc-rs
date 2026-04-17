//! Netherite scrap crafting.

/// Ancient debris smelted in furnace → netherite scrap.
pub fn debris_smelt_result() -> (&'static str, f32) {
    ("minecraft:netherite_scrap", 2.0)
}

/// 4 scrap + 4 gold ingots → 1 netherite ingot.
pub fn ingot_recipe() -> (Vec<&'static str>, &'static str) {
    (
        vec![
            "minecraft:netherite_scrap",
            "minecraft:netherite_scrap",
            "minecraft:netherite_scrap",
            "minecraft:netherite_scrap",
            "minecraft:gold_ingot",
            "minecraft:gold_ingot",
            "minecraft:gold_ingot",
            "minecraft:gold_ingot",
        ],
        "minecraft:netherite_ingot",
    )
}

/// Netherite floats in lava.
pub fn floats_in_lava() -> bool {
    true
}

/// Netherite blocks/tools are unburnable.
pub fn is_fire_immune() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debris_smelts_to_scrap() {
        assert_eq!(debris_smelt_result().0, "minecraft:netherite_scrap");
    }
}
