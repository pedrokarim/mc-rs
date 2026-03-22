use super::flat_generator::block_ids;
use super::noise::Simplex;
use super::random::Random;
use super::terrain_generator::extra_blocks;

/// Biome IDs matching Bedrock/PMMP.
pub mod biome_id {
    pub const OCEAN: u32 = 0;
    pub const PLAINS: u32 = 1;
    pub const DESERT: u32 = 2;
    pub const EXTREME_HILLS: u32 = 3;
    pub const FOREST: u32 = 4;
    pub const TAIGA: u32 = 5;
    pub const SWAMPLAND: u32 = 6;
    pub const RIVER: u32 = 7;
    pub const ICE_PLAINS: u32 = 12;
    pub const EXTREME_HILLS_EDGE: u32 = 20;
    pub const BIRCH_FOREST: u32 = 27;
}

/// Biome definition with elevation range and ground cover.
#[derive(Debug, Clone)]
pub struct BiomeDef {
    pub id: u32,
    pub min_elevation: f64,
    pub max_elevation: f64,
    /// Ground cover blocks from top down (block runtime IDs).
    /// Phase 3 will use these; stored here for structure.
    pub ground_cover: Vec<u32>,
}

/// Grassy biome cover: grass, dirt x4
fn grassy_cover() -> Vec<u32> {
    vec![
        block_ids::GRASS_BLOCK,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
    ]
}

/// Snowy biome cover: snow_layer, grass, dirt x3
fn snowy_cover() -> Vec<u32> {
    vec![
        extra_blocks::SNOW_LAYER,
        block_ids::GRASS_BLOCK,
        block_ids::DIRT,
        block_ids::DIRT,
        block_ids::DIRT,
    ]
}

/// Get a biome definition by ID.
pub fn get_biome(id: u32) -> BiomeDef {
    match id {
        biome_id::OCEAN => BiomeDef {
            id,
            min_elevation: 46.0,
            max_elevation: 58.0,
            ground_cover: vec![
                extra_blocks::GRAVEL,
                extra_blocks::GRAVEL,
                extra_blocks::GRAVEL,
                extra_blocks::GRAVEL,
                extra_blocks::GRAVEL,
            ],
        },
        biome_id::PLAINS => BiomeDef {
            id,
            min_elevation: 63.0,
            max_elevation: 68.0,
            ground_cover: grassy_cover(),
        },
        biome_id::DESERT => BiomeDef {
            id,
            min_elevation: 63.0,
            max_elevation: 74.0,
            ground_cover: vec![
                extra_blocks::SAND,
                extra_blocks::SAND,
                extra_blocks::SANDSTONE,
                extra_blocks::SANDSTONE,
                extra_blocks::SANDSTONE,
            ],
        },
        biome_id::EXTREME_HILLS => BiomeDef {
            id,
            min_elevation: 63.0,
            max_elevation: 127.0,
            ground_cover: grassy_cover(),
        },
        biome_id::FOREST => BiomeDef {
            id,
            min_elevation: 63.0,
            max_elevation: 81.0,
            ground_cover: grassy_cover(),
        },
        biome_id::TAIGA => BiomeDef {
            id,
            min_elevation: 63.0,
            max_elevation: 81.0,
            ground_cover: snowy_cover(),
        },
        biome_id::SWAMPLAND => BiomeDef {
            id,
            min_elevation: 62.0,
            max_elevation: 63.0,
            ground_cover: grassy_cover(),
        },
        biome_id::RIVER => BiomeDef {
            id,
            min_elevation: 58.0,
            max_elevation: 62.0,
            ground_cover: vec![
                block_ids::DIRT,
                block_ids::DIRT,
                block_ids::DIRT,
                block_ids::DIRT,
                block_ids::DIRT,
            ],
        },
        biome_id::ICE_PLAINS => BiomeDef {
            id,
            min_elevation: 63.0,
            max_elevation: 74.0,
            ground_cover: snowy_cover(),
        },
        biome_id::EXTREME_HILLS_EDGE => BiomeDef {
            id,
            min_elevation: 63.0,
            max_elevation: 97.0,
            ground_cover: grassy_cover(),
        },
        biome_id::BIRCH_FOREST => BiomeDef {
            id,
            min_elevation: 63.0,
            max_elevation: 81.0,
            ground_cover: grassy_cover(),
        },
        // Default: Plains
        _ => BiomeDef {
            id: biome_id::PLAINS,
            min_elevation: 63.0,
            max_elevation: 68.0,
            ground_cover: grassy_cover(),
        },
    }
}

/// Biome selector using temperature and rainfall noise.
/// Port of PMMP's BiomeSelector.
pub struct BiomeSelector {
    temperature: Simplex,
    rainfall: Simplex,
    /// Lookup table: 64x64 mapping (temperature, rainfall) → biome ID.
    map: Vec<u32>,
}

impl BiomeSelector {
    pub fn new(random: &mut Random) -> Self {
        let temperature = Simplex::new(random, 2, 1.0 / 16.0, 1.0 / 512.0);
        let rainfall = Simplex::new(random, 2, 1.0 / 16.0, 1.0 / 512.0);

        // Build lookup table
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

    /// Pick a biome at the given world coordinates.
    pub fn pick_biome(&self, x: f64, z: f64) -> u32 {
        let temperature = (self.get_temperature(x, z) * 63.0) as usize;
        let rainfall = (self.get_rainfall(x, z) * 63.0) as usize;
        let temperature = temperature.min(63);
        let rainfall = rainfall.min(63);
        self.map[temperature + (rainfall << 6)]
    }
}

/// PMMP's biome lookup function: temperature x rainfall → biome ID.
fn lookup_biome(temperature: f64, rainfall: f64) -> u32 {
    if rainfall < 0.25 {
        if temperature < 0.7 {
            biome_id::OCEAN
        } else if temperature < 0.85 {
            biome_id::RIVER
        } else {
            biome_id::SWAMPLAND
        }
    } else if rainfall < 0.60 {
        if temperature < 0.25 {
            biome_id::ICE_PLAINS
        } else if temperature < 0.75 {
            biome_id::PLAINS
        } else {
            biome_id::DESERT
        }
    } else if rainfall < 0.80 {
        if temperature < 0.25 {
            biome_id::TAIGA
        } else if temperature < 0.75 {
            biome_id::FOREST
        } else {
            biome_id::BIRCH_FOREST
        }
    } else if temperature < 0.20 {
        biome_id::EXTREME_HILLS
    } else if temperature < 0.40 {
        biome_id::EXTREME_HILLS_EDGE
    } else {
        biome_id::RIVER
    }
}

/// 1D Gaussian kernel for elevation smoothing.
/// Port of PMMP's Gaussian class.
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

/// Pick a biome with noise jitter (matching PMMP's Normal::pickBiome).
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
    fn test_biome_lookup() {
        // Low rainfall, low temp → Ocean
        assert_eq!(lookup_biome(0.5, 0.1), biome_id::OCEAN);
        // Mid rainfall, mid temp → Plains
        assert_eq!(lookup_biome(0.5, 0.4), biome_id::PLAINS);
        // Mid rainfall, high temp → Desert
        assert_eq!(lookup_biome(0.8, 0.4), biome_id::DESERT);
        // High rainfall, mid temp → Forest
        assert_eq!(lookup_biome(0.5, 0.7), biome_id::FOREST);
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
    fn test_biome_elevations() {
        let ocean = get_biome(biome_id::OCEAN);
        assert_eq!(ocean.min_elevation, 46.0);
        assert_eq!(ocean.max_elevation, 58.0);

        let mountains = get_biome(biome_id::EXTREME_HILLS);
        assert_eq!(mountains.min_elevation, 63.0);
        assert_eq!(mountains.max_elevation, 127.0);
    }

    #[test]
    fn test_gaussian_kernel() {
        let g = Gaussian::new(2);
        assert_eq!(g.kernel_1d.len(), 5);
        assert!(g.weight_sum_1d > 0.0);
        // Kernel should be symmetric
        let eps = 1e-10;
        assert!((g.kernel_1d[0] - g.kernel_1d[4]).abs() < eps);
        assert!((g.kernel_1d[1] - g.kernel_1d[3]).abs() < eps);
    }
}
