//! Sub-chunk and chunk column serialization to Bedrock network format.

use bytes::{BufMut, BytesMut};

use crate::chunk::{ChunkColumn, SubChunk, OVERWORLD_SUB_CHUNK_COUNT};

/// Plains biome ID.
const BIOME_PLAINS: i32 = 1;

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

    // Biome data: 24 sections, all plains
    serialize_biome_data(&mut buf);

    // Border blocks: empty (not Education Edition)
    buf.put_u8(0x00);

    (OVERWORLD_SUB_CHUNK_COUNT as u32, buf.to_vec())
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

/// Serialize biome data for the chunk payload.
/// For flat world: all plains, single-palette encoding per section.
fn serialize_biome_data(buf: &mut BytesMut) {
    // 24 biome sections (one per sub-chunk).
    // Single-biome section: header = 0x00 (0 bits = single value), then VarInt biome ID.
    for _ in 0..OVERWORLD_SUB_CHUNK_COUNT {
        buf.put_u8(0x00); // 0 bits per entry = single value
        write_zigzag_varint(buf, BIOME_PLAINS);
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
}
