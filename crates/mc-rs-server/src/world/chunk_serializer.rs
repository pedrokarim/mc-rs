use mc_rs_proto::io::ProtoWriter;

/// Serialize a sub-chunk with the given palette (network format).
///
/// Format:
/// - version: u8 = 8
/// - storage_count: u8
/// - For each storage layer:
///   - header: (bits_per_block << 1) | 1 (runtime flag)
///   - If bits == 0: single VarInt32 palette value (no word array)
///   - If bits > 0: word array + VarInt32 palette count + VarInt32[] palette
pub fn serialize_sub_chunk(blocks: &[u32; 4096], palette: &[u32]) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(512);
    w.write_u8(8); // version
    w.write_u8(1); // storage_count = 1 layer

    serialize_paletted_storage(&mut w, blocks, palette);

    w.into_bytes()
}

/// Serialize an empty sub-chunk (all air).
#[allow(dead_code)]
pub fn serialize_empty_sub_chunk() -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(4);
    w.write_u8(8); // version
    w.write_u8(0); // storage_count = 0 (no layers)
    w.into_bytes()
}

/// Serialize a single biome section (4x4x4 = 64 entries).
/// For flat world: all Plains (biome ID = 1), single-value palette.
pub fn serialize_biome_section_single(biome_id: u32) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(4);
    // header: bits=0, runtime flag set
    w.write_u8(1); // header: bits=0, runtime flag=1
                   // Single palette value (VarInt32 zigzag)
    w.write_var_i32(biome_id as i32);
    w.into_bytes()
}

/// Serialize a paletted storage (blocks or biomes).
fn serialize_paletted_storage(w: &mut ProtoWriter, data: &[u32], palette: &[u32]) {
    if palette.len() <= 1 {
        // Single-value: bits_per_block = 0
        let header = 1u8; // bits=0, runtime flag
        w.write_u8(header);
        w.write_var_i32(palette.first().copied().unwrap_or(0) as i32);
        return;
    }

    // Calculate bits per block
    let bits = bits_for_palette(palette.len());
    let header = (bits << 1) | 1; // runtime flag
    w.write_u8(header);

    // Word array
    let blocks_per_word = 32 / bits as usize;
    let word_count = data.len().div_ceil(blocks_per_word);

    for word_idx in 0..word_count {
        let mut word: u32 = 0;
        for bit_idx in 0..blocks_per_word {
            let block_idx = word_idx * blocks_per_word + bit_idx;
            if block_idx >= data.len() {
                break;
            }
            let palette_idx = data[block_idx];
            word |= (palette_idx & ((1 << bits) - 1)) << (bit_idx * bits as usize);
        }
        w.write_u32_le(word);
    }

    // Palette count + entries (VarInt32 zigzag)
    w.write_var_i32(palette.len() as i32);
    for &entry in palette {
        w.write_var_i32(entry as i32);
    }
}

/// Determine the number of bits per block for a given palette size.
/// Must be one of: 1, 2, 3, 4, 5, 6, 8, 16
fn bits_for_palette(palette_size: usize) -> u8 {
    match palette_size {
        0..=2 => 1,
        3..=4 => 2,
        5..=8 => 3,
        9..=16 => 4,
        17..=32 => 5,
        33..=64 => 6,
        65..=256 => 8,
        _ => 16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_sub_chunk() {
        let data = serialize_empty_sub_chunk();
        assert_eq!(data, vec![8, 0]); // version=8, 0 layers
    }

    #[test]
    fn test_single_block_sub_chunk() {
        let blocks = [0u32; 4096]; // all palette index 0
        let palette = vec![42]; // single block type
        let data = serialize_sub_chunk(&blocks, &palette);

        assert_eq!(data[0], 8); // version
        assert_eq!(data[1], 1); // 1 storage layer
        assert_eq!(data[2], 1); // header: bits=0, runtime=1
                                // VarInt32 zigzag(42) = 84 = [0x54]
    }

    #[test]
    fn test_biome_section_plains() {
        let data = serialize_biome_section_single(1); // Plains
        assert_eq!(data[0], 1); // header: bits=0, runtime=1
                                // VarInt32 zigzag(1) = 2 = [0x02]
        assert_eq!(data[1], 0x02);
    }

    #[test]
    fn test_multi_palette() {
        // 3 block types → 2 bits per block
        let palette = vec![10, 20, 30];
        let mut blocks = [0u32; 4096];
        blocks[0] = 0; // palette[0] = 10
        blocks[1] = 1; // palette[1] = 20
        blocks[2] = 2; // palette[2] = 30

        let data = serialize_sub_chunk(&blocks, &palette);
        assert_eq!(data[0], 8); // version
        assert_eq!(data[1], 1); // 1 layer
        let header = data[2];
        let bits = (header >> 1) & 0x7F;
        assert_eq!(bits, 2); // 2 bits per block for 3 palette entries
    }
}
