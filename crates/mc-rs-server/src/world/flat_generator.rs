use super::block_registry::BLOCKS;
use super::chunk_serializer;

/// Generate a flat chunk's serialized payload.
///
/// For now: send empty chunks (0 sub-chunks) with just biome data.
/// This tests the chunk format without block ID issues.
///
/// Returns (sub_chunk_count, payload_bytes).
pub fn generate_flat_chunk() -> (u32, Vec<u8>) {
    let mut payload = Vec::with_capacity(4096);

    // Sub-chunk -4 (y=-64 to -49): bedrock + dirt + grass
    let sub_chunk = build_flat_sub_chunk();
    payload.extend_from_slice(&sub_chunk);

    // All 24 biome sections (always required for overworld, index -4 to 19)
    let biome_ids = [[1u32; 16]; 16];
    let biome_data = chunk_serializer::serialize_biome_sections_from_columns(&biome_ids, 24);
    payload.extend_from_slice(&biome_data);

    // Border blocks count
    payload.push(0); // u8 = 0

    // Tile entities (empty)

    (1, payload) // 1 sub-chunk
}

/// Build the sub-chunk at index -4 containing the flat world layers.
///
/// Real Minecraft flat world layout within the 16-high sub-chunk (local y 0-15):
/// - y=0 (world y=-64): bedrock
/// - y=1 (world y=-63): dirt
/// - y=2 (world y=-62): dirt
/// - y=3 (world y=-61): grass_block
/// - y=4 to y=15: air
fn build_flat_sub_chunk() -> Vec<u8> {
    // Palette: [air, bedrock, dirt, grass_block]
    let palette = vec![BLOCKS.air, BLOCKS.bedrock, BLOCKS.dirt, BLOCKS.grass_block];

    // Build block array: 4096 entries (16x16x16)
    // Index = (x << 8) | (z << 4) | y
    let mut blocks = [0u32; 4096]; // default: palette index 0 (air)

    for x in 0..16u32 {
        for z in 0..16u32 {
            let base = (x << 8) | (z << 4);

            // y=0: bedrock (palette index 1)
            blocks[base as usize] = 1;

            // y=1,2: dirt (palette index 2)
            blocks[(base | 1) as usize] = 2;
            blocks[(base | 2) as usize] = 2;

            // y=3: grass_block (palette index 3)
            blocks[(base | 3) as usize] = 3;

            // y=4-15: air (palette index 0, already default)
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
