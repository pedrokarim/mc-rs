//! Biome registry and selection system for world generation.
//!
//! Biome selection uses temperature and humidity noise to assign biomes
//! to world columns. Biome IDs match Bedrock protocol values.

use crate::noise::OctaveNoise;

/// Tree types available for biome decoration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeType {
    Oak,
    Birch,
    Spruce,
    Acacia,
    None,
}

/// Definition of a single biome.
#[derive(Debug, Clone)]
pub struct BiomeDef {
    /// Bedrock biome ID (must match protocol).
    pub id: u8,
    /// Biome name.
    pub name: &'static str,
    /// Block name for the top surface layer.
    pub surface_block: &'static str,
    /// Block name for the 3-4 layers below surface.
    pub filler_block: &'static str,
    /// Block name for underwater floor.
    pub underwater_block: &'static str,
    /// Base terrain height offset (added to base ~64).
    pub height_offset: f64,
    /// Terrain height amplitude multiplier.
    pub height_scale: f64,
    /// Primary tree type for this biome.
    pub tree_type: TreeType,
    /// Approximate number of trees per chunk.
    pub tree_density: u32,
    /// Whether to place a snow layer on the surface.
    pub has_snow: bool,
}

/// All biome definitions (10 biomes).
static BIOME_DEFS: &[BiomeDef] = &[
    BiomeDef {
        id: 0,
        name: "ocean",
        surface_block: "minecraft:gravel",
        filler_block: "minecraft:gravel",
        underwater_block: "minecraft:gravel",
        height_offset: -20.0,
        height_scale: 0.3,
        tree_type: TreeType::None,
        tree_density: 0,
        has_snow: false,
    },
    BiomeDef {
        id: 1,
        name: "plains",
        surface_block: "minecraft:grass_block",
        filler_block: "minecraft:dirt",
        underwater_block: "minecraft:sand",
        height_offset: 0.0,
        height_scale: 0.5,
        tree_type: TreeType::Oak,
        tree_density: 1,
        has_snow: false,
    },
    BiomeDef {
        id: 2,
        name: "desert",
        surface_block: "minecraft:sand",
        filler_block: "minecraft:sand",
        underwater_block: "minecraft:sand",
        height_offset: 2.0,
        height_scale: 0.3,
        tree_type: TreeType::None,
        tree_density: 0,
        has_snow: false,
    },
    BiomeDef {
        id: 3,
        name: "extreme_hills",
        surface_block: "minecraft:stone",
        filler_block: "minecraft:stone",
        underwater_block: "minecraft:gravel",
        height_offset: 15.0,
        height_scale: 2.5,
        tree_type: TreeType::Spruce,
        tree_density: 2,
        has_snow: false,
    },
    BiomeDef {
        id: 4,
        name: "forest",
        surface_block: "minecraft:grass_block",
        filler_block: "minecraft:dirt",
        underwater_block: "minecraft:sand",
        height_offset: 1.0,
        height_scale: 0.6,
        tree_type: TreeType::Oak,
        tree_density: 8,
        has_snow: false,
    },
    BiomeDef {
        id: 5,
        name: "taiga",
        surface_block: "minecraft:grass_block",
        filler_block: "minecraft:dirt",
        underwater_block: "minecraft:gravel",
        height_offset: 3.0,
        height_scale: 0.8,
        tree_type: TreeType::Spruce,
        tree_density: 6,
        has_snow: false,
    },
    BiomeDef {
        id: 7,
        name: "river",
        surface_block: "minecraft:sand",
        filler_block: "minecraft:sand",
        underwater_block: "minecraft:sand",
        height_offset: -3.0,
        height_scale: 0.2,
        tree_type: TreeType::None,
        tree_density: 0,
        has_snow: false,
    },
    BiomeDef {
        id: 12,
        name: "ice_plains",
        surface_block: "minecraft:grass_block",
        filler_block: "minecraft:dirt",
        underwater_block: "minecraft:gravel",
        height_offset: 0.0,
        height_scale: 0.4,
        tree_type: TreeType::Spruce,
        tree_density: 1,
        has_snow: true,
    },
    BiomeDef {
        id: 27,
        name: "birch_forest",
        surface_block: "minecraft:grass_block",
        filler_block: "minecraft:dirt",
        underwater_block: "minecraft:sand",
        height_offset: 1.0,
        height_scale: 0.6,
        tree_type: TreeType::Birch,
        tree_density: 7,
        has_snow: false,
    },
    BiomeDef {
        id: 35,
        name: "savanna",
        surface_block: "minecraft:grass_block",
        filler_block: "minecraft:dirt",
        underwater_block: "minecraft:sand",
        height_offset: 1.0,
        height_scale: 0.4,
        tree_type: TreeType::Acacia,
        tree_density: 2,
        has_snow: false,
    },
];

/// Biome selection system using temperature, humidity, and river noise.
pub struct BiomeSelector {
    temperature_noise: OctaveNoise,
    humidity_noise: OctaveNoise,
    river_noise: OctaveNoise,
}

impl BiomeSelector {
    /// Create a new biome selector with the given seed.
    pub fn new(seed: u64) -> Self {
        Self {
            temperature_noise: OctaveNoise::new(seed.wrapping_add(100), 4, 2.0, 0.5),
            humidity_noise: OctaveNoise::new(seed.wrapping_add(200), 4, 2.0, 0.5),
            river_noise: OctaveNoise::new(seed.wrapping_add(300), 3, 2.0, 0.5),
        }
    }

    /// Select a biome for the given world block coordinates.
    pub fn get_biome(&self, block_x: i32, block_z: i32) -> &'static BiomeDef {
        let nx = block_x as f64 / 256.0;
        let nz = block_z as f64 / 256.0;

        // River detection: narrow bands where river noise is near zero
        let river = self.river_noise.sample_2d(nx * 2.0, nz * 2.0);
        if river.abs() < 0.03 {
            return biome_by_id(7); // River
        }

        let temp = self.temperature_noise.sample_2d(nx, nz);
        let humid = self.humidity_noise.sample_2d(nx, nz);

        // Selection based on temperature/humidity ranges
        if temp < -0.3 {
            // Cold
            biome_by_id(12) // Ice Plains
        } else if temp < 0.0 {
            // Cool
            if humid > 0.2 {
                biome_by_id(5) // Taiga
            } else {
                biome_by_id(0) // Ocean
            }
        } else if temp < 0.25 {
            // Temperate
            if humid > 0.4 {
                biome_by_id(3) // Mountains
            } else if humid > 0.1 {
                biome_by_id(4) // Forest
            } else {
                biome_by_id(1) // Plains
            }
        } else if temp < 0.5 {
            // Warm
            if humid > 0.3 {
                biome_by_id(27) // Birch Forest
            } else {
                biome_by_id(1) // Plains
            }
        } else {
            // Hot
            if humid > 0.0 {
                biome_by_id(35) // Savanna
            } else {
                biome_by_id(2) // Desert
            }
        }
    }
}

/// Look up a biome definition by its protocol ID.
fn biome_by_id(id: u8) -> &'static BiomeDef {
    BIOME_DEFS
        .iter()
        .find(|b| b.id == id)
        .unwrap_or(&BIOME_DEFS[1]) // fallback: plains
}

/// Get the static biome definitions table.
pub fn biome_defs() -> &'static [BiomeDef] {
    BIOME_DEFS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biome_ids_match_protocol() {
        assert_eq!(biome_by_id(0).name, "ocean");
        assert_eq!(biome_by_id(1).name, "plains");
        assert_eq!(biome_by_id(2).name, "desert");
        assert_eq!(biome_by_id(3).name, "extreme_hills");
        assert_eq!(biome_by_id(4).name, "forest");
        assert_eq!(biome_by_id(5).name, "taiga");
        assert_eq!(biome_by_id(7).name, "river");
        assert_eq!(biome_by_id(12).name, "ice_plains");
        assert_eq!(biome_by_id(27).name, "birch_forest");
        assert_eq!(biome_by_id(35).name, "savanna");
    }

    #[test]
    fn deterministic_biome_selection() {
        let sel1 = BiomeSelector::new(42);
        let sel2 = BiomeSelector::new(42);
        for x in -100..100 {
            for z in (-100..100).step_by(10) {
                assert_eq!(
                    sel1.get_biome(x, z).id,
                    sel2.get_biome(x, z).id,
                    "Biome mismatch at ({x}, {z})"
                );
            }
        }
    }

    #[test]
    fn biome_coverage() {
        let sel = BiomeSelector::new(12345);
        let mut found = std::collections::HashSet::new();
        for x in (-2000..2000).step_by(16) {
            for z in (-2000..2000).step_by(16) {
                found.insert(sel.get_biome(x, z).id);
            }
        }
        // Should find at least 5 different biomes in a 4000x4000 area
        assert!(
            found.len() >= 5,
            "Only found {} biomes: {:?}",
            found.len(),
            found
        );
    }

    #[test]
    fn all_biomes_have_valid_surface() {
        for biome in BIOME_DEFS {
            assert!(
                biome.surface_block.starts_with("minecraft:"),
                "Invalid surface for {}: {}",
                biome.name,
                biome.surface_block
            );
            assert!(
                biome.filler_block.starts_with("minecraft:"),
                "Invalid filler for {}: {}",
                biome.name,
                biome.filler_block
            );
        }
    }

    #[test]
    fn fallback_biome_is_plains() {
        let biome = biome_by_id(255); // Non-existent ID
        assert_eq!(biome.id, 1);
        assert_eq!(biome.name, "plains");
    }
}
