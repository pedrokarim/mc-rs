use mc_rs_proto::io::{ProtoReader, ProtoWriter};

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
    serialize_biome_section(&[biome_id; 64])
}

/// Serialize a single 4x4x4 biome section.
pub fn serialize_biome_section(biomes: &[u32; 64]) -> Vec<u8> {
    let mut palette = Vec::new();
    let mut palette_indices = [0u32; 64];

    for (i, &biome) in biomes.iter().enumerate() {
        let idx = if let Some(pos) = palette.iter().position(|&b| b == biome) {
            pos as u32
        } else {
            palette.push(biome);
            (palette.len() - 1) as u32
        };
        palette_indices[i] = idx;
    }

    let mut w = ProtoWriter::with_capacity(4);
    serialize_paletted_storage(&mut w, &palette_indices, &palette);
    w.into_bytes()
}

/// Serialize all biome sections for a chunk from its 16x16 biome columns.
///
/// Bedrock uses 4x4x4 biome paletted storages per sub-chunk. We do not yet
/// model vertical biome variation, so the same 4x4 horizontal map is repeated
/// across the 4 Y slices of each section and across all vertical sections.
pub fn serialize_biome_sections_from_columns(
    biome_ids: &[[u32; 16]; 16],
    section_count: usize,
) -> Vec<u8> {
    let mut coarse = [0u32; 64];

    #[allow(clippy::needless_range_loop)]
    for coarse_x in 0..4usize {
        #[allow(clippy::needless_range_loop)]
        for coarse_z in 0..4usize {
            let mut counts: Vec<(u32, usize)> = Vec::new();
            for x in (coarse_x * 4)..(coarse_x * 4 + 4) {
                for z in (coarse_z * 4)..(coarse_z * 4 + 4) {
                    let biome = biome_ids[x][z];
                    if let Some((_, count)) = counts.iter_mut().find(|(id, _)| *id == biome) {
                        *count += 1;
                    } else {
                        counts.push((biome, 1));
                    }
                }
            }

            let mut selected = biome_ids[coarse_x * 4][coarse_z * 4];
            let mut best_count = 0usize;
            for (biome, count) in counts {
                if count > best_count {
                    selected = biome;
                    best_count = count;
                }
            }

            for coarse_y in 0..4usize {
                let idx = (coarse_x << 4) | (coarse_z << 2) | coarse_y;
                coarse[idx] = selected;
            }
        }
    }

    let section = serialize_biome_section(&coarse);
    let mut data = Vec::with_capacity(section.len() * section_count);
    for _ in 0..section_count {
        data.extend_from_slice(&section);
    }
    data
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

/// A sub-chunk storing 16x16x16 blocks as actual runtime IDs.
/// Index = (x << 8) | (z << 4) | y
#[derive(Clone)]
pub struct SubChunk {
    pub blocks: [u32; 4096],
}

impl SubChunk {
    pub fn new_air(air_id: u32) -> Self {
        Self {
            blocks: [air_id; 4096],
        }
    }

    /// Get the block runtime ID at local coordinates.
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u32 {
        self.blocks[(x << 8) | (z << 4) | y]
    }

    /// Set the block runtime ID at local coordinates.
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, runtime_id: u32) {
        self.blocks[(x << 8) | (z << 4) | y] = runtime_id;
    }

    /// Serialize this sub-chunk to network format.
    pub fn serialize(&self) -> Vec<u8> {
        // Build palette from blocks
        let mut palette: Vec<u32> = Vec::new();
        let mut palette_indices = [0u32; 4096];

        for (i, &block) in self.blocks.iter().enumerate() {
            let idx = if let Some(pos) = palette.iter().position(|&b| b == block) {
                pos as u32
            } else {
                palette.push(block);
                (palette.len() - 1) as u32
            };
            palette_indices[i] = idx;
        }

        serialize_sub_chunk(&palette_indices, &palette)
    }
}

/// Deserialize a single sub-chunk from network bytes.
/// Returns (SubChunk, bytes_consumed).
pub fn deserialize_sub_chunk_data(data: &[u8]) -> Option<(SubChunk, usize)> {
    if data.len() < 2 {
        return None;
    }

    let mut reader = ProtoReader::new(data);
    let version = reader.read_u8().ok()?;
    if version != 8 {
        return None;
    }

    let storage_count = reader.read_u8().ok()?;
    let air_id = super::block_registry::BLOCKS.air;
    let mut blocks = [air_id; 4096];

    if storage_count > 0 {
        let header = reader.read_u8().ok()?;
        let bits_per_block = header >> 1;

        if bits_per_block == 0 {
            // Single-value palette
            let value = reader.read_var_i32().ok()? as u32;
            blocks.fill(value);
        } else {
            // Read word array
            let blocks_per_word = 32 / bits_per_block as usize;
            let word_count = 4096usize.div_ceil(blocks_per_word);
            let mask = (1u32 << bits_per_block) - 1;

            let mut palette_indices = [0u32; 4096];
            for word_idx in 0..word_count {
                let word = reader.read_u32_le().ok()?;
                for bit_idx in 0..blocks_per_word {
                    let block_idx = word_idx * blocks_per_word + bit_idx;
                    if block_idx >= 4096 {
                        break;
                    }
                    palette_indices[block_idx] =
                        (word >> (bit_idx * bits_per_block as usize)) & mask;
                }
            }

            // Read palette
            let palette_count = reader.read_var_i32().ok()? as usize;
            let mut palette = Vec::with_capacity(palette_count);
            for _ in 0..palette_count {
                palette.push(reader.read_var_i32().ok()? as u32);
            }

            // Convert palette indices to runtime IDs
            for i in 0..4096 {
                let idx = palette_indices[i] as usize;
                blocks[i] = if idx < palette.len() {
                    palette[idx]
                } else {
                    air_id
                };
            }
        }
        // Only use the first storage layer (main blocks)
    }

    Some((SubChunk { blocks }, reader.position()))
}

/// Rebuild the full network payload from sub-chunks and biome data.
pub fn rebuild_network_payload(sub_chunks: &[SubChunk], biome_data: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(16384);

    for sub in sub_chunks {
        payload.extend_from_slice(&sub.serialize());
    }

    payload.extend_from_slice(biome_data);

    // Border blocks count
    payload.push(0);

    payload
}

/// Parse a full chunk payload into sub-chunks and biome data.
/// Returns (Vec<SubChunk>, biome_data_bytes).
pub fn parse_chunk_payload(
    data: &[u8],
    sub_chunk_count: u32,
    air_id: u32,
) -> (Vec<SubChunk>, Vec<u8>) {
    let mut sub_chunks = Vec::with_capacity(sub_chunk_count as usize);
    let mut offset = 0;

    for _ in 0..sub_chunk_count {
        if offset >= data.len() {
            sub_chunks.push(SubChunk::new_air(air_id));
            continue;
        }
        match deserialize_sub_chunk_data(&data[offset..]) {
            Some((sub, consumed)) => {
                sub_chunks.push(sub);
                offset += consumed;
            }
            None => {
                sub_chunks.push(SubChunk::new_air(air_id));
                break;
            }
        }
    }

    // Remaining data is biome sections + border blocks
    let biome_data = if offset < data.len() {
        // Remove the trailing border blocks byte from biome data
        let remaining = &data[offset..];
        if remaining.len() > 1 {
            remaining[..remaining.len() - 1].to_vec()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    (sub_chunks, biome_data)
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
    fn test_biome_sections_from_columns_not_empty() {
        let mut biome_ids = [[1u32; 16]; 16];
        for row in biome_ids.iter_mut().skip(8) {
            for cell in row.iter_mut() {
                *cell = 2;
            }
        }

        let data = serialize_biome_sections_from_columns(&biome_ids, 24);
        assert!(!data.is_empty());
        assert!(data.len() >= 24 * 2);
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
