//! Générateur de monde moderne (architecture Minecraft 1.18+ « Caves & Cliffs »).
//!
//! Density-functions + bruit Perlin + biomes multi-noise, porté depuis le
//! worldgen vanilla Java (données verbatim dans `data/worldgen/`, que Bedrock
//! reproduit depuis l'unification). Distinct du générateur legacy
//! `super::terrain_generator` (heightmap PMMP), qui reste en place le temps de
//! la migration.

pub mod blended_noise;
pub mod data;
pub mod density;
pub mod noise_chunk;
pub mod perlin;
pub mod rng;
pub mod spline;
pub mod surface;
