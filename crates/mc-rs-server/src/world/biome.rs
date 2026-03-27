use super::block_registry::BLOCKS;
use super::noise::Simplex;
use super::random::Random;

/// Biome IDs matching Bedrock Edition.
pub mod biome_id {
    pub const OCEAN: u32 = 0;
    pub const PLAINS: u32 = 1;
    pub const DESERT: u32 = 2;
    pub const EXTREME_HILLS: u32 = 3;
    pub const FOREST: u32 = 4;
    pub const TAIGA: u32 = 5;
    pub const SWAMPLAND: u32 = 6;
    pub const RIVER: u32 = 7;
    pub const FROZEN_RIVER: u32 = 11;
    pub const ICE_PLAINS: u32 = 12;
    pub const ICE_MOUNTAINS: u32 = 13;
    pub const MUSHROOM_ISLAND: u32 = 14;
    pub const MUSHROOM_ISLAND_SHORE: u32 = 15;
    pub const BEACH: u32 = 16;
    pub const DESERT_HILLS: u32 = 17;
    pub const FOREST_HILLS: u32 = 18;
    pub const TAIGA_HILLS: u32 = 19;
    pub const EXTREME_HILLS_EDGE: u32 = 20;
    pub const JUNGLE: u32 = 21;
    pub const JUNGLE_HILLS: u32 = 22;
    pub const JUNGLE_EDGE: u32 = 23;
    pub const DEEP_OCEAN: u32 = 24;
    pub const STONE_BEACH: u32 = 25;
    pub const COLD_BEACH: u32 = 26;
    pub const BIRCH_FOREST: u32 = 27;
    pub const BIRCH_FOREST_HILLS: u32 = 28;
    pub const ROOFED_FOREST: u32 = 29;
    pub const COLD_TAIGA: u32 = 30;
    pub const COLD_TAIGA_HILLS: u32 = 31;
    pub const MEGA_TAIGA: u32 = 32;
    pub const MEGA_TAIGA_HILLS: u32 = 33;
    pub const EXTREME_HILLS_PLUS_TREES: u32 = 34;
    pub const SAVANNA: u32 = 35;
    pub const SAVANNA_PLATEAU: u32 = 36;
    pub const MESA: u32 = 37;
    pub const MESA_PLATEAU_STONE: u32 = 38;
    pub const MESA_PLATEAU: u32 = 39;
    pub const WARM_OCEAN: u32 = 40;
    pub const DEEP_WARM_OCEAN: u32 = 41;
    pub const LUKEWARM_OCEAN: u32 = 42;
    pub const DEEP_LUKEWARM_OCEAN: u32 = 43;
    pub const COLD_OCEAN: u32 = 44;
    pub const DEEP_COLD_OCEAN: u32 = 45;
    pub const FROZEN_OCEAN: u32 = 46;
    pub const DEEP_FROZEN_OCEAN: u32 = 47;
    pub const BAMBOO_JUNGLE: u32 = 48;
    pub const SUNFLOWER_PLAINS: u32 = 129;
    pub const ICE_PLAINS_SPIKES: u32 = 140;
    pub const DESERT_MUTATED: u32 = 130;
    pub const EXTREME_HILLS_MUTATED: u32 = 131;
    pub const FLOWER_FOREST: u32 = 132;
    pub const TAIGA_MUTATED: u32 = 133;
    pub const SWAMPLAND_MUTATED: u32 = 134;
    pub const JUNGLE_MUTATED: u32 = 149;
    pub const JUNGLE_EDGE_MUTATED: u32 = 151;
    pub const BIRCH_FOREST_MUTATED: u32 = 155;
    pub const BIRCH_FOREST_HILLS_MUTATED: u32 = 156;
    pub const ROOFED_FOREST_MUTATED: u32 = 157;
    pub const COLD_TAIGA_MUTATED: u32 = 158;
    pub const REDWOOD_TAIGA_MUTATED: u32 = 160;
    pub const REDWOOD_TAIGA_HILLS_MUTATED: u32 = 161;
    pub const EXTREME_HILLS_PLUS_TREES_MUTATED: u32 = 162;
    pub const SAVANNA_MUTATED: u32 = 163;
    pub const SAVANNA_PLATEAU_MUTATED: u32 = 164;
    pub const MESA_BRYCE: u32 = 165;
    pub const MESA_PLATEAU_STONE_MUTATED: u32 = 166;
    pub const MESA_PLATEAU_MUTATED: u32 = 167;
    pub const BAMBOO_JUNGLE_HILLS: u32 = 169;
}

/// Biome definition with elevation range and ground cover.
#[derive(Debug, Clone)]
pub struct BiomeDef {
    pub id: u32,
    pub min_elevation: f64,
    pub max_elevation: f64,
    pub ground_cover: Vec<u32>,
}

pub fn biome_name(id: u32) -> &'static str {
    match id {
        biome_id::OCEAN => "Ocean",
        biome_id::PLAINS => "Plains",
        biome_id::DESERT => "Desert",
        biome_id::EXTREME_HILLS => "Extreme Hills",
        biome_id::FOREST => "Forest",
        biome_id::TAIGA => "Taiga",
        biome_id::SWAMPLAND => "Swampland",
        biome_id::RIVER => "River",
        biome_id::FROZEN_RIVER => "Frozen River",
        biome_id::ICE_PLAINS => "Ice Plains",
        biome_id::ICE_MOUNTAINS => "Ice Mountains",
        biome_id::MUSHROOM_ISLAND => "Mushroom Island",
        biome_id::MUSHROOM_ISLAND_SHORE => "Mushroom Island Shore",
        biome_id::BEACH => "Beach",
        biome_id::DESERT_HILLS => "Desert Hills",
        biome_id::FOREST_HILLS => "Forest Hills",
        biome_id::TAIGA_HILLS => "Taiga Hills",
        biome_id::EXTREME_HILLS_EDGE => "Extreme Hills Edge",
        biome_id::JUNGLE => "Jungle",
        biome_id::JUNGLE_HILLS => "Jungle Hills",
        biome_id::JUNGLE_EDGE => "Jungle Edge",
        biome_id::DEEP_OCEAN => "Deep Ocean",
        biome_id::STONE_BEACH => "Stone Beach",
        biome_id::COLD_BEACH => "Cold Beach",
        biome_id::BIRCH_FOREST => "Birch Forest",
        biome_id::BIRCH_FOREST_HILLS => "Birch Forest Hills",
        biome_id::ROOFED_FOREST => "Roofed Forest",
        biome_id::COLD_TAIGA => "Cold Taiga",
        biome_id::COLD_TAIGA_HILLS => "Cold Taiga Hills",
        biome_id::MEGA_TAIGA => "Mega Taiga",
        biome_id::MEGA_TAIGA_HILLS => "Mega Taiga Hills",
        biome_id::EXTREME_HILLS_PLUS_TREES => "Extreme Hills+",
        biome_id::SAVANNA => "Savanna",
        biome_id::SAVANNA_PLATEAU => "Savanna Plateau",
        biome_id::MESA => "Mesa",
        biome_id::MESA_PLATEAU_STONE => "Mesa Plateau Stone",
        biome_id::MESA_PLATEAU => "Mesa Plateau",
        biome_id::WARM_OCEAN => "Warm Ocean",
        biome_id::DEEP_WARM_OCEAN => "Deep Warm Ocean",
        biome_id::LUKEWARM_OCEAN => "Lukewarm Ocean",
        biome_id::DEEP_LUKEWARM_OCEAN => "Deep Lukewarm Ocean",
        biome_id::COLD_OCEAN => "Cold Ocean",
        biome_id::DEEP_COLD_OCEAN => "Deep Cold Ocean",
        biome_id::FROZEN_OCEAN => "Frozen Ocean",
        biome_id::DEEP_FROZEN_OCEAN => "Deep Frozen Ocean",
        biome_id::BAMBOO_JUNGLE => "Bamboo Jungle",
        biome_id::SUNFLOWER_PLAINS => "Sunflower Plains",
        biome_id::ICE_PLAINS_SPIKES => "Ice Plains Spikes",
        biome_id::DESERT_MUTATED => "Desert M",
        biome_id::EXTREME_HILLS_MUTATED => "Extreme Hills M",
        biome_id::FLOWER_FOREST => "Flower Forest",
        biome_id::TAIGA_MUTATED => "Taiga M",
        biome_id::SWAMPLAND_MUTATED => "Swampland M",
        biome_id::JUNGLE_MUTATED => "Jungle M",
        biome_id::JUNGLE_EDGE_MUTATED => "Jungle Edge M",
        biome_id::BIRCH_FOREST_MUTATED => "Birch Forest M",
        biome_id::BIRCH_FOREST_HILLS_MUTATED => "Birch Forest Hills M",
        biome_id::ROOFED_FOREST_MUTATED => "Roofed Forest M",
        biome_id::COLD_TAIGA_MUTATED => "Cold Taiga M",
        biome_id::REDWOOD_TAIGA_MUTATED => "Redwood Taiga M",
        biome_id::REDWOOD_TAIGA_HILLS_MUTATED => "Redwood Taiga Hills M",
        biome_id::EXTREME_HILLS_PLUS_TREES_MUTATED => "Extreme Hills+ M",
        biome_id::SAVANNA_MUTATED => "Savanna M",
        biome_id::SAVANNA_PLATEAU_MUTATED => "Savanna Plateau M",
        biome_id::MESA_BRYCE => "Mesa Bryce",
        biome_id::MESA_PLATEAU_STONE_MUTATED => "Mesa Plateau Stone M",
        biome_id::MESA_PLATEAU_MUTATED => "Mesa Plateau M",
        biome_id::BAMBOO_JUNGLE_HILLS => "Bamboo Jungle Hills",
        _ => "Unknown",
    }
}

/// Convert noise_type string to (min_elevation, max_elevation).
/// Based on BDS biome JSON files + BetterVanillaGenerator depth/scale mapping.
/// min_elevation = 62 + depth * 17, max_elevation = min_elevation + scale * 35
fn noise_type_to_elevation(noise_type: &str) -> (f64, f64) {
    let (depth, scale) = match noise_type {
        "lowlands" => (0.125, 0.05),
        "default" => (0.1, 0.2),
        "default_mutated" => (0.2, 0.2),
        "taiga" => (0.2, 0.2),
        "mountains" | "hills" => (0.45, 0.3),
        "extreme" => (1.0, 0.5),
        "less_extreme" => (0.2, 0.4),
        "highlands" => (1.5, 0.025),
        "beach" => (0.0, 0.025),
        "stone_beach" => (0.1, 0.8),
        "ocean" => (-1.0, 0.1),
        "deep_ocean" => (-1.8, 0.1),
        "river" => (-0.5, 0.0),
        "swamp" => (-0.2, 0.1),
        "mushroom" => (0.2, 0.3),
        _ => (0.1, 0.2), // default
    };
    // Convert depth/scale to world Y elevations
    let base = 62.0;
    let min_el = base + depth * 17.0;
    let max_el = min_el + scale * 35.0 + 4.0; // +4 minimum range
    (min_el, max_el)
}

/// Convert raw noise_params [depth, scale] to elevations.
fn noise_params_to_elevation(depth: f64, scale: f64) -> (f64, f64) {
    let base = 62.0;
    let min_el = base + depth * 17.0;
    let max_el = min_el + scale * 35.0 + 4.0;
    (min_el, max_el)
}

/// Grassy cover: grass, dirt x2 (median depth = 3)
fn grassy() -> Vec<u32> {
    vec![BLOCKS.grass_block, BLOCKS.dirt, BLOCKS.dirt]
}

/// Snowy cover: snow_layer, grass, dirt
fn snowy() -> Vec<u32> {
    vec![BLOCKS.snow_layer, BLOCKS.grass_block, BLOCKS.dirt]
}

/// Sandy cover for beach: sand x3
fn sandy() -> Vec<u32> {
    vec![BLOCKS.sand, BLOCKS.sand, BLOCKS.sand]
}

/// Desert cover: sand x3 + sandstone x8 (real Bedrock: 8-12 blocks of sandstone)
fn desert_cover() -> Vec<u32> {
    vec![
        BLOCKS.sand,
        BLOCKS.sand,
        BLOCKS.sand,
        BLOCKS.sandstone,
        BLOCKS.sandstone,
        BLOCKS.sandstone,
        BLOCKS.sandstone,
        BLOCKS.sandstone,
        BLOCKS.sandstone,
        BLOCKS.sandstone,
        BLOCKS.sandstone,
    ]
}

/// Mesa cover: red_sand x2, hardened_clay x4
fn mesa() -> Vec<u32> {
    vec![
        BLOCKS.red_sand,
        BLOCKS.red_sand,
        BLOCKS.hardened_clay,
        BLOCKS.hardened_clay,
        BLOCKS.hardened_clay,
        BLOCKS.hardened_clay,
    ]
}

/// Gravel cover (ocean floor): 3 blocks
fn gravelly() -> Vec<u32> {
    vec![BLOCKS.gravel, BLOCKS.gravel, BLOCKS.gravel]
}

/// Dirt cover (river bed)
fn dirty() -> Vec<u32> {
    vec![BLOCKS.dirt, BLOCKS.dirt, BLOCKS.dirt]
}

/// Mushroom cover
fn mushroom() -> Vec<u32> {
    vec![BLOCKS.mycelium, BLOCKS.dirt, BLOCKS.dirt]
}

/// Podzol cover (mega taiga)
fn podzol() -> Vec<u32> {
    vec![BLOCKS.podzol, BLOCKS.dirt, BLOCKS.dirt]
}

/// Stone cover (stone beach)
fn stony() -> Vec<u32> {
    vec![BLOCKS.stone, BLOCKS.stone, BLOCKS.stone]
}

/// Snow block cover (ice plains spikes)
fn snow_block() -> Vec<u32> {
    vec![BLOCKS.snow_block, BLOCKS.snow_block, BLOCKS.dirt]
}

/// Get a biome definition by ID.
pub fn get_biome(id: u32) -> BiomeDef {
    match id {
        // ── Oceans ──
        biome_id::OCEAN
        | biome_id::WARM_OCEAN
        | biome_id::LUKEWARM_OCEAN
        | biome_id::COLD_OCEAN
        | biome_id::FROZEN_OCEAN => {
            let (min, max) = noise_type_to_elevation("ocean");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: gravelly(),
            }
        }
        biome_id::DEEP_OCEAN
        | biome_id::DEEP_WARM_OCEAN
        | biome_id::DEEP_LUKEWARM_OCEAN
        | biome_id::DEEP_COLD_OCEAN
        | biome_id::DEEP_FROZEN_OCEAN => {
            let (min, max) = noise_type_to_elevation("deep_ocean");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: gravelly(),
            }
        }

        // ── Plains ──
        biome_id::PLAINS | biome_id::SUNFLOWER_PLAINS => {
            let (min, max) = noise_type_to_elevation("lowlands");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }

        // ── Desert ──
        biome_id::DESERT => {
            let (min, max) = noise_type_to_elevation("lowlands");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: desert_cover(),
            }
        }
        biome_id::DESERT_HILLS => {
            let (min, max) = noise_type_to_elevation("mountains");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: desert_cover(),
            }
        }

        // ── Forest ──
        biome_id::FOREST | biome_id::ROOFED_FOREST => {
            let (min, max) = noise_type_to_elevation("default");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::FOREST_HILLS => {
            let (min, max) = noise_type_to_elevation("mountains");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::FLOWER_FOREST => {
            let (min, max) = noise_params_to_elevation(0.1, 0.4);
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::BIRCH_FOREST => {
            let (min, max) = noise_type_to_elevation("default");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::BIRCH_FOREST_HILLS => {
            let (min, max) = noise_type_to_elevation("mountains");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }

        // ── Taiga ──
        biome_id::TAIGA => {
            let (min, max) = noise_type_to_elevation("default");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::TAIGA_HILLS | biome_id::COLD_TAIGA_HILLS => {
            let (min, max) = noise_type_to_elevation("mountains");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::COLD_TAIGA => {
            let (min, max) = noise_type_to_elevation("taiga");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: snowy(),
            }
        }
        biome_id::MEGA_TAIGA => {
            let (min, max) = noise_type_to_elevation("taiga");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: podzol(),
            }
        }
        biome_id::MEGA_TAIGA_HILLS => {
            let (min, max) = noise_type_to_elevation("mountains");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: podzol(),
            }
        }

        // ── Jungle ──
        biome_id::JUNGLE | biome_id::JUNGLE_EDGE | biome_id::BAMBOO_JUNGLE => {
            let (min, max) = noise_type_to_elevation("default");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::JUNGLE_HILLS => {
            let (min, max) = noise_type_to_elevation("mountains");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }

        // ── Mountains ──
        biome_id::EXTREME_HILLS | biome_id::EXTREME_HILLS_PLUS_TREES => {
            let (min, max) = noise_type_to_elevation("extreme");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::EXTREME_HILLS_EDGE => {
            let (min, max) = noise_type_to_elevation("less_extreme");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }

        // ── Savanna ──
        biome_id::SAVANNA => {
            let (min, max) = noise_type_to_elevation("lowlands");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::SAVANNA_PLATEAU => {
            let (min, max) = noise_type_to_elevation("highlands");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }

        // ── Mesa / Badlands ──
        biome_id::MESA => {
            let (min, max) = noise_type_to_elevation("default");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: mesa(),
            }
        }
        biome_id::MESA_PLATEAU | biome_id::MESA_PLATEAU_STONE => {
            let (min, max) = noise_type_to_elevation("highlands");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: mesa(),
            }
        }

        // ── Swamp ──
        biome_id::SWAMPLAND => {
            let (min, max) = noise_type_to_elevation("swamp");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }

        // ── River ──
        biome_id::RIVER => {
            let (min, max) = noise_type_to_elevation("river");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: dirty(),
            }
        }

        // ── Beach ──
        biome_id::BEACH | biome_id::COLD_BEACH => {
            let (min, max) = noise_type_to_elevation("beach");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: sandy(),
            }
        }
        biome_id::STONE_BEACH => {
            let (min, max) = noise_type_to_elevation("stone_beach");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: stony(),
            }
        }

        // ── Mushroom ──
        biome_id::MUSHROOM_ISLAND => {
            let (min, max) = noise_type_to_elevation("mushroom");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: mushroom(),
            }
        }
        biome_id::MUSHROOM_ISLAND_SHORE => {
            let (min, max) = noise_type_to_elevation("beach");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: mushroom(),
            }
        }

        // ── Ice ──
        biome_id::ICE_PLAINS => {
            let (min, max) = noise_type_to_elevation("lowlands");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: snowy(),
            }
        }
        biome_id::ICE_PLAINS_SPIKES => {
            let (min, max) = noise_params_to_elevation(0.425, 0.45);
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: snow_block(),
            }
        }

        // ── Ice Mountains ──
        biome_id::ICE_MOUNTAINS => {
            let (min, max) = noise_type_to_elevation("mountains");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: snowy(),
            }
        }

        // ── Frozen River ──
        biome_id::FROZEN_RIVER => {
            let (min, max) = noise_type_to_elevation("river");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: snowy(),
            }
        }

        // ── Savanna Mutated (extreme terrain) ──
        biome_id::SAVANNA_MUTATED => {
            let (min, max) = noise_params_to_elevation(0.3625, 1.225);
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }

        // ── Mesa Bryce ──
        biome_id::MESA_BRYCE => {
            let (min, max) = noise_type_to_elevation("default");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: mesa(),
            }
        }
        biome_id::MESA_PLATEAU_MUTATED | biome_id::MESA_PLATEAU_STONE_MUTATED => {
            let (min, max) = noise_type_to_elevation("mountains");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: mesa(),
            }
        }

        // ── Mutated variants ──
        biome_id::DESERT_MUTATED => {
            let (min, max) = noise_params_to_elevation(0.225, 0.25);
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: desert_cover(),
            }
        }
        biome_id::EXTREME_HILLS_MUTATED | biome_id::EXTREME_HILLS_PLUS_TREES_MUTATED => {
            let (min, max) = noise_type_to_elevation("extreme");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::TAIGA_MUTATED => {
            let (min, max) = noise_type_to_elevation("default_mutated");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::SWAMPLAND_MUTATED => {
            let (min, max) = noise_params_to_elevation(-0.1, 0.3);
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::JUNGLE_MUTATED => {
            let (min, max) = noise_type_to_elevation("default_mutated");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::JUNGLE_EDGE_MUTATED => {
            let (min, max) = noise_type_to_elevation("default_mutated");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::BIRCH_FOREST_MUTATED => {
            let (min, max) = noise_type_to_elevation("default_mutated");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::BIRCH_FOREST_HILLS_MUTATED => {
            let (min, max) = noise_params_to_elevation(0.55, 0.5);
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::ROOFED_FOREST_MUTATED => {
            let (min, max) = noise_type_to_elevation("default_mutated");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::COLD_TAIGA_MUTATED => {
            let (min, max) = noise_params_to_elevation(0.3, 0.4);
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: snowy(),
            }
        }
        biome_id::REDWOOD_TAIGA_MUTATED => {
            let (min, max) = noise_type_to_elevation("taiga");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: podzol(),
            }
        }
        biome_id::REDWOOD_TAIGA_HILLS_MUTATED => {
            let (min, max) = noise_params_to_elevation(0.55, 0.5);
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: podzol(),
            }
        }
        biome_id::SAVANNA_PLATEAU_MUTATED => {
            let (min, max) = noise_params_to_elevation(1.05, 1.2125);
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
        biome_id::BAMBOO_JUNGLE_HILLS => {
            let (min, max) = noise_type_to_elevation("mountains");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }

        // ── Default (unknown biomes) ──
        _ => {
            let (min, max) = noise_type_to_elevation("default");
            BiomeDef {
                id: biome_id::PLAINS,
                min_elevation: min,
                max_elevation: max,
                ground_cover: grassy(),
            }
        }
    }
}

/// Biome selector using temperature and rainfall noise.
pub struct BiomeSelector {
    temperature: Simplex,
    rainfall: Simplex,
    map: Vec<u32>,
}

impl BiomeSelector {
    pub fn new(random: &mut Random) -> Self {
        // Lower expansion = larger biomes. 1/4096 gives biomes ~500-1000 blocks wide (vanilla-like)
        let temperature = Simplex::new(random, 2, 1.0 / 16.0, 1.0 / 4096.0);
        let rainfall = Simplex::new(random, 2, 1.0 / 16.0, 1.0 / 4096.0);

        let mut map = vec![biome_id::OCEAN; 64 * 64];
        for i in 0..64 {
            for j in 0..64 {
                let temp = i as f64 / 63.0;
                let rain = j as f64 / 63.0;
                map[i + (j << 6)] = lookup_biome(temp, rain);
            }
        }

        Self {
            temperature,
            rainfall,
            map,
        }
    }

    fn get_temperature(&self, x: f64, z: f64) -> f64 {
        (self.temperature.noise_2d(x, z, true) + 1.0) / 2.0
    }

    fn get_rainfall(&self, x: f64, z: f64) -> f64 {
        (self.rainfall.noise_2d(x, z, true) + 1.0) / 2.0
    }

    pub fn climate_at(&self, x: f64, z: f64) -> (f64, f64) {
        (self.get_temperature(x, z), self.get_rainfall(x, z))
    }

    pub fn pick_biome(&self, x: f64, z: f64) -> u32 {
        let temperature = ((self.get_temperature(x, z) * 63.0) as usize).min(63);
        let rainfall = ((self.get_rainfall(x, z) * 63.0) as usize).min(63);
        self.map[temperature + (rainfall << 6)]
    }
}

/// Full biome lookup covering all 60+ overworld biomes.
/// Maps temperature (0..1) x rainfall (0..1) to biome ID.
/// Temperature: 0=frozen, 0.5=temperate, 1=hot
/// Rainfall: 0=dry/ocean, 0.5=moderate, 1=wet/mountains
/// Biome lookup — common biomes dominate, rare biomes at extremes only.
/// Noise values cluster around 0.5 (Gaussian distribution), so the center
/// of the grid gets the most area. Rare biomes go to the edges.
fn lookup_biome(temperature: f64, rainfall: f64) -> u32 {
    // Oceans: very low rainfall (< 0.15)
    if rainfall < 0.08 {
        if temperature < 0.3 {
            biome_id::DEEP_FROZEN_OCEAN
        } else if temperature < 0.7 {
            biome_id::DEEP_OCEAN
        } else {
            biome_id::DEEP_WARM_OCEAN
        }
    } else if rainfall < 0.15 {
        if temperature < 0.2 {
            biome_id::FROZEN_OCEAN
        } else if temperature < 0.4 {
            biome_id::COLD_OCEAN
        } else if temperature < 0.6 {
            biome_id::OCEAN
        } else if temperature < 0.8 {
            biome_id::LUKEWARM_OCEAN
        } else {
            biome_id::WARM_OCEAN
        }

    // Coastal: low rainfall (0.15 - 0.25)
    } else if rainfall < 0.25 {
        if temperature < 0.15 {
            biome_id::COLD_BEACH
        } else if temperature < 0.4 {
            biome_id::BEACH
        } else if temperature < 0.7 {
            biome_id::RIVER
        } else if temperature < 0.9 {
            biome_id::SWAMPLAND
        } else {
            biome_id::MUSHROOM_ISLAND_SHORE
        }

    // ── COMMON BIOMES: rainfall 0.25 - 0.80 (biggest area) ──

    // Cold biomes: low temperature
    } else if temperature < 0.20 {
        if rainfall < 0.45 {
            biome_id::ICE_PLAINS
        } else if rainfall < 0.60 {
            biome_id::COLD_TAIGA
        } else if rainfall < 0.75 {
            biome_id::TAIGA
        } else {
            biome_id::MEGA_TAIGA
        }

    // Temperate: moderate temperature (0.20 - 0.50) — LARGEST AREA
    } else if temperature < 0.50 {
        if rainfall < 0.40 {
            biome_id::PLAINS
        } else if rainfall < 0.55 {
            biome_id::FOREST
        } else if rainfall < 0.65 {
            biome_id::BIRCH_FOREST
        } else if rainfall < 0.80 {
            biome_id::ROOFED_FOREST
        } else {
            biome_id::EXTREME_HILLS
        }

    // Warm: high temperature (0.50 - 0.80)
    } else if temperature < 0.80 {
        if rainfall < 0.35 {
            biome_id::SUNFLOWER_PLAINS
        } else if rainfall < 0.50 {
            biome_id::SAVANNA
        } else if rainfall < 0.65 {
            biome_id::FOREST_HILLS
        } else if rainfall < 0.80 {
            biome_id::JUNGLE
        } else {
            biome_id::BAMBOO_JUNGLE
        }

    // Hot: very high temperature (0.80 - 1.0)
    } else if rainfall < 0.40 {
        biome_id::DESERT
    } else if rainfall < 0.55 {
        biome_id::SAVANNA_PLATEAU
    } else if rainfall < 0.70 {
        biome_id::DESERT_HILLS
    } else if rainfall < 0.85 {
        biome_id::JUNGLE_HILLS
    } else {
        biome_id::MESA
    }
}

/// 1D Gaussian kernel for elevation smoothing.
pub struct Gaussian {
    pub smooth_size: usize,
    pub kernel_1d: Vec<f64>,
    pub weight_sum_1d: f64,
}

impl Gaussian {
    pub fn new(smooth_size: usize) -> Self {
        let bell_size = 1.0 / smooth_size as f64;
        let bell_height = 2.0 * smooth_size as f64;
        let kernel_size = smooth_size * 2 + 1;
        let mut kernel_1d = vec![0.0; kernel_size];

        for (sx, item) in kernel_1d.iter_mut().enumerate().take(kernel_size) {
            let offset = sx as i32 - smooth_size as i32;
            let bx = bell_size * offset as f64;
            *item = bell_height.sqrt() * (-bx * bx / 2.0).exp();
        }

        let weight_sum_1d: f64 = kernel_1d.iter().sum();
        Self {
            smooth_size,
            kernel_1d,
            weight_sum_1d,
        }
    }
}

/// Pick a biome with noise jitter.
pub fn pick_biome_with_jitter(selector: &BiomeSelector, x: i32, z: i32, seed: u64) -> u32 {
    let hash = (x as i64 * 2345803) ^ (z as i64 * 9236449) ^ seed as i64;
    let hash = hash.wrapping_mul(hash.wrapping_add(223));
    let x_noise = (hash >> 20) & 3;
    let z_noise = (hash >> 22) & 3;
    let x_noise = if x_noise == 3 { 1 } else { x_noise };
    let z_noise = if z_noise == 3 { 1 } else { z_noise };

    selector.pick_biome(
        (x as i64 + x_noise - 1) as f64,
        (z as i64 + z_noise - 1) as f64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biome_lookup_variety() {
        let mut biomes = std::collections::HashSet::new();
        for i in 0..64 {
            for j in 0..64 {
                biomes.insert(lookup_biome(i as f64 / 63.0, j as f64 / 63.0));
            }
        }
        assert!(
            biomes.len() >= 15,
            "Expected at least 15 biomes, got {}: {:?}",
            biomes.len(),
            biomes
        );
    }

    #[test]
    fn test_biome_selector_deterministic() {
        let mut rng1 = Random::new(42);
        let sel1 = BiomeSelector::new(&mut rng1);
        let mut rng2 = Random::new(42);
        let sel2 = BiomeSelector::new(&mut rng2);

        for x in 0..10 {
            for z in 0..10 {
                assert_eq!(
                    sel1.pick_biome(x as f64 * 16.0, z as f64 * 16.0),
                    sel2.pick_biome(x as f64 * 16.0, z as f64 * 16.0),
                );
            }
        }
    }

    #[test]
    fn test_noise_type_elevations() {
        let (min, max) = noise_type_to_elevation("ocean");
        assert!(min < 60.0, "Ocean should be below sea level");
        assert!(max < 70.0);

        let (min, max) = noise_type_to_elevation("extreme");
        assert!(min > 70.0, "Mountains should be above sea level");
        assert!(max > 90.0, "Mountains should go high");

        let (min, max) = noise_type_to_elevation("deep_ocean");
        assert!(min < 40.0, "Deep ocean should be very low");
    }

    #[test]
    fn test_all_biome_ids_valid() {
        // Test that all biome IDs used in the lookup produce valid BiomeDefs
        let mut rng = Random::new(42);
        let sel = BiomeSelector::new(&mut rng);
        for x in -20..20 {
            for z in -20..20 {
                let id = pick_biome_with_jitter(&sel, x * 32, z * 32, 42);
                let def = get_biome(id);
                assert!(
                    def.min_elevation < def.max_elevation,
                    "Biome {id}: min {} >= max {}",
                    def.min_elevation,
                    def.max_elevation
                );
                assert!(
                    !def.ground_cover.is_empty(),
                    "Biome {id} has no ground cover"
                );
            }
        }
    }

    #[test]
    fn test_gaussian_kernel() {
        let g = Gaussian::new(2);
        assert_eq!(g.kernel_1d.len(), 5);
        assert!(g.weight_sum_1d > 0.0);
        let eps = 1e-10;
        assert!((g.kernel_1d[0] - g.kernel_1d[4]).abs() < eps);
    }
}
