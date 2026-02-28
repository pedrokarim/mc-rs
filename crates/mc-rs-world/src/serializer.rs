//! Sub-chunk and chunk column serialization to Bedrock network format.

use bytes::{BufMut, BytesMut};

use crate::chunk::{ChunkColumn, SubChunk, OVERWORLD_SUB_CHUNK_COUNT};

/// Serialize a full chunk column to the LevelChunk packet payload.
///
/// Returns `(sub_chunk_count, payload_bytes)`.
/// Payload = SubChunks[] + BiomeData + BorderBlocks(0x00).
pub fn serialize_chunk_column(column: &ChunkColumn) -> (u32, Vec<u8>) {
    let mut buf = BytesMut::new();

    // Sub-chunk Y indices: sub-chunk 0 maps to Y=-64 so y_index = -4
    for (index, sub_chunk) in column.sub_chunks.iter().enumerate() {
        let y_index = index as i8 - 4; // 0 -> -4, 4 -> 0, 23 -> 19
        serialize_sub_chunk(&mut buf, sub_chunk, y_index);
    }

    // Biome data from the chunk's biome array
    serialize_biome_data(&mut buf, &column.biomes);

    // Border blocks: empty (not Education Edition)
    buf.put_u8(0x00);

    (OVERWORLD_SUB_CHUNK_COUNT as u32, buf.to_vec())
}

/// Serialize a chunk column, returning a cached result if available.
///
/// If the column has a cached payload, returns it directly.
/// Otherwise, serializes, stores the result in the cache, and returns it.
pub fn serialize_chunk_column_cached(column: &mut ChunkColumn) -> (u32, Vec<u8>) {
    if let Some((count, ref payload)) = column.cached_payload {
        return (count, payload.clone());
    }
    let result = serialize_chunk_column(column);
    column.cached_payload = Some(result.clone());
    result
}

/// Serialize a single sub-chunk to network format (Version 9).
fn serialize_sub_chunk(buf: &mut BytesMut, sub_chunk: &SubChunk, y_index: i8) {
    buf.put_u8(9); // version
    buf.put_u8(1); // num_layers (no waterlogging)
    buf.put_u8(y_index as u8); // y_index (i8 -> u8 for two's complement)

    let palette_size = sub_chunk.palette.len();

    if palette_size <= 1 {
        // Single-block sub-chunk: bits_per_block = 0
        // storage_header = (0 << 1) | 1 = 1 (runtime flag set)
        buf.put_u8(0x01);
        // NO block data words for bits_per_block = 0
        // Palette size as signed VarInt
        write_zigzag_varint(buf, palette_size as i32);
        // Palette entry
        if palette_size == 1 {
            write_zigzag_varint(buf, sub_chunk.palette[0] as i32);
        }
    } else {
        let bpb = bits_per_block_for_palette(palette_size);
        let storage_header = (bpb << 1) | 1; // bit 0 = runtime flag
        buf.put_u8(storage_header);

        // Pack block indices into u32 words (LSB-first)
        let blocks_per_word = 32 / bpb as usize;
        let word_count = 4096_usize.div_ceil(blocks_per_word);

        for word_idx in 0..word_count {
            let mut word: u32 = 0;
            for slot in 0..blocks_per_word {
                let block_idx = word_idx * blocks_per_word + slot;
                if block_idx < 4096 {
                    let palette_index = sub_chunk.blocks[block_idx] as u32;
                    word |= palette_index << (bpb as u32 * slot as u32);
                }
            }
            buf.put_u32_le(word);
        }

        // Palette
        write_zigzag_varint(buf, palette_size as i32);
        for &runtime_id in &sub_chunk.palette {
            write_zigzag_varint(buf, runtime_id as i32);
        }
    }
}

/// Determine minimum bits-per-block for a given palette size.
/// Valid values: 1, 2, 3, 4, 5, 6, 8, 16.
fn bits_per_block_for_palette(palette_size: usize) -> u8 {
    match palette_size {
        0..=1 => 0,
        2 => 1,
        3..=4 => 2,
        5..=8 => 3,
        9..=16 => 4,
        17..=32 => 5,
        33..=64 => 6,
        65..=256 => 8,
        _ => 16,
    }
}

/// Serialize biome data from the chunk's 2D biome array.
///
/// Each of 24 sections encodes biomes at 4x4x4 resolution (64 entries).
/// Since our biome data is 2D, all 4 Y levels in a section share the same biome.
/// All 24 sections are identical, so we compute the section data once and repeat it.
fn serialize_biome_data(buf: &mut BytesMut, biomes: &[u8; 256]) {
    // Build a 4x4 biome grid (downsampled from 16x16 to 4x4 by taking every 4th column).
    // Indexed as [section_x * 4 + section_z] where section_x/z are in 0..4.
    let mut biome_4x4 = [0u8; 16];
    for sx in 0..4 {
        for sz in 0..4 {
            // Sample the center of each 4-block span
            let bx = sx * 4 + 2;
            let bz = sz * 4 + 2;
            biome_4x4[sx * 4 + sz] = biomes[bx * 16 + bz];
        }
    }

    // Check if all 16 entries are the same biome (common case)
    let all_same = biome_4x4.iter().all(|&b| b == biome_4x4[0]);

    // Build section data once (all 24 sections are identical since biomes are 2D)
    let mut section_buf = BytesMut::new();

    if all_same {
        // Single-biome section: header = 0x00 (0 bits = single value)
        section_buf.put_u8(0x00);
        write_zigzag_varint(&mut section_buf, biome_4x4[0] as i32);
    } else {
        // Multi-biome section: palette-based encoding for 64 entries.
        // Build palette using O(1) lookup array instead of Vec::contains().
        let mut biome_to_palette = [0xFFu8; 256];
        let mut palette: Vec<u8> = Vec::new();
        for &b in &biome_4x4 {
            if biome_to_palette[b as usize] == 0xFF {
                biome_to_palette[b as usize] = palette.len() as u8;
                palette.push(b);
            }
        }

        let bpe = bits_per_entry_for_biome_palette(palette.len());
        // Biome storage header: (bpe << 1) — no runtime flag for biomes
        section_buf.put_u8(bpe << 1);

        // Pack 64 entries into u32 words (4x4x4, all 4 Y levels have same biome as XZ)
        let entries_per_word = 32 / bpe as usize;
        let word_count = 64_usize.div_ceil(entries_per_word);

        for word_idx in 0..word_count {
            let mut word: u32 = 0;
            for slot in 0..entries_per_word {
                let entry_idx = word_idx * entries_per_word + slot;
                if entry_idx < 64 {
                    let sx = entry_idx % 4;
                    let sz = (entry_idx / 4) % 4;
                    let biome_id = biome_4x4[sx * 4 + sz];
                    let palette_idx = biome_to_palette[biome_id as usize] as u32;
                    word |= palette_idx << (bpe as u32 * slot as u32);
                }
            }
            section_buf.put_u32_le(word);
        }

        // Palette
        write_zigzag_varint(&mut section_buf, palette.len() as i32);
        for &biome_id in &palette {
            write_zigzag_varint(&mut section_buf, biome_id as i32);
        }
    }

    // Write the same section data 24 times
    let section_bytes = section_buf.freeze();
    for _ in 0..OVERWORLD_SUB_CHUNK_COUNT {
        buf.extend_from_slice(&section_bytes);
    }
}

/// Determine minimum bits-per-entry for a biome palette.
/// Valid values: 1, 2, 3, 4, 5, 6.
fn bits_per_entry_for_biome_palette(palette_size: usize) -> u8 {
    match palette_size {
        0..=1 => 0,
        2 => 1,
        3..=4 => 2,
        5..=8 => 3,
        9..=16 => 4,
        17..=32 => 5,
        _ => 6,
    }
}

fn write_zigzag_varint(buf: &mut BytesMut, value: i32) {
    let encoded = ((value << 1) ^ (value >> 31)) as u32;
    write_varuint32(buf, encoded);
}

fn write_varuint32(buf: &mut BytesMut, mut value: u32) {
    loop {
        if value & !0x7F == 0 {
            buf.put_u8(value as u8);
            return;
        }
        buf.put_u8((value & 0x7F | 0x80) as u8);
        value >>= 7;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_per_block_selection() {
        assert_eq!(bits_per_block_for_palette(1), 0);
        assert_eq!(bits_per_block_for_palette(2), 1);
        assert_eq!(bits_per_block_for_palette(3), 2);
        assert_eq!(bits_per_block_for_palette(4), 2);
        assert_eq!(bits_per_block_for_palette(5), 3);
        assert_eq!(bits_per_block_for_palette(8), 3);
        assert_eq!(bits_per_block_for_palette(16), 4);
        assert_eq!(bits_per_block_for_palette(256), 8);
    }

    #[test]
    fn single_block_subchunk_serialization() {
        let sub = SubChunk::new_single(42);
        let mut buf = BytesMut::new();
        serialize_sub_chunk(&mut buf, &sub, 0);

        assert_eq!(buf[0], 9, "version");
        assert_eq!(buf[1], 1, "num_layers");
        assert_eq!(buf[2], 0, "y_index");
        assert_eq!(buf[3], 0x01, "storage_header (bpb=0, runtime=1)");
        // Palette size = 1 as zigzag VarInt = 2
        assert_eq!(buf[4], 0x02, "palette_size zigzag(1) = 2");
        // Palette entry = 42 as zigzag VarInt = 84
        assert_eq!(buf[5], 84, "palette[0] zigzag(42) = 84");
        assert_eq!(buf.len(), 6, "total bytes for single-block sub-chunk");
    }

    #[test]
    fn mixed_subchunk_has_block_data() {
        let mut sub = SubChunk::new_single(10); // air
        sub.set_block(0, 0, 0, 20); // bedrock
        sub.set_block(0, 1, 0, 30); // dirt
        sub.set_block(0, 2, 0, 40); // grass

        let mut buf = BytesMut::new();
        serialize_sub_chunk(&mut buf, &sub, 0);

        assert_eq!(buf[0], 9, "version");
        assert_eq!(buf[1], 1, "num_layers");
        assert_eq!(buf[2], 0, "y_index");
        // 4 palette entries -> bits_per_block = 2, header = (2 << 1) | 1 = 5
        assert_eq!(buf[3], 5, "storage_header (bpb=2, runtime=1)");
        // With bpb=2: blocks_per_word=16, word_count=256
        // After header: 256 * 4 = 1024 bytes of block data
        // Total = 3 (header) + 1 (storage_header) + 1024 (data) + palette
        assert!(buf.len() > 1024, "should contain block data words");
    }

    #[test]
    fn packed_array_correctness() {
        // Create a sub-chunk with 2 block types to get bpb=1
        let mut sub = SubChunk::new_single(100);
        sub.set_block(0, 1, 0, 200); // block index 1 -> palette index 1

        let mut buf = BytesMut::new();
        serialize_sub_chunk(&mut buf, &sub, 0);

        // bpb=1, header = (1 << 1) | 1 = 3
        assert_eq!(buf[3], 3, "storage_header (bpb=1, runtime=1)");

        // First u32 word (bytes 4-7): block indices 0-31
        // Block 0 (palette idx 0) at bit 0, block 1 (palette idx 1) at bit 1
        let word0 = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(word0 & 1, 0, "block 0 should be palette index 0");
        assert_eq!((word0 >> 1) & 1, 1, "block 1 should be palette index 1");
    }

    #[test]
    fn full_chunk_column_serialization() {
        use crate::block_hash::FlatWorldBlocks;
        use crate::flat_generator::generate_flat_chunk;

        let blocks = FlatWorldBlocks::compute();
        let column = generate_flat_chunk(0, 0, &blocks);
        let (count, payload) = serialize_chunk_column(&column);

        assert_eq!(count, 24);
        assert!(!payload.is_empty());
        // First bytes should be the first sub-chunk: version=9, layers=1
        assert_eq!(payload[0], 9);
        assert_eq!(payload[1], 1);
        // Payload should end with border_blocks=0x00
        assert_eq!(*payload.last().unwrap(), 0x00);
    }

    #[test]
    fn single_biome_section_encoding() {
        let biomes = [1u8; 256]; // All plains
        let mut buf = BytesMut::new();
        serialize_biome_data(&mut buf, &biomes);
        // 24 sections, each: 0x00 (header) + zigzag(1) = 0x02
        assert_eq!(buf.len(), 24 * 2);
        for i in 0..24 {
            assert_eq!(buf[i * 2], 0x00, "section {i} header");
            assert_eq!(buf[i * 2 + 1], 0x02, "section {i} biome plains=zigzag(1)=2");
        }
    }

    #[test]
    fn multi_biome_section_encoding() {
        let mut biomes = [1u8; 256]; // Plains
                                     // Set some columns to desert (2)
        for z in 0..16 {
            for x in 8..16 {
                biomes[x * 16 + z] = 2;
            }
        }
        let mut buf = BytesMut::new();
        serialize_biome_data(&mut buf, &biomes);
        // Should have palette-based encoding (not single-biome)
        // First section header should NOT be 0x00 since we have 2 biomes
        assert_ne!(buf[0], 0x00, "multi-biome should use palette encoding");
        assert!(buf.len() > 24 * 2, "multi-biome data should be larger");
    }

    #[test]
    fn multi_biome_all_sections_identical() {
        // All 24 sections should be byte-identical since biomes are 2D
        let mut biomes = [1u8; 256];
        for z in 0..16 {
            for x in 8..16 {
                biomes[x * 16 + z] = 2;
            }
        }
        let mut buf = BytesMut::new();
        serialize_biome_data(&mut buf, &biomes);
        // Find section size (total / 24)
        assert_eq!(buf.len() % 24, 0, "biome data should be 24 equal sections");
        let section_size = buf.len() / 24;
        let first_section = &buf[..section_size];
        for i in 1..24 {
            let section = &buf[i * section_size..(i + 1) * section_size];
            assert_eq!(first_section, section, "section {i} should match section 0");
        }
    }

    #[test]
    fn biome_palette_bits() {
        assert_eq!(bits_per_entry_for_biome_palette(1), 0);
        assert_eq!(bits_per_entry_for_biome_palette(2), 1);
        assert_eq!(bits_per_entry_for_biome_palette(4), 2);
        assert_eq!(bits_per_entry_for_biome_palette(8), 3);
    }

    #[test]
    fn cached_payload_returns_same_result() {
        use crate::block_hash::FlatWorldBlocks;
        use crate::flat_generator::generate_flat_chunk;

        let blocks = FlatWorldBlocks::compute();
        let mut column = generate_flat_chunk(0, 0, &blocks);

        // First call: no cache, computes and stores
        assert!(column.cached_payload.is_none());
        let (count1, payload1) = serialize_chunk_column_cached(&mut column);
        assert!(column.cached_payload.is_some());

        // Second call: returns cached result
        let (count2, payload2) = serialize_chunk_column_cached(&mut column);
        assert_eq!(count1, count2);
        assert_eq!(payload1, payload2);
    }

    #[test]
    fn cache_invalidated_on_block_change() {
        use crate::block_hash::FlatWorldBlocks;
        use crate::flat_generator::generate_flat_chunk;

        let blocks = FlatWorldBlocks::compute();
        let mut column = generate_flat_chunk(0, 0, &blocks);

        // Populate cache
        let (_count, payload_before) = serialize_chunk_column_cached(&mut column);
        assert!(column.cached_payload.is_some());

        // Modify block → invalidates cache
        column.set_block_world(0, 0, 0, 999);
        assert!(column.cached_payload.is_none());

        // Re-serialize produces different payload
        let (_count, payload_after) = serialize_chunk_column_cached(&mut column);
        assert_ne!(payload_before, payload_after);
        assert!(column.cached_payload.is_some());
    }

    #[test]
    fn cached_matches_uncached() {
        use crate::block_hash::FlatWorldBlocks;
        use crate::flat_generator::generate_flat_chunk;

        let blocks = FlatWorldBlocks::compute();
        let mut column = generate_flat_chunk(0, 0, &blocks);

        let (count_uncached, payload_uncached) = serialize_chunk_column(&column);
        let (count_cached, payload_cached) = serialize_chunk_column_cached(&mut column);

        assert_eq!(count_uncached, count_cached);
        assert_eq!(payload_uncached, payload_cached);
    }
}
