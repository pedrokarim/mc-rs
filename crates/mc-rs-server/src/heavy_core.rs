//! Heavy core — required for making mace.

pub const BLOCK_ID: u16 = 1200;
pub const ITEM_ID: &str = "minecraft:heavy_core";

/// Can be found in ominous vault.
pub fn spawn_location() -> &'static str {
    "ominous_vault_loot"
}

/// Crafted into mace with breeze rod.
pub fn mace_recipe() -> (&'static [&'static str], &'static str) {
    (
        &["minecraft:heavy_core", "minecraft:breeze_rod"],
        "minecraft:mace",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mace_recipe_non_empty() {
        assert!(!mace_recipe().0.is_empty());
    }
}
