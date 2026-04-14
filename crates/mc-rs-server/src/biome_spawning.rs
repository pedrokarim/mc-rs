//! Biome-specific vanilla spawning — port PMMP worldgen spawn feature.
//! Définit quels features (arbres, fleurs, structures) spawn dans chaque biome.

use crate::biomes_registry::BiomeKind;

#[derive(Debug, Clone, Copy)]
pub struct BiomeFeatures {
    pub tree_density: f32,      // arbres / chunk
    pub flower_chance: f32,     // 0..1
    pub tall_grass_chance: f32, // 0..1
    pub water_lake_chance: f32,
    pub lava_lake_chance: f32,
    pub snow_layer: bool,
    pub ice_patches: bool,
}

pub fn biome_features(biome: BiomeKind) -> BiomeFeatures {
    match biome {
        BiomeKind::Plains | BiomeKind::SunflowerPlains | BiomeKind::Meadow => BiomeFeatures {
            tree_density: 0.05,
            flower_chance: 0.3,
            tall_grass_chance: 0.25,
            water_lake_chance: 0.01,
            lava_lake_chance: 0.0005,
            snow_layer: false,
            ice_patches: false,
        },
        BiomeKind::Forest | BiomeKind::BirchForest | BiomeKind::OldGrowthBirchForest => {
            BiomeFeatures {
                tree_density: 0.8,
                flower_chance: 0.2,
                tall_grass_chance: 0.2,
                water_lake_chance: 0.01,
                lava_lake_chance: 0.0005,
                snow_layer: false,
                ice_patches: false,
            }
        }
        BiomeKind::DarkForest => BiomeFeatures {
            tree_density: 1.2,
            flower_chance: 0.15,
            tall_grass_chance: 0.15,
            water_lake_chance: 0.005,
            lava_lake_chance: 0.0001,
            snow_layer: false,
            ice_patches: false,
        },
        BiomeKind::Jungle | BiomeKind::SparseJungle | BiomeKind::BambooJungle => BiomeFeatures {
            tree_density: 1.5,
            flower_chance: 0.1,
            tall_grass_chance: 0.3,
            water_lake_chance: 0.01,
            lava_lake_chance: 0.0001,
            snow_layer: false,
            ice_patches: false,
        },
        BiomeKind::Taiga | BiomeKind::OldGrowthPineTaiga | BiomeKind::OldGrowthSpruceTaiga => {
            BiomeFeatures {
                tree_density: 0.7,
                flower_chance: 0.05,
                tall_grass_chance: 0.1,
                water_lake_chance: 0.01,
                lava_lake_chance: 0.0005,
                snow_layer: false,
                ice_patches: false,
            }
        }
        BiomeKind::SnowyTaiga | BiomeKind::SnowyPlains | BiomeKind::IceSpikes
        | BiomeKind::FrozenPeaks | BiomeKind::SnowySlopes | BiomeKind::Grove => BiomeFeatures {
            tree_density: 0.4,
            flower_chance: 0.02,
            tall_grass_chance: 0.05,
            water_lake_chance: 0.005,
            lava_lake_chance: 0.0,
            snow_layer: true,
            ice_patches: true,
        },
        BiomeKind::Desert => BiomeFeatures {
            tree_density: 0.0,
            flower_chance: 0.0,
            tall_grass_chance: 0.0,
            water_lake_chance: 0.0,
            lava_lake_chance: 0.001,
            snow_layer: false,
            ice_patches: false,
        },
        BiomeKind::Savanna | BiomeKind::SavannaPlateau | BiomeKind::WindsweptSavanna => {
            BiomeFeatures {
                tree_density: 0.1,
                flower_chance: 0.05,
                tall_grass_chance: 0.4,
                water_lake_chance: 0.005,
                lava_lake_chance: 0.0005,
                snow_layer: false,
                ice_patches: false,
            }
        }
        BiomeKind::Swamp | BiomeKind::MangroveSwamp => BiomeFeatures {
            tree_density: 0.4,
            flower_chance: 0.05,
            tall_grass_chance: 0.2,
            water_lake_chance: 0.05,
            lava_lake_chance: 0.0,
            snow_layer: false,
            ice_patches: false,
        },
        BiomeKind::MushroomFields => BiomeFeatures {
            tree_density: 0.0,
            flower_chance: 0.0,
            tall_grass_chance: 0.0,
            water_lake_chance: 0.01,
            lava_lake_chance: 0.0,
            snow_layer: false,
            ice_patches: false,
        },
        BiomeKind::CherryGrove => BiomeFeatures {
            tree_density: 0.6,
            flower_chance: 0.25,
            tall_grass_chance: 0.2,
            water_lake_chance: 0.01,
            lava_lake_chance: 0.0,
            snow_layer: false,
            ice_patches: false,
        },
        _ => BiomeFeatures {
            tree_density: 0.0,
            flower_chance: 0.0,
            tall_grass_chance: 0.0,
            water_lake_chance: 0.0,
            lava_lake_chance: 0.0,
            snow_layer: false,
            ice_patches: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desert_no_trees() {
        assert_eq!(biome_features(BiomeKind::Desert).tree_density, 0.0);
    }

    #[test]
    fn jungle_has_many_trees() {
        assert!(biome_features(BiomeKind::Jungle).tree_density > 1.0);
    }

    #[test]
    fn snowy_biomes_have_snow_layer() {
        assert!(biome_features(BiomeKind::SnowyPlains).snow_layer);
    }
}
