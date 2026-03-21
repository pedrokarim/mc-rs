use super::chunk_serializer;

/// Block runtime IDs (FNV1 hashes used when block_network_ids_are_hashes=true).
/// These are the hashed values the client expects for each block state.
/// Extracted from PMMP's canonical_block_states.nbt hash computation.
///
/// For Phase 1, we use known hash values for the 4 blocks we need.
/// The client computes: FNV1_32(block_state_nbt) for each canonical state.
pub mod block_ids {
    // These values must match what the Bedrock client computes.
    // We'll use placeholder runtime IDs and verify against the client.
    // If hashes mode doesn't work, we can fall back to sequential IDs.
    //
    // Actually, with block_network_ids_are_hashes=true, the runtime IDs
    // in the palette ARE the FNV1 hashes. The client looks up blocks by hash.
    //
    // Known FNV1-32a hashes for common block states (from PMMP/BDS analysis):
    pub const AIR: u32 = 0xd5eb690;       // "minecraft:air" default state
    pub const BEDROCK: u32 = 0xbdcaabad;   // "minecraft:bedrock" infiniburn=false
    pub const DIRT: u32 = 0x6d61f16c;      // "minecraft:dirt" dirt_type=normal
    pub const GRASS_BLOCK: u32 = 0x52ab2520; // "minecraft:grass_block" (or grass)
}

/// Generate a flat chunk's serialized payload.
///
/// For now: send empty chunks (0 sub-chunks) with just biome data.
/// This tests the chunk format without block ID issues.
///
/// Returns (sub_chunk_count, payload_bytes).
pub fn generate_flat_chunk() -> (u32, Vec<u8>) {
    let mut payload = Vec::with_capacity(512);

    // 1 empty sub-chunk (all air) — version=8, storage_count=0
    payload.push(8); // version
    payload.push(0); // 0 storage layers = all air

    // All 24 biome sections (always required for overworld, index -4 to 19)
    let biome_section = chunk_serializer::serialize_biome_section_single(1); // Plains
    for _ in 0..24 {
        payload.extend_from_slice(&biome_section);
    }

    // Border blocks count
    payload.push(0); // u8 = 0

    // Tile entities (empty — nothing to write)

    (1, payload) // 1 sub-chunk
}

/// Build the sub-chunk at index -4 containing the flat world layers.
///
/// Layout within the 16-high sub-chunk (local y 0-15):
/// - y=0 (world y=-64): bedrock
/// - y=1 (world y=-63): dirt
/// - y=2 (world y=-62): dirt
/// - y=3 (world y=-61): dirt
/// - y=4 (world y=-60): grass_block
/// - y=5 to y=15: air
fn build_flat_sub_chunk() -> Vec<u8> {
    // Palette: [air, bedrock, dirt, grass_block]
    let palette = vec![
        block_ids::AIR,
        block_ids::BEDROCK,
        block_ids::DIRT,
        block_ids::GRASS_BLOCK,
    ];

    // Build block array: 4096 entries (16x16x16)
    // Index = (x << 8) | (z << 4) | y
    let mut blocks = [0u32; 4096]; // default: palette index 0 (air)

    for x in 0..16u32 {
        for z in 0..16u32 {
            let base = (x << 8) | (z << 4);

            // y=0: bedrock (palette index 1)
            blocks[base as usize] = 1; // y=0: bedrock

            // y=1,2,3: dirt (palette index 2)
            blocks[(base | 1) as usize] = 2;
            blocks[(base | 2) as usize] = 2;
            blocks[(base | 3) as usize] = 2;

            // y=4: grass_block (palette index 3)
            blocks[(base | 4) as usize] = 3;

            // y=5-15: air (palette index 0, already default)
        }
    }

    chunk_serializer::serialize_sub_chunk(&blocks, &palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_flat_chunk() {
        let (count, payload) = generate_flat_chunk();
        assert_eq!(count, 1);
        assert!(!payload.is_empty());
        // Should have: sub_chunk_data + 24 biome sections + 1 byte border
        // Minimum: some bytes for sub-chunk + 24 * 2 bytes biome + 1
        assert!(payload.len() > 50);
    }

    #[test]
    fn test_flat_sub_chunk_has_correct_version() {
        let data = build_flat_sub_chunk();
        assert_eq!(data[0], 8); // version = 8
        assert_eq!(data[1], 1); // 1 storage layer
    }
}
