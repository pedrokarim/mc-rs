//! Générateur The End — **density functions vanilla** (`noise_settings/end.json`).
//!
//! Réutilise le moteur worldgen moderne via [`worldgen::dim_noise`] : le terrain
//! suit le vrai `final_density` vanilla de l'End — `sloped_cheese = end_islands +
//! base_3d_noise`, puis gradients Y (offset -0.234375, ×23.4375, ×0.64). Le nœud
//! `end_islands` (implémenté dans `density.rs`) produit l'**île centrale dômée**
//! (`clamp(100 - dist·8)`) + les **îles externes** flottantes (>1024 blocs) tirées
//! par un bruit 2D. Remplace l'ancien placeholder géométrique (ellipsoïde + piliers).
//!
//! v1 : biome `the_end` partout, end_stone uniquement. Le portail de sortie + les
//! piliers d'obsidienne + les end crystals sont posés par le combat du dragon
//! (`dragon_fight_manager`), pas par le terrain. Multi-biome (highlands/midlands/
//! barrens/small_islands via erosion) = milestone suivant.

use std::sync::{LazyLock, Mutex};

use super::block_registry::BLOCKS;
use super::worldgen::density::{self, NoiseRouter};
use super::worldgen::dim_noise::{generate_dim_chunk, DimConfig, DimGrid};
use crate::obsidian_pillar;

const END_BIOME: u32 = 9; // BiomeIds::THE_END (Bedrock)
/// Base des piliers d'obsidienne (ancrée dans l'île centrale).
const PILLAR_BASE_Y: i32 = 50;

/// Router End caché par seed.
static ROUTER: LazyLock<Mutex<(u64, Option<NoiseRouter>)>> =
    LazyLock::new(|| Mutex::new((0, None)));

fn with_router<R>(seed: u64, f: impl FnOnce(&NoiseRouter) -> R) -> R {
    let mut guard = ROUTER.lock().unwrap();
    if guard.1.is_none() || guard.0 != seed {
        *guard = (seed, Some(density::build_the_end(seed)));
    }
    f(guard.1.as_ref().unwrap())
}

fn end_config() -> DimConfig {
    DimConfig {
        min_y: 0,
        height: 256, // bornes Bedrock (16 sub-chunks) ; les îles vivent vers y40-72
        sea_level: 0,
        cell_w: 8, // size_horizontal 2
        cell_h: 4, // size_vertical 1
        default_block: BLOCKS.get("minecraft:end_stone"),
        default_fluid: None, // pas de fluide à l'End
        biome_id: END_BIOME,
        bedrock_floor: 0, // îles flottantes : ni sol ni toit bedrock
        bedrock_roof: 0,
    }
}

/// Pose les 10 piliers d'obsidienne (réf `obsidian_pillar`) sur l'île centrale :
/// colonne d'obsidienne `rayon 2` de `PILLAR_BASE_Y` au sommet, coiffée de bedrock
/// (l'end crystal qui s'y pose est une entité — `dragon_fight_manager`). `grid.set`
/// clippe hors du chunk, donc on itère tous les piliers à chaque chunk sans déborder.
fn place_pillars(grid: &mut DimGrid) {
    let obsidian = BLOCKS.get("minecraft:obsidian");
    let bedrock = BLOCKS.bedrock;
    for p in obsidian_pillar::vanilla_pillar_locations() {
        let top = 76 + (p.height as i32 - 41); // ~78..92 (variété vanilla)
        for dx in -2..=2 {
            for dz in -2..=2 {
                if dx * dx + dz * dz > 4 {
                    continue; // colonne ~circulaire (rayon 2)
                }
                for y in PILLAR_BASE_Y..top {
                    grid.set(p.x + dx, y, p.z + dz, obsidian);
                }
                grid.set(p.x + dx, top, p.z + dz, bedrock);
            }
        }
    }
}

/// Génère un chunk de l'End : renvoie `(sub_chunk_count, payload)`.
pub fn generate_end_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> (u32, Vec<u8>) {
    let router = with_router(seed, |r| r.clone());
    generate_dim_chunk(
        chunk_x,
        chunk_z,
        seed,
        &router,
        &end_config(),
        place_pillars,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_chunk_has_16_sub_chunks() {
        let (count, payload) = generate_end_chunk(0, 0, 1);
        assert_eq!(count, 16); // 256 / 16
        assert!(!payload.is_empty());
    }

    #[test]
    fn end_generation_is_deterministic() {
        assert_eq!(generate_end_chunk(1, 1, 1).1, generate_end_chunk(1, 1, 1).1);
    }

    #[test]
    fn central_island_is_solid_and_void_above() {
        // Au centre (0,0) l'île doit être solide à mi-hauteur (~y50) et vide haut
        // (~y120) — profil d'île flottante via le vrai final_density vanilla.
        let r = density::build_the_end(42);
        assert!(
            r.final_density.compute(0, 50, 0) > 0.0,
            "île centrale solide attendue à y50"
        );
        assert!(
            r.final_density.compute(0, 120, 0) <= 0.0,
            "vide attendu au-dessus de l'île"
        );
    }

    #[test]
    fn far_from_origin_is_void() {
        // Très loin de l'origine (hors île centrale, pas d'île externe ici) → vide.
        let r = density::build_the_end(42);
        assert!(
            r.final_density.compute(5000, 50, 5000) <= 0.0,
            "le vide de l'End attendu loin de toute île"
        );
    }
}
