//! LevelDB key prefixes for chunk storage.

/// Sub-chunk prefix.
pub const SUB_CHUNK_PREFIX: u8 = 47; // '/'
/// Block entity prefix.
pub const BLOCK_ENTITY_PREFIX: u8 = 49; // '1'
/// Entity prefix.
pub const ENTITY_PREFIX: u8 = 50; // '2'
/// Pending ticks.
pub const PENDING_TICKS_PREFIX: u8 = 51; // '3'
/// Random ticks.
pub const RANDOM_TICKS_PREFIX: u8 = 52; // '4'
/// Biomes (3D).
pub const BIOMES_PREFIX: u8 = 43; // '+' (legacy 2D was 45 '-')
/// Heightmap.
pub const HEIGHTMAP_PREFIX: u8 = 45; // '-' (now 'Hsh')
/// Version (chunk format version).
pub const VERSION_KEY: u8 = 44; // ','
/// Hardcoded spawn area.
pub const SPAWN_AREA_KEY: u8 = 57; // '9'
/// Finalization (population status).
pub const FINALIZATION_KEY: u8 = 54; // '6'

#[cfg(test)]
mod tests {
    #[test]
    fn distinct_prefixes() {
        assert_ne!(super::SUB_CHUNK_PREFIX, super::BLOCK_ENTITY_PREFIX);
    }
}
