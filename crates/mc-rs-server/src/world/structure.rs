//! Structure loading and placement system.
//! Loads Bedrock Edition .nbt structure files (gzip-compressed NBT LE)
//! and places them in the world during terrain generation.

use std::collections::HashMap;
use std::io::Read;

use super::biome::biome_id;
use super::flat_generator::block_ids;
use super::random::Random;

/// A loaded structure: dimensions + block data.
#[derive(Debug, Clone)]
pub struct Structure {
    pub size_x: i32,
    pub size_y: i32,
    pub size_z: i32,
    /// Block data: (local_x, local_y, local_z) -> runtime block ID.
    /// Only non-air blocks are stored.
    pub blocks: HashMap<(i32, i32, i32), u32>,
}

/// Load a structure from a gzip-compressed NBT file.
/// Supports both formats:
/// - Legacy (Java-style): `palette` + `blocks` with `pos` and `state`
/// - Bedrock: `structure.block_indices` + `structure.palette.default.block_palette`
pub fn load_structure(path: &str, block_mapping: &HashMap<String, u32>) -> Option<Structure> {
    let compressed = std::fs::read(path).ok()?;

    // Decompress gzip
    let mut decoder = flate2::read::GzDecoder::new(&compressed[..]);
    let mut data = Vec::new();
    decoder.read_to_end(&mut data).ok()?;

    // Parse NBT LE
    let mut buf = &data[..];
    let root = mc_rs_nbt::read_nbt_be(&mut buf).ok()?;

    // Extract size
    let size_list = root.compound.get("size")?;
    let (size_x, size_y, size_z) = extract_int_list_3(size_list)?;

    // Try legacy format first (palette + blocks with pos)
    if let Some(palette_list) = root.compound.get("palette") {
        return load_legacy_structure(
            palette_list,
            &root.compound,
            size_x,
            size_y,
            size_z,
            block_mapping,
        );
    }

    // Try Bedrock format (structure.block_indices)
    if let Some(mc_rs_nbt::tag::NbtTag::Compound(structure)) = root.compound.get("structure") {
        return load_bedrock_structure(structure, size_x, size_y, size_z, block_mapping);
    }

    None
}

/// Load legacy structure format (palette + blocks with pos/state).
fn load_legacy_structure(
    palette_tag: &mc_rs_nbt::tag::NbtTag,
    root: &HashMap<String, mc_rs_nbt::tag::NbtTag>,
    size_x: i32,
    size_y: i32,
    size_z: i32,
    block_mapping: &HashMap<String, u32>,
) -> Option<Structure> {
    // Parse palette: list of compounds with "Name" field
    let palette_list = match palette_tag {
        mc_rs_nbt::tag::NbtTag::List(list) => list,
        _ => return None,
    };

    let mut palette = Vec::new();
    for entry in palette_list {
        match entry {
            mc_rs_nbt::tag::NbtTag::Compound(block) => {
                // Legacy uses "Name" (capital N)
                let name = match block.get("Name") {
                    Some(mc_rs_nbt::tag::NbtTag::String(s)) => s.as_str(),
                    _ => "minecraft:air",
                };
                let runtime_id = block_mapping.get(name).copied().unwrap_or(block_ids::AIR);
                palette.push(runtime_id);
            }
            _ => palette.push(block_ids::AIR),
        }
    }

    // Parse blocks: list of compounds with "state" (palette index) and "pos" (list of 3 ints)
    let blocks_list = match root.get("blocks")? {
        mc_rs_nbt::tag::NbtTag::List(list) => list,
        _ => return None,
    };

    let mut blocks = HashMap::new();
    for block_tag in blocks_list {
        match block_tag {
            mc_rs_nbt::tag::NbtTag::Compound(block) => {
                let state = match block.get("state") {
                    Some(mc_rs_nbt::tag::NbtTag::Int(v)) => *v as usize,
                    _ => continue,
                };
                let pos = match block.get("pos") {
                    Some(tag) => extract_int_list_3(tag),
                    _ => continue,
                };
                let (x, y, z) = match pos {
                    Some(p) => p,
                    None => continue,
                };

                let runtime_id = palette.get(state).copied().unwrap_or(block_ids::AIR);
                if runtime_id != block_ids::AIR {
                    blocks.insert((x, y, z), runtime_id);
                }
            }
            _ => continue,
        }
    }

    Some(Structure {
        size_x,
        size_y,
        size_z,
        blocks,
    })
}

/// Load Bedrock structure format (structure.block_indices + palette).
fn load_bedrock_structure(
    structure: &HashMap<String, mc_rs_nbt::tag::NbtTag>,
    size_x: i32,
    size_y: i32,
    size_z: i32,
    block_mapping: &HashMap<String, u32>,
) -> Option<Structure> {
    // Extract palette
    let palette_compound = match structure.get("palette")? {
        mc_rs_nbt::tag::NbtTag::Compound(c) => c,
        _ => return None,
    };
    let default_palette = match palette_compound.get("default")? {
        mc_rs_nbt::tag::NbtTag::Compound(c) => c,
        _ => return None,
    };
    let block_palette = match default_palette.get("block_palette")? {
        mc_rs_nbt::tag::NbtTag::List(list) => list,
        _ => return None,
    };

    let mut palette = Vec::new();
    for entry in block_palette {
        match entry {
            mc_rs_nbt::tag::NbtTag::Compound(block) => {
                let name = match block.get("name") {
                    Some(mc_rs_nbt::tag::NbtTag::String(s)) => s.as_str(),
                    _ => "minecraft:air",
                };
                palette.push(block_mapping.get(name).copied().unwrap_or(block_ids::AIR));
            }
            _ => palette.push(block_ids::AIR),
        }
    }

    // Extract block indices
    let block_indices = match structure.get("block_indices")? {
        mc_rs_nbt::tag::NbtTag::List(list) => list,
        _ => return None,
    };
    let layer0 = match block_indices.first()? {
        mc_rs_nbt::tag::NbtTag::List(indices) => indices,
        _ => return None,
    };

    let mut blocks = HashMap::new();
    for (i, tag) in layer0.iter().enumerate() {
        let palette_idx = match tag {
            mc_rs_nbt::tag::NbtTag::Int(v) => *v,
            _ => continue,
        };
        if palette_idx < 0 {
            continue;
        }
        let runtime_id = palette
            .get(palette_idx as usize)
            .copied()
            .unwrap_or(block_ids::AIR);
        if runtime_id == block_ids::AIR {
            continue;
        }

        let x = i as i32 / (size_z * size_y);
        let remainder = i as i32 % (size_z * size_y);
        let y = remainder / size_z;
        let z = remainder % size_z;
        blocks.insert((x, y, z), runtime_id);
    }

    Some(Structure {
        size_x,
        size_y,
        size_z,
        blocks,
    })
}

/// Extract 3 ints from a List tag.
fn extract_int_list_3(tag: &mc_rs_nbt::tag::NbtTag) -> Option<(i32, i32, i32)> {
    match tag {
        mc_rs_nbt::tag::NbtTag::List(list) if list.len() == 3 => {
            let a = match &list[0] {
                mc_rs_nbt::tag::NbtTag::Int(v) => *v,
                _ => return None,
            };
            let b = match &list[1] {
                mc_rs_nbt::tag::NbtTag::Int(v) => *v,
                _ => return None,
            };
            let c = match &list[2] {
                mc_rs_nbt::tag::NbtTag::Int(v) => *v,
                _ => return None,
            };
            Some((a, b, c))
        }
        _ => None,
    }
}

/// Build the block name → first runtime ID mapping from canonical_block_states.nbt.
pub fn build_block_mapping() -> HashMap<String, u32> {
    let data = match std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/canonical_block_states.nbt"
    )) {
        Ok(d) => d,
        Err(_) => return HashMap::new(),
    };

    let mut buf = &data[..];
    let mut mapping = HashMap::new();
    let mut index = 0u32;

    while !buf.is_empty() {
        match mc_rs_nbt::read_nbt_network(&mut buf) {
            Ok(root) => {
                if let Some(mc_rs_nbt::tag::NbtTag::String(name)) = root.compound.get("name") {
                    mapping.entry(name.clone()).or_insert(index);
                }
                index += 1;
            }
            Err(_) => break,
        }
    }

    mapping
}

/// Structure definition: which structure, where, how rare.
pub struct StructureDef {
    pub name: &'static str,
    pub path: &'static str,
    pub biomes: &'static [u32],
    pub chance: u32, // 1 in N chunks
}

/// All registered structures.
pub fn get_structure_defs() -> Vec<StructureDef> {
    vec![
        StructureDef {
            name: "fossil_skull_01",
            path: concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/data/structures/fossils/fossil_skull_01.nbt"
            ),
            biomes: &[
                biome_id::DESERT,
                biome_id::DESERT_HILLS,
                biome_id::SWAMPLAND,
            ],
            chance: 64,
        },
        StructureDef {
            name: "fossil_skull_02",
            path: concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/data/structures/fossils/fossil_skull_02.nbt"
            ),
            biomes: &[
                biome_id::DESERT,
                biome_id::DESERT_HILLS,
                biome_id::SWAMPLAND,
            ],
            chance: 64,
        },
        StructureDef {
            name: "fossil_spine_01",
            path: concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/data/structures/fossils/fossil_spine_01.nbt"
            ),
            biomes: &[
                biome_id::DESERT,
                biome_id::DESERT_HILLS,
                biome_id::SWAMPLAND,
            ],
            chance: 64,
        },
        StructureDef {
            name: "fossil_spine_02",
            path: concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/data/structures/fossils/fossil_spine_02.nbt"
            ),
            biomes: &[
                biome_id::DESERT,
                biome_id::DESERT_HILLS,
                biome_id::SWAMPLAND,
            ],
            chance: 64,
        },
    ]
}

/// Deterministic hash for structure placement.
fn structure_hash(chunk_x: i32, chunk_z: i32, seed: u64, structure_idx: u32) -> u64 {
    let h = (chunk_x as u64).wrapping_mul(341873128712)
        ^ (chunk_z as u64).wrapping_mul(132897987541)
        ^ seed
        ^ (structure_idx as u64).wrapping_mul(1000000007);
    h.wrapping_mul(h.wrapping_add(223))
}

/// Generate structure blocks for a chunk.
/// Returns a map of (local_x, world_y, local_z) -> runtime block ID.
pub fn generate_structures(
    chunk_x: i32,
    chunk_z: i32,
    seed: u64,
    center_biome: u32,
    surfaces: &[[i32; 16]; 16],
    block_mapping: &HashMap<String, u32>,
) -> HashMap<(u8, i32, u8), u32> {
    let mut blocks = HashMap::new();
    let defs = get_structure_defs();

    for (idx, def) in defs.iter().enumerate() {
        // Check biome
        if !def.biomes.contains(&center_biome) {
            continue;
        }

        // Check chance
        let hash = structure_hash(chunk_x, chunk_z, seed, idx as u32);
        if !hash.is_multiple_of(def.chance as u64) {
            continue;
        }

        // Load structure
        let structure = match load_structure(def.path, block_mapping) {
            Some(s) => s,
            None => continue,
        };

        // Place at chunk center, at surface level
        let center_surface = surfaces[8][8];
        // Structures like fossils are placed underground
        let place_y = if def.name.contains("fossil") {
            // Underground: random depth between 15 and 40
            let mut rng = Random::new(hash as i64);
            rng.next_range(15, 40)
        } else {
            center_surface + 1
        };

        // Place structure blocks (centered in chunk)
        let offset_x = (16 - structure.size_x) / 2;
        let offset_z = (16 - structure.size_z) / 2;

        for (&(sx, sy, sz), &block_id) in &structure.blocks {
            let world_x = offset_x + sx;
            let world_z = offset_z + sz;
            let world_y = place_y + sy;

            if (0..16).contains(&world_x) && (0..16).contains(&world_z) && world_y > 0 {
                blocks.insert((world_x as u8, world_y, world_z as u8), block_id);
            }
        }
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_block_mapping() {
        let mapping = build_block_mapping();
        assert!(!mapping.is_empty(), "Block mapping should not be empty");
        assert!(
            mapping.contains_key("minecraft:stone"),
            "Should contain stone"
        );
        assert!(mapping.contains_key("minecraft:air"), "Should contain air");
        assert!(
            mapping.contains_key("minecraft:bone_block"),
            "Should contain bone_block (for fossils)"
        );
    }

    #[test]
    fn test_load_fossil() {
        let mapping = build_block_mapping();
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/data/structures/fossils/fossil_skull_04.nbt"
        );
        let structure = load_structure(path, &mapping);
        assert!(structure.is_some(), "Should load fossil structure");

        let s = structure.unwrap();
        assert!(
            s.size_x > 0 && s.size_y > 0 && s.size_z > 0,
            "Size should be positive"
        );
        assert!(!s.blocks.is_empty(), "Should have blocks");

        eprintln!(
            "Fossil skull 04: {}x{}x{}, {} blocks",
            s.size_x,
            s.size_y,
            s.size_z,
            s.blocks.len()
        );
    }

    #[test]
    fn test_structure_placement_deterministic() {
        let mapping = build_block_mapping();
        let surfaces = [[65i32; 16]; 16];

        let blocks1 = generate_structures(0, 0, 42, biome_id::DESERT, &surfaces, &mapping);
        let blocks2 = generate_structures(0, 0, 42, biome_id::DESERT, &surfaces, &mapping);

        assert_eq!(
            blocks1.len(),
            blocks2.len(),
            "Placement should be deterministic"
        );
    }

    #[test]
    fn test_no_structures_in_wrong_biome() {
        let mapping = build_block_mapping();
        let surfaces = [[65i32; 16]; 16];

        // Forest should not have fossils
        let blocks = generate_structures(0, 0, 42, biome_id::FOREST, &surfaces, &mapping);
        assert!(
            blocks.is_empty(),
            "Forest should not have fossil structures"
        );
    }
}
