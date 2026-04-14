//! Lush caves — underground biome.

/// Azalea tree roots generate moss.
/// Moss + bone meal spreads.
/// Glow berries hang from cave vines.
/// Spore blossoms hang from ceiling.
/// Dripleaf + small dripleaf.
/// Rooted dirt grows hanging roots.

pub const MIN_Y: i32 = -60;
pub const MAX_Y: i32 = 50;

/// Unique blocks in lush caves.
pub fn unique_blocks() -> &'static [&'static str] {
    &[
        "minecraft:moss_block",
        "minecraft:moss_carpet",
        "minecraft:azalea",
        "minecraft:flowering_azalea",
        "minecraft:azalea_leaves",
        "minecraft:flowering_azalea_leaves",
        "minecraft:big_dripleaf",
        "minecraft:big_dripleaf_stem",
        "minecraft:small_dripleaf",
        "minecraft:spore_blossom",
        "minecraft:cave_vines",
        "minecraft:cave_vines_plant",
        "minecraft:glow_berries",
        "minecraft:glow_lichen",
        "minecraft:rooted_dirt",
        "minecraft:hanging_roots",
    ]
}

/// Biome ID (Bedrock).
pub const BIOME_ID: u8 = 188;

#[cfg(test)]
mod tests {
    #[test]
    fn unique_blocks_list_non_empty() {
        assert!(!super::unique_blocks().is_empty());
    }
}
