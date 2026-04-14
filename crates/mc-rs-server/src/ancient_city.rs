//! Ancient City structure — Deep Dark biome.

pub const MIN_DEPTH: i32 = -52;
pub const MAX_DEPTH: i32 = -36;

/// Ancient City loot chests.
pub fn loot_chest_entries() -> &'static [(&'static str, u32, u32, u32)] {
    // (item, min, max, weight)
    &[
        ("minecraft:diamond", 1, 3, 10),
        ("minecraft:iron_ingot", 1, 5, 30),
        ("minecraft:lead", 1, 2, 25),
        ("minecraft:name_tag", 1, 1, 25),
        ("minecraft:enchanted_book", 1, 1, 20),
        ("minecraft:echo_shard", 1, 3, 40),
        ("minecraft:disc_fragment_5", 1, 1, 5),
        ("minecraft:music_disc_5", 1, 1, 5),
        ("minecraft:snowball", 8, 16, 20),
        ("minecraft:amethyst_shard", 1, 3, 30),
        ("minecraft:swift_sneak_template", 1, 1, 5),
        ("minecraft:ward_armor_trim_template", 1, 1, 5),
    ]
}

/// Redstone needed to open reinforced deepslate vault (not possible).
/// Instead, requires breaking the deepslate.
pub fn reinforced_deepslate_breakable_only_by_wither() -> bool { true }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loot_has_echo_shard() {
        assert!(loot_chest_entries().iter().any(|(i, _, _, _)| *i == "minecraft:echo_shard"));
    }
}
