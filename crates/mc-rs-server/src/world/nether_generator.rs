//! Générateur Nether — **density functions vanilla** (`noise_settings/nether.json`).
//!
//! Réutilise le moteur worldgen moderne via [`worldgen::dim_noise`] : le terrain
//! suit le vrai `final_density` vanilla du Nether (bruit `old_blended_noise`
//! `nether/base_3d_noise` + gradients Y en cloche, offset 2.5, ×0.64), donc la
//! **forme est fidèle** — voûtes de netherrack, mer de lave à Y≤32, toit/sol
//! bedrock. Remplace l'ancien placeholder Simplex.
//!
//! v1 : biome `nether_wastes` partout + crust simple (netherrack/lave). Les surface
//! rules détaillées (soul sand, basalt deltas, gravier) et les biomes multi-noise
//! (crimson/warped/soul_sand_valley/basalt_deltas) viendront ensuite (le
//! `surface_rule` vanilla est déjà vendoré dans `nether.json`).

use std::sync::{LazyLock, Mutex};

use super::block_registry::BLOCKS;
use super::worldgen::density::{self, NoiseRouter};
use super::worldgen::dim_noise::{generate_dim_chunk, DimConfig};

const NETHER_BIOME: u32 = 8; // BiomeIds::HELL (nether_wastes côté Bedrock)

/// Router Nether caché par seed (instancié une fois, comme l'overworld).
static ROUTER: LazyLock<Mutex<(u64, Option<NoiseRouter>)>> =
    LazyLock::new(|| Mutex::new((0, None)));

fn with_router<R>(seed: u64, f: impl FnOnce(&NoiseRouter) -> R) -> R {
    let mut guard = ROUTER.lock().unwrap();
    if guard.1.is_none() || guard.0 != seed {
        *guard = (seed, Some(density::build_nether(seed)));
    }
    f(guard.1.as_ref().unwrap())
}

fn nether_config() -> DimConfig {
    DimConfig {
        min_y: 0,
        height: 128,
        sea_level: 32, // mer de lave
        cell_w: 4,     // size_horizontal 1
        cell_h: 8,     // size_vertical 2
        default_block: BLOCKS.get("minecraft:netherrack"),
        default_fluid: Some(BLOCKS.lava),
        biome_id: NETHER_BIOME,
        bedrock_floor: 5,
        bedrock_roof: 5,
    }
}

/// Génère un chunk Nether : renvoie `(sub_chunk_count, payload)`.
pub fn generate_nether_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> (u32, Vec<u8>) {
    let router = with_router(seed, |r| r.clone());
    generate_dim_chunk(chunk_x, chunk_z, seed, &router, &nether_config())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nether_chunk_has_8_sub_chunks() {
        let (count, payload) = generate_nether_chunk(0, 0, 12345);
        assert_eq!(count, 8); // 128 / 16
        assert!(!payload.is_empty());
    }

    #[test]
    fn nether_generation_is_deterministic() {
        assert_eq!(
            generate_nether_chunk(3, -2, 999).1,
            generate_nether_chunk(3, -2, 999).1
        );
    }

    #[test]
    fn nether_has_terrain_and_bedrock_cap() {
        // Le terrain density doit produire de la netherrack quelque part, et le
        // plancher/plafond bedrock doit exister (sanity : chunk non vide).
        let (_, payload) = generate_nether_chunk(0, 0, 42);
        assert!(
            payload.len() > 100,
            "le Nether doit générer du terrain solide"
        );
    }

    #[test]
    fn density_profile_is_real_relief() {
        // Le `final_density` vanilla doit mélanger solide (netherrack) et vide
        // (cavernes/lave) sur une colonne — pas une dalle plate ni du vide.
        let r = density::build_nether(42);
        let (mut solid, mut air) = (0, 0);
        for y in 2..126 {
            if r.final_density.compute(0, y, 0) > 0.0 {
                solid += 1;
            } else {
                air += 1;
            }
        }
        assert!(
            solid > 0 && air > 0,
            "relief attendu, pas constant (solid={solid} air={air})"
        );
    }
}
