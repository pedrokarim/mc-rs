//! Générateur The End — **density functions vanilla** (`noise_settings/end.json`).
//!
//! Réutilise le moteur worldgen moderne via [`worldgen::dim_noise`] : le terrain
//! suit le vrai `final_density` vanilla de l'End — `sloped_cheese = end_islands +
//! base_3d_noise`, puis gradients Y (offset -0.234375, ×23.4375, ×0.64). Le nœud
//! `end_islands` (implémenté dans `density.rs`) produit l'**île centrale dômée**
//! (`clamp(100 - dist·8)`) + les **îles externes** flottantes (>1024 blocs) tirées
//! par un bruit 2D. Remplace l'ancien placeholder géométrique (ellipsoïde + piliers).
//!
//! Biome `the_end` partout (Bedrock fusionne tous les biomes End). Les piliers
//! d'obsidienne + cages sont posés par `place_spikes` (cristaux = entités du
//! `dragon_fight_manager`) ; les **chorus trees des îles externes** par
//! `end_decorate`. End gateways (sortie post-dragon) + end cities = suite.

use std::sync::{LazyLock, Mutex};

use super::block_registry::BLOCKS;
use super::worldgen::density::{self, NoiseRouter};
use super::worldgen::dim_noise::{generate_dim_chunk, DimConfig, DimGrid};
use crate::obsidian_pillar;

const END_BIOME: u32 = 9; // BiomeIds::THE_END (Bedrock)

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
        bedrock_floor: 0,    // îles flottantes : ni sol ni toit bedrock
        bedrock_roof: 0,
    }
}

/// Pose les 10 end spikes (spec vanilla `obsidian_pillar::end_spikes`) : colonne
/// d'obsidienne `rayon` de Y=0 au sommet, coiffée de bedrock, + cage de 73 barreaux
/// de fer (5×5) pour les piliers gardés. Le **cristal** (à `height+1`) est une
/// entité spawnée par `dragon_fight_manager`. `grid.set` clippe hors du chunk, donc
/// on itère tous les piliers à chaque chunk sans déborder.
fn place_spikes(grid: &mut DimGrid, seed: u64) {
    let obsidian = BLOCKS.get("minecraft:obsidian");
    let bedrock = BLOCKS.bedrock;
    let iron_bars = BLOCKS.get("minecraft:iron_bars");
    for s in obsidian_pillar::end_spikes(seed) {
        let r2 = s.radius * s.radius;
        for dx in -s.radius..=s.radius {
            for dz in -s.radius..=s.radius {
                if dx * dx + dz * dz > r2 {
                    continue; // colonne circulaire de rayon `s.radius`
                }
                for y in 0..s.height {
                    grid.set(s.x + dx, y, s.z + dz, obsidian);
                }
                grid.set(s.x + dx, s.height, s.z + dz, bedrock); // sommet bedrock
            }
        }
        // Cage de fer (5×5, 73 barreaux) autour du cristal des piliers gardés :
        // parois où |dx|==2 ou |dz|==2, + couche du dessus pleine (dy==3).
        if s.guarded {
            let cy = s.crystal_y();
            for dy in 0..=3i32 {
                for dx in -2..=2i32 {
                    for dz in -2..=2i32 {
                        if dx.abs() == 2 || dz.abs() == 2 || dy == 3 {
                            grid.set(s.x + dx, cy + dy, s.z + dz, iron_bars);
                        }
                    }
                }
            }
        }
    }
}

/// Génère un chunk de l'End : renvoie `(sub_chunk_count, payload)`.
pub fn generate_end_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> (u32, Vec<u8>) {
    let router = with_router(seed, |r| r.clone());
    let (base_x, base_z) = (chunk_x * 16, chunk_z * 16);
    generate_dim_chunk(
        chunk_x,
        chunk_z,
        seed,
        &router,
        &end_config(),
        |grid| {
            place_spikes(grid, seed); // piliers + cristaux au centre
            super::end_decorate::decorate(grid, base_x, base_z, seed); // chorus îles externes
        },
        |_, _, _| END_BIOME, // Bedrock : tout l'End est `the_end`
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

#[cfg(test)]
mod map_probe {
    use super::*;
    /// Structure vanilla : île centrale solide, anneau de vide (~100..1024),
    /// puis îles externes au-delà de ~1024 blocs.
    #[test]
    fn end_has_center_voidring_and_outer_islands() {
        let r = with_router(42, |x| x.clone());
        let solid = |wx: i32, wz: i32| {
            (0..120)
                .filter(|&y| r.final_density.compute(wx, y, wz) > 0.0)
                .count()
        };
        let center = solid(0, 0);
        let ring: usize = (200..900).step_by(64).map(|d| solid(d, 0)).sum();
        let outer: usize = (1100..3000)
            .step_by(53)
            .map(|d| solid(d, 0) + solid(d, d))
            .sum();
        assert!(center > 0, "île centrale solide attendue");
        assert_eq!(
            ring, 0,
            "anneau de vide attendu entre centre et îles externes"
        );
        assert!(outer > 0, "îles externes attendues au-delà de ~1024 blocs");
    }
}
