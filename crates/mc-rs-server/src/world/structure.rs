//! Structure loading and placement system.
//! Loads Bedrock Edition .nbt structure files (gzip-compressed NBT LE)
//! and places them in the world during terrain generation.

use std::collections::HashMap;
use std::io::Read;

use super::biome::biome_id;
use super::block_registry::BLOCKS;
use super::block_registry_data::BLOCK_NAME_TO_FIRST_RUNTIME_ID;
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
                let runtime_id = block_mapping.get(name).copied().unwrap_or(BLOCKS.air);
                palette.push(runtime_id);
            }
            _ => palette.push(BLOCKS.air),
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

                let runtime_id = palette.get(state).copied().unwrap_or(BLOCKS.air);
                if runtime_id != BLOCKS.air {
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
                palette.push(block_mapping.get(name).copied().unwrap_or(BLOCKS.air));
            }
            _ => palette.push(BLOCKS.air),
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
            .unwrap_or(BLOCKS.air);
        if runtime_id == BLOCKS.air {
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

/// Build the block name → first runtime ID mapping from generated registry data.
pub fn build_block_mapping() -> HashMap<String, u32> {
    BLOCK_NAME_TO_FIRST_RUNTIME_ID
        .iter()
        .map(|(name, runtime_id)| ((*name).to_string(), *runtime_id))
        .collect()
}

/// How to place a structure vertically.
#[derive(Clone, Copy)]
pub enum Placement {
    /// On the surface (Y = surface + 1)
    Surface,
    /// Underground at random depth
    Underground { min_y: i32, max_y: i32 },
    /// At a fixed Y level
    FixedY(i32),
    /// Underwater (on the ocean floor)
    OceanFloor,
}

/// Structure definition: which structure, where, how rare.
pub struct StructureDef {
    pub name: &'static str,
    pub path: &'static str,
    pub biomes: &'static [u32],
    pub chance: u32, // 1 in N chunks
    pub placement: Placement,
}

/// Helper macro to define structures concisely.
macro_rules! structure {
    ($name:expr, $dir:expr, $file:expr, $biomes:expr, $chance:expr, $placement:expr) => {
        StructureDef {
            name: $name,
            path: concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/data/structures/",
                $dir,
                "/",
                $file
            ),
            biomes: $biomes,
            chance: $chance,
            placement: $placement,
        }
    };
}

// Biome lists for reuse
const DESERT_BIOMES: &[u32] = &[biome_id::DESERT, biome_id::DESERT_HILLS];
const DESERT_SWAMP: &[u32] = &[
    biome_id::DESERT,
    biome_id::DESERT_HILLS,
    biome_id::SWAMPLAND,
];
const OCEAN_BIOMES: &[u32] = &[
    biome_id::OCEAN,
    biome_id::DEEP_OCEAN,
    biome_id::COLD_OCEAN,
    biome_id::DEEP_COLD_OCEAN,
    biome_id::LUKEWARM_OCEAN,
    biome_id::DEEP_LUKEWARM_OCEAN,
    biome_id::WARM_OCEAN,
    biome_id::DEEP_WARM_OCEAN,
    biome_id::FROZEN_OCEAN,
    biome_id::DEEP_FROZEN_OCEAN,
];
const WARM_OCEAN: &[u32] = &[biome_id::WARM_OCEAN, biome_id::DEEP_WARM_OCEAN];
const ICE_BIOMES: &[u32] = &[biome_id::ICE_PLAINS, biome_id::COLD_TAIGA];
const ALL_OVERWORLD: &[u32] = &[
    biome_id::PLAINS,
    biome_id::FOREST,
    biome_id::TAIGA,
    biome_id::DESERT,
    biome_id::SAVANNA,
    biome_id::JUNGLE,
    biome_id::EXTREME_HILLS,
    biome_id::SWAMPLAND,
    biome_id::BIRCH_FOREST,
    biome_id::ROOFED_FOREST,
    biome_id::MEGA_TAIGA,
    biome_id::COLD_TAIGA,
    biome_id::ICE_PLAINS,
    biome_id::BEACH,
    biome_id::OCEAN,
    biome_id::DEEP_OCEAN,
    biome_id::MESA,
];
const PLAINS_BIOMES: &[u32] = &[
    biome_id::PLAINS,
    biome_id::SUNFLOWER_PLAINS,
    biome_id::SAVANNA,
    biome_id::TAIGA,
    biome_id::ICE_PLAINS,
    biome_id::DESERT,
];

/// All registered structures.
#[allow(clippy::vec_init_then_push)]
pub fn get_structure_defs() -> Vec<StructureDef> {
    let ug = Placement::Underground {
        min_y: 15,
        max_y: 40,
    };
    let sf = Placement::Surface;
    let of = Placement::OceanFloor;

    vec![
        // ── Fossils (desert, swamp — underground) ──
        structure!(
            "fossil_skull_01",
            "fossils",
            "fossil_skull_01.nbt",
            DESERT_SWAMP,
            64,
            ug
        ),
        structure!(
            "fossil_skull_02",
            "fossils",
            "fossil_skull_02.nbt",
            DESERT_SWAMP,
            64,
            ug
        ),
        structure!(
            "fossil_skull_03",
            "fossils",
            "fossil_skull_03.nbt",
            DESERT_SWAMP,
            64,
            ug
        ),
        structure!(
            "fossil_skull_04",
            "fossils",
            "fossil_skull_04.nbt",
            DESERT_SWAMP,
            64,
            ug
        ),
        structure!(
            "fossil_spine_01",
            "fossils",
            "fossil_spine_01.nbt",
            DESERT_SWAMP,
            64,
            ug
        ),
        structure!(
            "fossil_spine_02",
            "fossils",
            "fossil_spine_02.nbt",
            DESERT_SWAMP,
            64,
            ug
        ),
        structure!(
            "fossil_spine_03",
            "fossils",
            "fossil_spine_03.nbt",
            DESERT_SWAMP,
            64,
            ug
        ),
        structure!(
            "fossil_spine_04",
            "fossils",
            "fossil_spine_04.nbt",
            DESERT_SWAMP,
            64,
            ug
        ),
        // ── Ocean Ruins (ocean — ocean floor) ──
        // Small cold ruins
        structure!(
            "ruin1_brick",
            "ruin",
            "ruin1_brick.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin1_cracked",
            "ruin",
            "ruin1_cracked.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin1_mossy",
            "ruin",
            "ruin1_mossy.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin2_brick",
            "ruin",
            "ruin2_brick.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin2_cracked",
            "ruin",
            "ruin2_cracked.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin2_mossy",
            "ruin",
            "ruin2_mossy.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin3_brick",
            "ruin",
            "ruin3_brick.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin3_cracked",
            "ruin",
            "ruin3_cracked.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin3_mossy",
            "ruin",
            "ruin3_mossy.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin4_brick",
            "ruin",
            "ruin4_brick.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin4_cracked",
            "ruin",
            "ruin4_cracked.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin4_mossy",
            "ruin",
            "ruin4_mossy.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin5_brick",
            "ruin",
            "ruin5_brick.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin5_cracked",
            "ruin",
            "ruin5_cracked.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin5_mossy",
            "ruin",
            "ruin5_mossy.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin6_brick",
            "ruin",
            "ruin6_brick.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin6_cracked",
            "ruin",
            "ruin6_cracked.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin6_mossy",
            "ruin",
            "ruin6_mossy.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin7_brick",
            "ruin",
            "ruin7_brick.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin7_cracked",
            "ruin",
            "ruin7_cracked.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin7_mossy",
            "ruin",
            "ruin7_mossy.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin8_brick",
            "ruin",
            "ruin8_brick.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin8_cracked",
            "ruin",
            "ruin8_cracked.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        structure!(
            "ruin8_mossy",
            "ruin",
            "ruin8_mossy.nbt",
            OCEAN_BIOMES,
            24,
            of
        ),
        // Big cold ruins
        structure!(
            "big_ruin1_brick",
            "ruin",
            "big_ruin1_brick.nbt",
            OCEAN_BIOMES,
            48,
            of
        ),
        structure!(
            "big_ruin1_cracked",
            "ruin",
            "big_ruin1_cracked.nbt",
            OCEAN_BIOMES,
            48,
            of
        ),
        structure!(
            "big_ruin1_mossy",
            "ruin",
            "big_ruin1_mossy.nbt",
            OCEAN_BIOMES,
            48,
            of
        ),
        structure!(
            "big_ruin2_brick",
            "ruin",
            "big_ruin2_brick.nbt",
            OCEAN_BIOMES,
            48,
            of
        ),
        structure!(
            "big_ruin2_cracked",
            "ruin",
            "big_ruin2_cracked.nbt",
            OCEAN_BIOMES,
            48,
            of
        ),
        structure!(
            "big_ruin2_mossy",
            "ruin",
            "big_ruin2_mossy.nbt",
            OCEAN_BIOMES,
            48,
            of
        ),
        structure!(
            "big_ruin3_brick",
            "ruin",
            "big_ruin3_brick.nbt",
            OCEAN_BIOMES,
            48,
            of
        ),
        structure!(
            "big_ruin3_cracked",
            "ruin",
            "big_ruin3_cracked.nbt",
            OCEAN_BIOMES,
            48,
            of
        ),
        structure!(
            "big_ruin3_mossy",
            "ruin",
            "big_ruin3_mossy.nbt",
            OCEAN_BIOMES,
            48,
            of
        ),
        structure!(
            "big_ruin8_brick",
            "ruin",
            "big_ruin8_brick.nbt",
            OCEAN_BIOMES,
            48,
            of
        ),
        structure!(
            "big_ruin8_cracked",
            "ruin",
            "big_ruin8_cracked.nbt",
            OCEAN_BIOMES,
            48,
            of
        ),
        structure!(
            "big_ruin8_mossy",
            "ruin",
            "big_ruin8_mossy.nbt",
            OCEAN_BIOMES,
            48,
            of
        ),
        // Warm ruins
        structure!(
            "big_ruin_warm4",
            "ruin",
            "big_ruin_warm4.nbt",
            WARM_OCEAN,
            48,
            of
        ),
        structure!(
            "big_ruin_warm5",
            "ruin",
            "big_ruin_warm5.nbt",
            WARM_OCEAN,
            48,
            of
        ),
        structure!(
            "big_ruin_warm6",
            "ruin",
            "big_ruin_warm6.nbt",
            WARM_OCEAN,
            48,
            of
        ),
        structure!(
            "big_ruin_warm7",
            "ruin",
            "big_ruin_warm7.nbt",
            WARM_OCEAN,
            48,
            of
        ),
        structure!("ruin_warm1", "ruin", "ruin_warm1.nbt", WARM_OCEAN, 24, of),
        structure!("ruin_warm2", "ruin", "ruin_warm2.nbt", WARM_OCEAN, 24, of),
        structure!("ruin_warm3", "ruin", "ruin_warm3.nbt", WARM_OCEAN, 24, of),
        structure!("ruin_warm4", "ruin", "ruin_warm4.nbt", WARM_OCEAN, 24, of),
        structure!("ruin_warm5", "ruin", "ruin_warm5.nbt", WARM_OCEAN, 24, of),
        structure!("ruin_warm6", "ruin", "ruin_warm6.nbt", WARM_OCEAN, 24, of),
        structure!("ruin_warm7", "ruin", "ruin_warm7.nbt", WARM_OCEAN, 24, of),
        structure!("ruin_warm8", "ruin", "ruin_warm8.nbt", WARM_OCEAN, 24, of),
        // ── Shipwrecks (ocean — ocean floor) ──
        structure!(
            "sw_full",
            "shipwreck",
            "swrightsideupfull.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_full_deg",
            "shipwreck",
            "swrightsideupfulldegraded.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_front",
            "shipwreck",
            "swrightsideupfronthalf.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_front_deg",
            "shipwreck",
            "swrightsideupfronthalfdegraded.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_back",
            "shipwreck",
            "swrightsideupbackhalf.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_back_deg",
            "shipwreck",
            "swrightsideupbackhalfdegraded.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_side_full",
            "shipwreck",
            "swsidewaysfull.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_side_deg",
            "shipwreck",
            "swsidewaysfulldegraded.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_side_front",
            "shipwreck",
            "swsidewaysfronthalf.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_side_front_d",
            "shipwreck",
            "swsidewaysfronthalfdegraded.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_side_back",
            "shipwreck",
            "swsidewaysbackhalf.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_side_back_d",
            "shipwreck",
            "swsidewaysbackhalfdegraded.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_upside_full",
            "shipwreck",
            "swupsidedownfull.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_upside_deg",
            "shipwreck",
            "swupsidedownfulldegraded.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_upside_front",
            "shipwreck",
            "swupsidedownfronthalf.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_upside_fd",
            "shipwreck",
            "swupsidedownfronthalfdegraded.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_upside_back",
            "shipwreck",
            "swupsidedownbackhalf.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_upside_bd",
            "shipwreck",
            "swupsidedownbackhalfdegraded.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_mast",
            "shipwreck",
            "swwithmast.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        structure!(
            "sw_mast_deg",
            "shipwreck",
            "swwithmastdegraded.nbt",
            OCEAN_BIOMES,
            100,
            of
        ),
        // ── Ruined Portals (all biomes — surface) ──
        structure!(
            "portal_1",
            "ruined_portal",
            "portal_1.nbt",
            ALL_OVERWORLD,
            500,
            sf
        ),
        structure!(
            "portal_2",
            "ruined_portal",
            "portal_2.nbt",
            ALL_OVERWORLD,
            500,
            sf
        ),
        structure!(
            "portal_3",
            "ruined_portal",
            "portal_3.nbt",
            ALL_OVERWORLD,
            500,
            sf
        ),
        structure!(
            "portal_4",
            "ruined_portal",
            "portal_4.nbt",
            ALL_OVERWORLD,
            500,
            sf
        ),
        structure!(
            "portal_5",
            "ruined_portal",
            "portal_5.nbt",
            ALL_OVERWORLD,
            500,
            sf
        ),
        structure!(
            "portal_6",
            "ruined_portal",
            "portal_6.nbt",
            ALL_OVERWORLD,
            500,
            sf
        ),
        structure!(
            "portal_7",
            "ruined_portal",
            "portal_7.nbt",
            ALL_OVERWORLD,
            500,
            sf
        ),
        structure!(
            "portal_8",
            "ruined_portal",
            "portal_8.nbt",
            ALL_OVERWORLD,
            500,
            sf
        ),
        structure!(
            "portal_9",
            "ruined_portal",
            "portal_9.nbt",
            ALL_OVERWORLD,
            500,
            sf
        ),
        structure!(
            "portal_10",
            "ruined_portal",
            "portal_10.nbt",
            ALL_OVERWORLD,
            500,
            sf
        ),
        structure!(
            "giant_portal_1",
            "ruined_portal",
            "giant_portal_1.nbt",
            ALL_OVERWORLD,
            1500,
            sf
        ),
        structure!(
            "giant_portal_2",
            "ruined_portal",
            "giant_portal_2.nbt",
            ALL_OVERWORLD,
            1500,
            sf
        ),
        structure!(
            "giant_portal_3",
            "ruined_portal",
            "giant_portal_3.nbt",
            ALL_OVERWORLD,
            1500,
            sf
        ),
        // ── Igloos (ice biomes — surface) ──
        structure!(
            "igloo_top",
            "igloo",
            "igloo_top_trapdoor.nbt",
            ICE_BIOMES,
            300,
            sf
        ),
        // ── Coral (warm ocean — ocean floor) ──
        structure!(
            "coral_crust1",
            "coralcrust",
            "crust1.nbt",
            WARM_OCEAN,
            8,
            of
        ),
        structure!(
            "coral_crust2",
            "coralcrust",
            "crust2.nbt",
            WARM_OCEAN,
            8,
            of
        ),
        structure!(
            "coral_crust3",
            "coralcrust",
            "crust3.nbt",
            WARM_OCEAN,
            8,
            of
        ),
        structure!(
            "coral_crust4",
            "coralcrust",
            "crust4.nbt",
            WARM_OCEAN,
            8,
            of
        ),
        structure!(
            "coral_crust5",
            "coralcrust",
            "crust5.nbt",
            WARM_OCEAN,
            8,
            of
        ),
        structure!(
            "coral_out1",
            "coralcrust",
            "outcropping1.nbt",
            WARM_OCEAN,
            8,
            of
        ),
        structure!(
            "coral_out2",
            "coralcrust",
            "outcropping2.nbt",
            WARM_OCEAN,
            8,
            of
        ),
        structure!(
            "coral_out3",
            "coralcrust",
            "outcropping3.nbt",
            WARM_OCEAN,
            8,
            of
        ),
        structure!(
            "coral_out4",
            "coralcrust",
            "outcropping4.nbt",
            WARM_OCEAN,
            8,
            of
        ),
        structure!(
            "coral_out5",
            "coralcrust",
            "outcropping5.nbt",
            WARM_OCEAN,
            8,
            of
        ),
        structure!(
            "coral_out6",
            "coralcrust",
            "outcropping6.nbt",
            WARM_OCEAN,
            8,
            of
        ),
        // ── Pillager Outpost (plains-like — surface) ──
        structure!(
            "watchtower",
            "pillageroutpost",
            "watchtower.nbt",
            PLAINS_BIOMES,
            800,
            sf
        ),
        structure!(
            "watchtower_og",
            "pillageroutpost",
            "watchtower_overgrown.nbt",
            PLAINS_BIOMES,
            800,
            sf
        ),
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

        // Determine Y placement based on type
        let center_surface = surfaces[8][8];
        let place_y = match def.placement {
            Placement::Surface => center_surface + 1,
            Placement::Underground { min_y, max_y } => {
                let mut rng = Random::new(hash as i64);
                rng.next_range(min_y, max_y)
            }
            Placement::FixedY(y) => y,
            Placement::OceanFloor => {
                // Place on the ocean floor (surface is underwater)
                if center_surface < 60 {
                    center_surface + 1
                } else {
                    continue; // Not underwater, skip
                }
            }
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
