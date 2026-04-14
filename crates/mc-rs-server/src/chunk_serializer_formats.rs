//! Chunk serialization format versions.

/// Max chunk section height (sub-chunks).
pub const WORLD_HEIGHT_SECTIONS: usize = 24; // -64 to 320 with 24 sections of 16 blocks
pub const SECTION_HEIGHT: usize = 16;

/// Chunk format version (Bedrock).
pub const CHUNK_VERSION_V9: u8 = 40;
pub const CHUNK_VERSION_V10: u8 = 41;

/// Sub-chunk version.
pub const SUB_CHUNK_VERSION: u8 = 9;

/// Biome data — 3D biomes (4x4x4 resolution).
pub const BIOME_RESOLUTION: usize = 4;

/// Height map — 16*16 short entries.
pub const HEIGHT_MAP_SIZE: usize = 16 * 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biome_4x4x4() {
        assert_eq!(BIOME_RESOLUTION, 4);
    }
}
