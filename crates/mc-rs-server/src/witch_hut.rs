//! Witch hut — swamp hut where witches spawn.

/// Swamp huts always spawn a witch, with cat.
pub const HAS_WITCH: bool = true;
pub const HAS_CAT: bool = true;

/// Contents: crafting table, cauldron, flower pot with red mushroom.
pub fn contents_blocks() -> &'static [&'static str] {
    &[
        "minecraft:crafting_table",
        "minecraft:cauldron",
        "minecraft:flower_pot",
        "minecraft:spruce_planks",
        "minecraft:spruce_log",
        "minecraft:spruce_fence",
    ]
}

/// Small size (7x7).
pub const SIZE: u32 = 7;

/// Found in swamp/mangrove biomes.
pub fn valid_biomes() -> &'static [u8] {
    &[6, 134, 198] // swamp, swamp hills, mangrove swamp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_cauldron() {
        assert!(contents_blocks().iter().any(|s| s.contains("cauldron")));
    }
}
