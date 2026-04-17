//! Dripstone caves — underground biome.

/// Generates stalactites + stalagmites.
/// Rare dripstone formations.
pub const MIN_Y: i32 = -60;
pub const MAX_Y: i32 = 50;

pub fn unique_blocks() -> &'static [&'static str] {
    &["minecraft:dripstone_block", "minecraft:pointed_dripstone"]
}

pub const BIOME_ID: u8 = 189;

/// Large stalactite max length (45 blocks).
pub const MAX_STALACTITE_LENGTH: u32 = 45;

#[cfg(test)]
mod tests {
    #[test]
    fn unique_blocks_list_non_empty() {
        assert!(!super::unique_blocks().is_empty());
    }
}
