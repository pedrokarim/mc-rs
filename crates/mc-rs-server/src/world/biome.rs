use super::flat_generator::block_ids;
use super::noise::Simplex;
use super::random::Random;
use super::terrain_generator::extra_blocks;

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

/// Grassy cover: grass, dirt x4
fn grassy() -> Vec<u32> {
    vec![
        block_ids::GRASS_BLOCK,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
    ]
}

/// Snowy cover: snow_layer, grass, dirt x3
fn snowy() -> Vec<u32> {
    vec![
        extra_blocks::SNOW_LAYER,
        block_ids::GRASS_BLOCK,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
    ]
}

/// Sandy cover: sand x2, sandstone x3
fn sandy() -> Vec<u32> {
    vec![
        extra_blocks::SAND,
        extra_blocks::SAND,
        extra_blocks::SANDSTONE,
        extra_blocks::SANDSTONE,
        extra_blocks::SANDSTONE,
    ]
}

/// Mesa cover: red_sand, hardened_clay x4
fn mesa() -> Vec<u32> {
    vec![
        extra_blocks::RED_SAND,
        extra_blocks::HARDENED_CLAY,
        extra_blocks::HARDENED_CLAY,
        extra_blocks::HARDENED_CLAY,
        extra_blocks::HARDENED_CLAY,
    ]
}

/// Gravel cover (ocean floor)
fn gravelly() -> Vec<u32> {
    vec![
        extra_blocks::GRAVEL,
        extra_blocks::GRAVEL,
        extra_blocks::GRAVEL,
        extra_blocks::GRAVEL,
        extra_blocks::GRAVEL,
    ]
}

/// Dirt cover (river bed)
fn dirty() -> Vec<u32> {
    vec![
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
    ]
}

/// Mushroom cover
fn mushroom() -> Vec<u32> {
    vec![
        extra_blocks::MYCELIUM,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
    ]
}

/// Podzol cover (mega taiga)
fn podzol() -> Vec<u32> {
    vec![
        extra_blocks::PODZOL,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
    ]
}

/// Stone cover (stone beach)
fn stony() -> Vec<u32> {
    vec![
        extra_blocks::STONE,
        extra_blocks::STONE,
        extra_blocks::STONE,
        extra_blocks::STONE,
        extra_blocks::STONE,
    ]
}

/// Snow block cover (ice plains spikes)
fn snow_block() -> Vec<u32> {
    vec![
        extra_blocks::SNOW_BLOCK,
        extra_blocks::SNOW_BLOCK,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
    ]
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
                ground_cover: sandy(),
            }
        }
        biome_id::DESERT_HILLS => {
            let (min, max) = noise_type_to_elevation("mountains");
            BiomeDef {
                id,
                min_elevation: min,
                max_elevation: max,
                ground_cover: sandy(),
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
                ground_cover: sandy(),
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
        let temperature = Simplex::new(random, 2, 1.0 / 16.0, 1.0 / 512.0);
        let rainfall = Simplex::new(random, 2, 1.0 / 16.0, 1.0 / 512.0);

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
fn lookup_biome(temperature: f64, rainfall: f64) -> u32 {
    if rainfall < 0.10 {
        // Deep oceans
        if temperature < 0.20 {
            biome_id::DEEP_FROZEN_OCEAN
        } else if temperature < 0.40 {
            biome_id::DEEP_COLD_OCEAN
        } else if temperature < 0.60 {
            biome_id::DEEP_OCEAN
        } else if temperature < 0.80 {
            biome_id::DEEP_LUKEWARM_OCEAN
        } else {
            biome_id::DEEP_WARM_OCEAN
        }
    } else if rainfall < 0.19 {
        // Oceans
        if temperature < 0.20 {
            biome_id::FROZEN_OCEAN
        } else if temperature < 0.40 {
            biome_id::COLD_OCEAN
        } else if temperature < 0.60 {
            biome_id::OCEAN
        } else if temperature < 0.80 {
            biome_id::LUKEWARM_OCEAN
        } else {
            biome_id::WARM_OCEAN
        }
    } else if rainfall < 0.27 {
        // Coasts, beaches, rivers
        if temperature < 0.12 {
            biome_id::COLD_BEACH
        } else if temperature < 0.25 {
            biome_id::FROZEN_RIVER
        } else if temperature < 0.40 {
            biome_id::STONE_BEACH
        } else if temperature < 0.55 {
            biome_id::BEACH
        } else if temperature < 0.70 {
            biome_id::RIVER
        } else if temperature < 0.85 {
            biome_id::MUSHROOM_ISLAND_SHORE
        } else {
            biome_id::SWAMPLAND
        }
    } else if rainfall < 0.38 {
        // Flat biomes
        if temperature < 0.10 {
            biome_id::ICE_PLAINS
        } else if temperature < 0.20 {
            biome_id::ICE_PLAINS_SPIKES
        } else if temperature < 0.32 {
            biome_id::COLD_TAIGA
        } else if temperature < 0.42 {
            biome_id::COLD_TAIGA_MUTATED
        } else if temperature < 0.55 {
            biome_id::PLAINS
        } else if temperature < 0.65 {
            biome_id::SUNFLOWER_PLAINS
        } else if temperature < 0.78 {
            biome_id::SAVANNA
        } else if temperature < 0.88 {
            biome_id::DESERT
        } else {
            biome_id::DESERT_MUTATED
        }
    } else if rainfall < 0.50 {
        // Forests + temperate
        if temperature < 0.12 {
            biome_id::COLD_TAIGA_HILLS
        } else if temperature < 0.25 {
            biome_id::TAIGA
        } else if temperature < 0.35 {
            biome_id::TAIGA_MUTATED
        } else if temperature < 0.47 {
            biome_id::FOREST
        } else if temperature < 0.57 {
            biome_id::FLOWER_FOREST
        } else if temperature < 0.68 {
            biome_id::BIRCH_FOREST
        } else if temperature < 0.78 {
            biome_id::BIRCH_FOREST_MUTATED
        } else if temperature < 0.88 {
            biome_id::SAVANNA_PLATEAU
        } else {
            biome_id::DESERT_HILLS
        }
    } else if rainfall < 0.62 {
        // Dense forests + hills
        if temperature < 0.12 {
            biome_id::MEGA_TAIGA
        } else if temperature < 0.22 {
            biome_id::REDWOOD_TAIGA_MUTATED
        } else if temperature < 0.33 {
            biome_id::TAIGA_HILLS
        } else if temperature < 0.44 {
            biome_id::FOREST_HILLS
        } else if temperature < 0.55 {
            biome_id::ROOFED_FOREST
        } else if temperature < 0.65 {
            biome_id::ROOFED_FOREST_MUTATED
        } else if temperature < 0.75 {
            biome_id::BIRCH_FOREST_HILLS
        } else if temperature < 0.85 {
            biome_id::BIRCH_FOREST_HILLS_MUTATED
        } else {
            biome_id::SWAMPLAND_MUTATED
        }
    } else if rainfall < 0.74 {
        // Mesa + moderate mountains
        if temperature < 0.12 {
            biome_id::MEGA_TAIGA_HILLS
        } else if temperature < 0.22 {
            biome_id::REDWOOD_TAIGA_HILLS_MUTATED
        } else if temperature < 0.35 {
            biome_id::ICE_MOUNTAINS
        } else if temperature < 0.48 {
            biome_id::EXTREME_HILLS_EDGE
        } else if temperature < 0.60 {
            biome_id::JUNGLE_EDGE
        } else if temperature < 0.70 {
            biome_id::JUNGLE_EDGE_MUTATED
        } else if temperature < 0.80 {
            biome_id::MESA
        } else if temperature < 0.90 {
            biome_id::MESA_BRYCE
        } else {
            biome_id::MESA_PLATEAU
        }
    } else if rainfall < 0.86 {
        // Jungles + big terrain
        if temperature < 0.12 {
            biome_id::EXTREME_HILLS_MUTATED
        } else if temperature < 0.25 {
            biome_id::EXTREME_HILLS
        } else if temperature < 0.38 {
            biome_id::EXTREME_HILLS_PLUS_TREES
        } else if temperature < 0.50 {
            biome_id::EXTREME_HILLS_PLUS_TREES_MUTATED
        } else if temperature < 0.62 {
            biome_id::JUNGLE
        } else if temperature < 0.72 {
            biome_id::JUNGLE_MUTATED
        } else if temperature < 0.82 {
            biome_id::BAMBOO_JUNGLE
        } else if temperature < 0.92 {
            biome_id::BAMBOO_JUNGLE_HILLS
        } else {
            biome_id::MESA_PLATEAU_STONE
        }
    } else {
        // Extreme terrain
        if temperature < 0.15 {
            biome_id::SAVANNA_MUTATED
        } else if temperature < 0.30 {
            biome_id::SAVANNA_PLATEAU_MUTATED
        } else if temperature < 0.45 {
            biome_id::JUNGLE_HILLS
        } else if temperature < 0.60 {
            biome_id::MUSHROOM_ISLAND
        } else if temperature < 0.75 {
            biome_id::MESA_PLATEAU_MUTATED
        } else if temperature < 0.88 {
            biome_id::MESA_PLATEAU_STONE_MUTATED
        } else {
            biome_id::SAVANNA_MUTATED
        }
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
