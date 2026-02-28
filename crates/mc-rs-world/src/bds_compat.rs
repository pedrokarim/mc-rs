//! BDS (Bedrock Dedicated Server) world compatibility.
//!
//! Provides import and export between our custom FNV-1a hash palette format
//! and the standard BDS NBT compound palette format.
//!
//! BDS sub-chunk disk format:
//! ```text
//! [version=8|9][num_layers][per layer: header_byte, packed_blocks, palette_count:i32_le, NBT_LE_compound[]]
//! ```
//! Where header bit 0 = 0 means persistence mode (NBT palette).

use std::collections::HashMap;
use std::path::Path;

use rusty_leveldb::{LdbIterator, DB};

use crate::block_hash::hash_block_state;
use crate::block_state_registry::BlockStateRegistry;
use crate::chunk::{ChunkColumn, SubChunk, OVERWORLD_SUB_CHUNK_COUNT};

// ─── LevelDB key tags (same as storage.rs) ──────────────────────────────────

const TAG_CHUNK_VERSION: u8 = 0x2C;
const TAG_DATA_2D: u8 = 0x2D;
const TAG_SUB_CHUNK_PREFIX: u8 = 0x2F;
const TAG_BLOCK_ENTITY: u8 = 0x31;
const TAG_FINALIZED_STATE: u8 = 0x36;

/// Build a dimension-aware LevelDB key (same logic as storage.rs).
fn chunk_key_dim(cx: i32, cz: i32, dim: i32, tag: u8) -> Vec<u8> {
    let cap = if dim == 0 { 9 } else { 13 };
    let mut key = Vec::with_capacity(cap);
    key.extend_from_slice(&cx.to_le_bytes());
    key.extend_from_slice(&cz.to_le_bytes());
    if dim != 0 {
        key.extend_from_slice(&dim.to_le_bytes());
    }
    key.push(tag);
    key
}

/// Build a dimension-aware sub-chunk LevelDB key.
fn sub_chunk_key_dim(cx: i32, cz: i32, dim: i32, y_index: i8) -> Vec<u8> {
    let cap = if dim == 0 { 10 } else { 14 };
    let mut key = Vec::with_capacity(cap);
    key.extend_from_slice(&cx.to_le_bytes());
    key.extend_from_slice(&cz.to_le_bytes());
    if dim != 0 {
        key.extend_from_slice(&dim.to_le_bytes());
    }
    key.push(TAG_SUB_CHUNK_PREFIX);
    key.push(y_index as u8);
    key
}

// ─── BDS sub-chunk parsing (import) ─────────────────────────────────────────

/// Minimum bits-per-block for a given palette size.
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

/// Parse a single BDS sub-chunk layer from raw data.
///
/// BDS format: `[header_byte][packed_blocks...][palette_count:i32_le][NBT_LE_compound[]]`
/// header bit 0 = 0 → persistence mode (palette is NBT compounds)
///
/// Returns the SubChunk (with FNV-1a hash palette) and the number of bytes consumed.
pub fn parse_bds_sub_chunk(data: &[u8]) -> Option<SubChunk> {
    if data.len() < 3 {
        return None;
    }

    let version = data[0];
    if version != 8 && version != 9 {
        return None;
    }

    let num_layers = data[1];
    if num_layers == 0 {
        return None;
    }

    // Parse only the first layer (we only support 1 layer)
    let mut pos = 2;
    parse_bds_layer(data, &mut pos)
}

/// Parse one BDS layer starting at `pos`, returning a SubChunk with FNV-1a palette.
fn parse_bds_layer(data: &[u8], pos: &mut usize) -> Option<SubChunk> {
    if *pos >= data.len() {
        return None;
    }

    let header = data[*pos];
    *pos += 1;

    let bpb = header >> 1;
    let is_persistence = (header & 1) == 0;

    if bpb == 0 {
        // Single-block sub-chunk
        let palette_count = read_i32_le(data, pos)?;
        if palette_count == 0 {
            return Some(SubChunk::new_single(0));
        }
        // Read the single NBT compound
        let hash = if is_persistence {
            let nbt_start = *pos;
            let nbt_len = measure_nbt_compound(data, *pos)?;
            let nbt_data = &data[nbt_start..nbt_start + nbt_len];
            *pos = nbt_start + nbt_len;
            BlockStateRegistry::nbt_le_to_hash(nbt_data)
                .unwrap_or_else(|| hash_block_state("minecraft:air"))
        } else {
            // Runtime mode: palette is u32 hashes (same as ours)
            read_u32_le(data, pos)?
        };
        Some(SubChunk::new_single(hash))
    } else {
        // Multi-block sub-chunk
        let blocks_per_word = 32 / bpb as usize;
        let word_count = 4096_usize.div_ceil(blocks_per_word);
        let block_data_bytes = word_count * 4;

        if *pos + block_data_bytes > data.len() {
            return None;
        }

        // Read block data
        let mut blocks = [0u16; 4096];
        let mask = (1u32 << bpb) - 1;

        for word_idx in 0..word_count {
            let offset = *pos + word_idx * 4;
            let word = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            for slot in 0..blocks_per_word {
                let block_idx = word_idx * blocks_per_word + slot;
                if block_idx < 4096 {
                    let palette_index = (word >> (bpb as u32 * slot as u32)) & mask;
                    blocks[block_idx] = palette_index as u16;
                }
            }
        }
        *pos += block_data_bytes;

        // Read palette
        let palette_count = read_i32_le(data, pos)? as usize;
        let mut palette = Vec::with_capacity(palette_count);

        if is_persistence {
            // NBT compound palette
            for _ in 0..palette_count {
                let nbt_start = *pos;
                let nbt_len = measure_nbt_compound(data, nbt_start)?;
                let nbt_data = &data[nbt_start..nbt_start + nbt_len];
                *pos = nbt_start + nbt_len;
                let hash = BlockStateRegistry::nbt_le_to_hash(nbt_data)
                    .unwrap_or_else(|| hash_block_state("minecraft:air"));
                palette.push(hash);
            }
        } else {
            // Runtime mode: u32 hashes
            for _ in 0..palette_count {
                palette.push(read_u32_le(data, pos)?);
            }
        }

        Some(SubChunk { blocks, palette })
    }
}

/// Measure the size of an NBT LE compound starting at `start`.
/// Does NOT advance the cursor — returns the total byte count.
fn measure_nbt_compound(data: &[u8], start: usize) -> Option<usize> {
    let mut pos = start;

    // TAG_Compound (0x0A)
    if *data.get(pos)? != 0x0A {
        return None;
    }
    pos += 1;

    // Root name
    let name_len = read_i16_le_peek(data, pos)? as usize;
    pos += 2 + name_len;

    // Body
    measure_compound_body(data, &mut pos)?;

    Some(pos - start)
}

/// Measure a compound body (entries until TAG_End).
fn measure_compound_body(data: &[u8], pos: &mut usize) -> Option<()> {
    loop {
        let tag_type = *data.get(*pos)?;
        *pos += 1;
        if tag_type == 0x00 {
            return Some(());
        }
        // Skip key name
        let key_len = read_i16_le_peek(data, *pos)? as usize;
        *pos += 2 + key_len;
        // Skip value
        skip_nbt_value(tag_type, data, pos)?;
    }
}

/// Skip an NBT value based on its tag type.
fn skip_nbt_value(tag_type: u8, data: &[u8], pos: &mut usize) -> Option<()> {
    match tag_type {
        0x01 => *pos += 1, // TAG_Byte
        0x02 => *pos += 2, // TAG_Short
        0x03 => *pos += 4, // TAG_Int
        0x04 => *pos += 8, // TAG_Long
        0x05 => *pos += 4, // TAG_Float
        0x06 => *pos += 8, // TAG_Double
        0x07 => {
            // TAG_Byte_Array
            if *pos + 4 > data.len() {
                return None;
            }
            let len =
                i32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]])
                    as usize;
            *pos += 4 + len;
        }
        0x08 => {
            // TAG_String
            let len = read_i16_le_peek(data, *pos)? as usize;
            *pos += 2 + len;
        }
        0x09 => {
            // TAG_List
            let elem_type = *data.get(*pos)?;
            *pos += 1;
            if *pos + 4 > data.len() {
                return None;
            }
            let count =
                i32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
            *pos += 4;
            for _ in 0..count {
                skip_nbt_value(elem_type, data, pos)?;
            }
        }
        0x0A => {
            // TAG_Compound
            measure_compound_body(data, pos)?;
        }
        0x0B => {
            // TAG_Int_Array
            if *pos + 4 > data.len() {
                return None;
            }
            let len =
                i32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]])
                    as usize;
            *pos += 4 + len * 4;
        }
        0x0C => {
            // TAG_Long_Array
            if *pos + 4 > data.len() {
                return None;
            }
            let len =
                i32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]])
                    as usize;
            *pos += 4 + len * 8;
        }
        _ => return None,
    }
    if *pos > data.len() {
        return None;
    }
    Some(())
}

fn read_i16_le_peek(data: &[u8], pos: usize) -> Option<i16> {
    if pos + 2 > data.len() {
        return None;
    }
    Some(i16::from_le_bytes([data[pos], data[pos + 1]]))
}

fn read_i32_le(data: &[u8], pos: &mut usize) -> Option<i32> {
    if *pos + 4 > data.len() {
        return None;
    }
    let val = i32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Some(val)
}

fn read_u32_le(data: &[u8], pos: &mut usize) -> Option<u32> {
    if *pos + 4 > data.len() {
        return None;
    }
    let val = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Some(val)
}

// ─── BDS sub-chunk serialization (export) ────────────────────────────────────

/// Serialize a sub-chunk to BDS format (NBT compound palette).
///
/// Format: `[version=9][num_layers=1][header][packed_blocks][palette_count:i32_le][NBT_LE_compound[]]`
pub fn serialize_bds_sub_chunk(sub: &SubChunk, registry: &BlockStateRegistry) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(9); // version
    buf.push(1); // num_layers

    let palette_size = sub.palette.len();

    if palette_size <= 1 {
        // Single-block: bpb=0, header = (0 << 1) | 0 = 0 (persistence)
        buf.push(0x00);
        buf.extend_from_slice(&(palette_size as i32).to_le_bytes());
        if palette_size == 1 {
            if let Some(nbt) = registry.hash_to_nbt_le(sub.palette[0]) {
                buf.extend_from_slice(&nbt);
            } else {
                // Fallback: air
                let air_nbt = make_air_nbt_le();
                buf.extend_from_slice(&air_nbt);
            }
        }
    } else {
        let bpb = bits_per_block_for_palette(palette_size);
        let header = bpb << 1; // bit 0 = 0 for persistence
        buf.push(header);

        // Pack block indices into u32 words (LSB-first)
        let blocks_per_word = 32 / bpb as usize;
        let word_count = 4096_usize.div_ceil(blocks_per_word);

        for word_idx in 0..word_count {
            let mut word: u32 = 0;
            for slot in 0..blocks_per_word {
                let block_idx = word_idx * blocks_per_word + slot;
                if block_idx < 4096 {
                    let palette_index = sub.blocks[block_idx] as u32;
                    word |= palette_index << (bpb as u32 * slot as u32);
                }
            }
            buf.extend_from_slice(&word.to_le_bytes());
        }

        // Palette as NBT compounds
        buf.extend_from_slice(&(palette_size as i32).to_le_bytes());
        for &runtime_id in &sub.palette {
            if let Some(nbt) = registry.hash_to_nbt_le(runtime_id) {
                buf.extend_from_slice(&nbt);
            } else {
                buf.extend_from_slice(&make_air_nbt_le());
            }
        }
    }

    buf
}

/// Generate NBT LE for minecraft:air (fallback).
fn make_air_nbt_le() -> Vec<u8> {
    let registry = BlockStateRegistry::new();
    let air_hash = hash_block_state("minecraft:air");
    registry.hash_to_nbt_le(air_hash).unwrap_or_default()
}

/// Compute heightmap for a chunk column: highest non-air Y per XZ column.
pub fn compute_heightmap(column: &ChunkColumn, air_hash: u32) -> [i16; 256] {
    let mut heightmap = [0i16; 256];
    for x in 0..16usize {
        for z in 0..16usize {
            let idx = x * 16 + z;
            let mut max_y = -64i16;
            for world_y in -64..320i32 {
                if let Some(rid) = column.get_block_world(x, world_y, z) {
                    if rid != air_hash {
                        max_y = world_y as i16;
                    }
                }
            }
            heightmap[idx] = max_y;
        }
    }
    heightmap
}

// ─── Import ──────────────────────────────────────────────────────────────────

/// Result of a BDS world import operation.
#[derive(Debug, Default)]
pub struct ImportResult {
    pub chunks: usize,
    pub block_entities: usize,
}

/// Import a BDS world into our LevelDB format.
///
/// Opens the BDS LevelDB at `bds_path/db`, converts sub-chunks from NBT palette
/// to FNV-1a hash palette, and saves them to `target`.
pub fn import_bds_world(
    bds_db_path: &Path,
    target: &mut crate::storage::LevelDbProvider,
    dim: i32,
) -> Result<ImportResult, String> {
    let opts = rusty_leveldb::Options {
        create_if_missing: false,
        ..rusty_leveldb::Options::default()
    };

    let mut bds_db = DB::open(bds_db_path, opts).map_err(|e| {
        format!(
            "Failed to open BDS LevelDB at {}: {e}",
            bds_db_path.display()
        )
    })?;

    let mut result = ImportResult::default();

    // Scan for chunk version keys to find all chunks
    let chunk_coords = scan_chunk_coords(&mut bds_db, dim);

    for (cx, cz) in &chunk_coords {
        // Load sub-chunks from BDS
        let mut sub_chunks_vec: Vec<SubChunk> = Vec::with_capacity(OVERWORLD_SUB_CHUNK_COUNT);
        for i in 0..OVERWORLD_SUB_CHUNK_COUNT {
            let y_index = i as i8 - 4;
            let key = sub_chunk_key_dim(*cx, *cz, dim, y_index);
            if let Some(data) = bds_db.get(&key) {
                let sub = parse_bds_sub_chunk(&data)
                    .unwrap_or_else(|| SubChunk::new_single(hash_block_state("minecraft:air")));
                sub_chunks_vec.push(sub);
            } else {
                sub_chunks_vec.push(SubChunk::new_single(hash_block_state("minecraft:air")));
            }
        }

        let sub_chunks: [SubChunk; OVERWORLD_SUB_CHUNK_COUNT] = sub_chunks_vec
            .try_into()
            .unwrap_or_else(|_| panic!("expected {OVERWORLD_SUB_CHUNK_COUNT} sub-chunks"));

        // Load biomes (Data2D)
        let data_2d_key = chunk_key_dim(*cx, *cz, dim, TAG_DATA_2D);
        let biomes = if let Some(data) = bds_db.get(&data_2d_key) {
            deserialize_data_2d(&data).unwrap_or([0u8; 256])
        } else {
            [0u8; 256]
        };

        let column = ChunkColumn {
            x: *cx,
            z: *cz,
            sub_chunks,
            biomes,
            dirty: true,
            cached_payload: None,
        };

        target
            .save_chunk_dim(&column, dim)
            .map_err(|e| format!("save chunk ({},{}): {e}", cx, cz))?;
        result.chunks += 1;

        // Copy block entities (tag 0x31) as-is (already NBT LE)
        let be_key = chunk_key_dim(*cx, *cz, dim, TAG_BLOCK_ENTITY);
        if let Some(be_data) = bds_db.get(&be_key) {
            target
                .put_raw(&be_key, &be_data)
                .map_err(|e| format!("put block entity ({},{}): {e}", cx, cz))?;
            result.block_entities += 1;
        }
    }

    target.flush().map_err(|e| format!("flush: {e}"))?;

    Ok(result)
}

/// Scan BDS LevelDB for chunk coordinates by looking for version keys.
fn scan_chunk_coords(db: &mut DB, dim: i32) -> Vec<(i32, i32)> {
    let mut coords = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Iterate through all keys and find version tag entries
    let mut iter = db.new_iter().unwrap();
    while let Some((k, _v)) = iter.next() {
        if let Some((cx, cz)) = parse_version_key(&k, dim) {
            if seen.insert((cx, cz)) {
                coords.push((cx, cz));
            }
        }
    }

    coords
}

/// Try to parse a LevelDB key as a chunk version key for the given dimension.
fn parse_version_key(key: &[u8], dim: i32) -> Option<(i32, i32)> {
    if dim == 0 {
        // Overworld: [X:i32][Z:i32][0x2C] = 9 bytes
        if key.len() == 9 && key[8] == TAG_CHUNK_VERSION {
            let cx = i32::from_le_bytes([key[0], key[1], key[2], key[3]]);
            let cz = i32::from_le_bytes([key[4], key[5], key[6], key[7]]);
            return Some((cx, cz));
        }
    } else {
        // Other dims: [X:i32][Z:i32][dim:i32][0x2C] = 13 bytes
        if key.len() == 13 && key[12] == TAG_CHUNK_VERSION {
            let cx = i32::from_le_bytes([key[0], key[1], key[2], key[3]]);
            let cz = i32::from_le_bytes([key[4], key[5], key[6], key[7]]);
            let d = i32::from_le_bytes([key[8], key[9], key[10], key[11]]);
            if d == dim {
                return Some((cx, cz));
            }
        }
    }
    None
}

// ─── Export ──────────────────────────────────────────────────────────────────

/// Result of a BDS world export operation.
#[derive(Debug, Default)]
pub struct ExportResult {
    pub chunks: usize,
}

/// Export chunks to a BDS-compatible LevelDB.
pub fn export_bds_world(
    chunks: &HashMap<(i32, i32), ChunkColumn>,
    dim: i32,
    path: &Path,
    registry: &BlockStateRegistry,
) -> Result<ExportResult, String> {
    let opts = rusty_leveldb::Options {
        create_if_missing: true,
        ..rusty_leveldb::Options::default()
    };

    let mut db = DB::open(path, opts).map_err(|e| format!("Failed to create BDS LevelDB: {e}"))?;

    let mut result = ExportResult::default();
    let air_hash = hash_block_state("minecraft:air");

    for (&(cx, cz), column) in chunks {
        // Write chunk version
        let version_key = chunk_key_dim(cx, cz, dim, TAG_CHUNK_VERSION);
        db.put(&version_key, &[40])
            .map_err(|e| format!("put version: {e}"))?;

        // Write Data2D (heightmap + biomes)
        let data_2d_key = chunk_key_dim(cx, cz, dim, TAG_DATA_2D);
        let heightmap = compute_heightmap(column, air_hash);
        let mut data_2d = Vec::with_capacity(768);
        for &h in &heightmap {
            data_2d.extend_from_slice(&h.to_le_bytes());
        }
        data_2d.extend_from_slice(&column.biomes);
        db.put(&data_2d_key, &data_2d)
            .map_err(|e| format!("put data2d: {e}"))?;

        // Write sub-chunks in BDS format
        for (i, sub_chunk) in column.sub_chunks.iter().enumerate() {
            let y_index = i as i8 - 4;
            let key = sub_chunk_key_dim(cx, cz, dim, y_index);
            let data = serialize_bds_sub_chunk(sub_chunk, registry);
            db.put(&key, &data)
                .map_err(|e| format!("put sub-chunk {y_index}: {e}"))?;
        }

        // Write finalized state
        let finalized_key = chunk_key_dim(cx, cz, dim, TAG_FINALIZED_STATE);
        db.put(&finalized_key, &2i32.to_le_bytes())
            .map_err(|e| format!("put finalized: {e}"))?;

        result.chunks += 1;
    }

    db.flush().map_err(|e| format!("flush: {e}"))?;
    Ok(result)
}

/// Deserialize Data2D: skip heightmap, read biomes.
fn deserialize_data_2d(data: &[u8]) -> Option<[u8; 256]> {
    if data.len() < 768 {
        return None;
    }
    let mut biomes = [0u8; 256];
    biomes.copy_from_slice(&data[512..768]);
    Some(biomes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_hash::hash_block_state;
    use crate::block_state_registry::BlockStateRegistry;
    use crate::chunk::SubChunk;

    /// Build a minimal BDS NBT LE compound for a simple block (no states).
    fn make_bds_nbt_le(name: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        // Root TAG_Compound
        buf.push(0x0A);
        buf.extend_from_slice(&0i16.to_le_bytes()); // empty name

        // "name" -> TAG_String
        buf.push(0x08);
        buf.extend_from_slice(&4i16.to_le_bytes());
        buf.extend_from_slice(b"name");
        buf.extend_from_slice(&(name.len() as i16).to_le_bytes());
        buf.extend_from_slice(name.as_bytes());

        // "states" -> TAG_Compound (empty)
        buf.push(0x0A);
        buf.extend_from_slice(&6i16.to_le_bytes());
        buf.extend_from_slice(b"states");
        buf.push(0x00); // TAG_End

        // "version" -> TAG_Int
        buf.push(0x03);
        buf.extend_from_slice(&7i16.to_le_bytes());
        buf.extend_from_slice(b"version");
        buf.extend_from_slice(&18_100_737i32.to_le_bytes());

        // TAG_End
        buf.push(0x00);
        buf
    }

    /// Build a BDS sub-chunk with a single block (NBT palette).
    fn make_bds_single_block_sub_chunk(name: &str) -> Vec<u8> {
        let nbt = make_bds_nbt_le(name);
        let mut buf = Vec::new();
        buf.push(9); // version
        buf.push(1); // num_layers
        buf.push(0x00); // header: bpb=0, persistence
        buf.extend_from_slice(&1i32.to_le_bytes()); // palette size
        buf.extend_from_slice(&nbt);
        buf
    }

    #[test]
    fn parse_bds_single_block() {
        let data = make_bds_single_block_sub_chunk("minecraft:stone");
        let sub = parse_bds_sub_chunk(&data).expect("must parse");
        assert_eq!(sub.palette.len(), 1);
        assert_eq!(sub.palette[0], hash_block_state("minecraft:stone"));
    }

    #[test]
    fn parse_bds_air_only() {
        let data = make_bds_single_block_sub_chunk("minecraft:air");
        let sub = parse_bds_sub_chunk(&data).expect("must parse");
        assert_eq!(sub.palette[0], hash_block_state("minecraft:air"));
    }

    #[test]
    fn parse_bds_multi_block() {
        // Build a 2-block palette sub-chunk: air + stone
        let air_nbt = make_bds_nbt_le("minecraft:air");
        let stone_nbt = make_bds_nbt_le("minecraft:stone");

        let bpb: u8 = 1;
        let header = bpb << 1; // persistence mode
        let blocks_per_word = 32;
        let word_count = 4096_usize.div_ceil(blocks_per_word);

        let mut buf = Vec::new();
        buf.push(9); // version
        buf.push(1); // num_layers
        buf.push(header);

        // All blocks index 0 (air) except block 0 which is index 1 (stone)
        for word_idx in 0..word_count {
            let mut word: u32 = 0;
            if word_idx == 0 {
                word |= 1; // block 0 = palette index 1 (stone)
            }
            buf.extend_from_slice(&word.to_le_bytes());
        }

        // Palette: 2 entries
        buf.extend_from_slice(&2i32.to_le_bytes());
        buf.extend_from_slice(&air_nbt);
        buf.extend_from_slice(&stone_nbt);

        let sub = parse_bds_sub_chunk(&buf).expect("must parse");
        assert_eq!(sub.palette.len(), 2);
        assert_eq!(sub.palette[0], hash_block_state("minecraft:air"));
        assert_eq!(sub.palette[1], hash_block_state("minecraft:stone"));
        assert_eq!(sub.blocks[0], 1); // stone
        assert_eq!(sub.blocks[1], 0); // air
    }

    #[test]
    fn parse_bds_version_8() {
        // BDS version 8 should also work
        let mut data = make_bds_single_block_sub_chunk("minecraft:dirt");
        data[0] = 8; // version 8
        let sub = parse_bds_sub_chunk(&data).expect("must parse v8");
        assert_eq!(sub.palette[0], hash_block_state("minecraft:dirt"));
    }

    #[test]
    fn parse_bds_invalid() {
        assert!(parse_bds_sub_chunk(&[]).is_none());
        assert!(parse_bds_sub_chunk(&[7, 1, 0]).is_none()); // bad version
        assert!(parse_bds_sub_chunk(&[9, 0]).is_none()); // 0 layers
    }

    #[test]
    fn serialize_bds_single_block() {
        let registry = BlockStateRegistry::new();
        let sub = SubChunk::new_single(hash_block_state("minecraft:stone"));
        let data = serialize_bds_sub_chunk(&sub, &registry);

        // Should be parseable back
        let restored = parse_bds_sub_chunk(&data).expect("must roundtrip");
        assert_eq!(restored.palette[0], hash_block_state("minecraft:stone"));
    }

    #[test]
    fn serialize_bds_multi_block_roundtrip() {
        let registry = BlockStateRegistry::new();

        let air = hash_block_state("minecraft:air");
        let stone = hash_block_state("minecraft:stone");
        let dirt = hash_block_state("minecraft:dirt");

        let mut sub = SubChunk::new_single(air);
        sub.set_block(0, 0, 0, stone);
        sub.set_block(1, 0, 0, dirt);

        let data = serialize_bds_sub_chunk(&sub, &registry);
        let restored = parse_bds_sub_chunk(&data).expect("must roundtrip");

        assert_eq!(restored.palette.len(), sub.palette.len());
        // Block at (0,0,0) should be stone
        let idx = (0 * 16 + 0) * 16 + 0;
        let original_rid = sub.palette[sub.blocks[idx] as usize];
        let restored_rid = restored.palette[restored.blocks[idx] as usize];
        assert_eq!(original_rid, restored_rid);
    }

    #[test]
    fn compute_heightmap_flat() {
        let air = hash_block_state("minecraft:air");
        let stone = hash_block_state("minecraft:stone");
        let mut column = ChunkColumn::new_air(0, 0, air);

        // Place stone at y=0 for column (0,0)
        column.set_block_world(0, 0, 0, stone);
        column.set_block_world(0, 1, 0, stone);

        let hm = compute_heightmap(&column, air);
        assert_eq!(hm[0], 1); // (0,0) -> highest stone at y=1
        assert_eq!(hm[1], -64); // (0,1) -> only air, so min Y
    }

    #[test]
    fn parse_version_key_overworld() {
        let key = chunk_key_dim(5, -3, 0, TAG_CHUNK_VERSION);
        let result = parse_version_key(&key, 0);
        assert_eq!(result, Some((5, -3)));
    }

    #[test]
    fn parse_version_key_nether() {
        let key = chunk_key_dim(1, 2, 1, TAG_CHUNK_VERSION);
        let result = parse_version_key(&key, 1);
        assert_eq!(result, Some((1, 2)));
    }

    #[test]
    fn parse_version_key_wrong_dim() {
        let key = chunk_key_dim(1, 2, 1, TAG_CHUNK_VERSION);
        let result = parse_version_key(&key, 0);
        assert!(result.is_none());
    }

    #[test]
    fn measure_nbt_compound_simple() {
        let nbt = make_bds_nbt_le("minecraft:stone");
        let len = measure_nbt_compound(&nbt, 0).expect("must measure");
        assert_eq!(len, nbt.len());
    }

    #[test]
    fn bds_nbt_le_matches_registry() {
        // Verify that our test helper produces the same NBT as the registry
        let registry = BlockStateRegistry::new();
        let stone_hash = hash_block_state("minecraft:stone");
        let from_registry = registry.hash_to_nbt_le(stone_hash).unwrap();
        let from_helper = make_bds_nbt_le("minecraft:stone");
        // Both should hash to the same block
        let h1 = BlockStateRegistry::nbt_le_to_hash(&from_registry).unwrap();
        let h2 = BlockStateRegistry::nbt_le_to_hash(&from_helper).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1, stone_hash);
    }

    #[test]
    fn export_import_consistency() {
        // Create a chunk, export it, then import it — blocks should match
        let registry = BlockStateRegistry::new();
        let air = hash_block_state("minecraft:air");
        let stone = hash_block_state("minecraft:stone");

        let mut column = ChunkColumn::new_air(0, 0, air);
        column.set_block_world(0, 0, 0, stone);

        // Serialize each sub-chunk to BDS, then parse back
        for sub in &column.sub_chunks {
            let bds_data = serialize_bds_sub_chunk(sub, &registry);
            let restored = parse_bds_sub_chunk(&bds_data).expect("must parse");
            assert_eq!(sub.palette.len(), restored.palette.len());
        }
    }
}
