//! Générateur Nether — **density functions vanilla** (`noise_settings/nether.json`).
//!
//! Réutilise le moteur worldgen moderne via [`worldgen::dim_noise`] : le terrain
//! suit le vrai `final_density` vanilla du Nether (bruit `old_blended_noise`
//! `nether/base_3d_noise` + gradients Y en cloche, offset 2.5, ×0.64), donc la
//! **forme est fidèle** — voûtes de netherrack, mer de lave à Y≤32, toit/sol
//! bedrock. Remplace l'ancien placeholder Simplex.
//!
//! Biomes : **multi-noise vanilla** (5 biomes — nether_wastes / soul_sand_valley /
//! crimson_forest / warped_forest / basalt_deltas) via `ClimateSampler` + le param
//! list `biome_parameters/nether.json`. Les surface rules détaillées par biome
//! (nylium, soul sand, basalt) et les features (fungi, glowstone) viennent ensuite ;
//! le `surface_rule` vanilla est déjà vendoré dans `nether.json`.

use std::sync::{LazyLock, Mutex};

use super::block_registry::BLOCKS;
use super::worldgen::climate::{ClimateSampler, OverworldBiomes};
use super::worldgen::density::{self, NoiseRouter};
use super::worldgen::dim_noise::{generate_dim_chunk, DimConfig};

/// Param list multi-noise du Nether (5 biomes), chargé une fois.
static BIOMES: LazyLock<OverworldBiomes> =
    LazyLock::new(|| super::worldgen::climate::load_biomes("nether"));

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
        bedrock_floor: 5,
        bedrock_roof: 5,
    }
}

/// Index du biome Nether (Java) au point monde, via le climat multi-noise.
pub fn nether_biome_index(sampler: &ClimateSampler, wx: i32, wy: i32, wz: i32) -> u16 {
    BIOMES
        .params
        .find(&sampler.sample(wx >> 2, wy >> 2, wz >> 2))
}

/// Génère un chunk Nether : renvoie `(sub_chunk_count, payload)`.
pub fn generate_nether_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> (u32, Vec<u8>) {
    let router = with_router(seed, |r| r.clone());
    let sampler = ClimateSampler::from_router(&router);
    let (base_x, base_z) = (chunk_x * 16, chunk_z * 16);
    let biome_at = |wx: i32, wy: i32, wz: i32| -> u32 {
        BIOMES.bedrock_ids[nether_biome_index(&sampler, wx, wy, wz) as usize]
    };
    let decorate = |grid: &mut super::worldgen::dim_noise::DimGrid| {
        super::nether_decorate::decorate(grid, base_x, base_z, &sampler, seed);
    };
    generate_dim_chunk(
        chunk_x,
        chunk_z,
        seed,
        &router,
        &nether_config(),
        decorate,
        biome_at,
    )
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
    #[test]
    fn nether_has_multiple_biomes() {
        // Le climat multi-noise doit produire plusieurs biomes Nether (pas un seul).
        let router = with_router(42, |r| r.clone());
        let sampler = ClimateSampler::from_router(&router);
        let mut seen = std::collections::HashSet::new();
        for x in (-512..512).step_by(48) {
            for z in (-512..512).step_by(48) {
                seen.insert(nether_biome_index(&sampler, x, 64, z));
            }
        }
        assert!(
            seen.len() >= 3,
            "Nether multi-biome attendu (trouvé {})",
            seen.len()
        );
    }
}
