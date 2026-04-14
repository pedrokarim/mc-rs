//! Pale Garden — 1.21.4 biome (creaking home).

pub const BIOME_ID: u8 = 200;

pub fn unique_blocks() -> &'static [&'static str] {
    &[
        "minecraft:pale_oak_log",
        "minecraft:pale_oak_leaves",
        "minecraft:pale_oak_sapling",
        "minecraft:pale_oak_planks",
        "minecraft:pale_moss_block",
        "minecraft:pale_moss_carpet",
        "minecraft:pale_hanging_moss",
        "minecraft:creaking_heart",
        "minecraft:open_eyeblossom",
        "minecraft:closed_eyeblossom",
    ]
}

/// Native mobs.
pub fn native_mobs() -> &'static [&'static str] {
    &["minecraft:creaking"]
}

/// Eyeblossom cycles open/closed based on day/night.
pub fn eyeblossom_opens_at_night() -> bool { true }

#[cfg(test)]
mod tests {
    #[test]
    fn has_creaking() {
        assert!(super::native_mobs().contains(&"minecraft:creaking"));
    }
}
