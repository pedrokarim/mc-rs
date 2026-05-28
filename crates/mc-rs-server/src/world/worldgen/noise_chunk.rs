//! Phase A4 — Échantillonnage NoiseChunk + remplissage terrain.
//!
//! Porte la sémantique vanilla `NoiseChunk` : la density function `final_density`
//! est échantillonnée aux COINS des cellules (4×4 horizontal, 8 vertical) puis
//! interpolée trilinéairement à l'intérieur de chaque cellule. C'est précisément
//! ce que signifie le wrapper `interpolated` côté données vanilla — l'A3 traite
//! ces marqueurs comme transparents (cf. `density.rs`) et l'interpolation par
//! cellules est faite ici.
//!
//! Simplifications assumées à ce stade (levées dans les phases suivantes) :
//! - Biomes : Plains partout (Phase B = placement multi-noise 6D).
//! - Blocs : `stone` si densité > 0, sinon `water` au niveau de la mer ou en
//!   dessous, sinon air. Les surface rules (grass/dirt/sand/gravel…) = Phase C ;
//!   les aquifères + grottes par bruit = Phase D.
//!
//! Le router de bruit (équivalent du `RandomState` vanilla) est instancié une
//! seule fois par seed et réutilisé pour tous les chunks, garantissant la
//! continuité du bruit aux frontières.

use std::sync::{LazyLock, Mutex};

use super::super::block_registry::BLOCKS;
use super::super::chunk_serializer;
use super::density::{self, NoiseRouter};

/// Plancher du monde (overworld 1.18+).
pub(super) const MIN_Y: i32 = -64;
/// Hauteur totale colonne (overworld). `noise.height` de `noise_settings`.
pub(super) const HEIGHT: i32 = 384;
/// Plafond (exclusif) = MIN_Y + HEIGHT.
pub(super) const MAX_Y: i32 = MIN_Y + HEIGHT;
/// Niveau de la mer (`sea_level` de `noise_settings/overworld.json`).
pub(super) const SEA_LEVEL: i32 = 63;
/// Colonnes par chunk (16×16).
pub(super) const COLS: usize = 256;
/// Taille de la grille de blocs d'un chunk (toute la hauteur).
pub(super) const GRID_LEN: usize = HEIGHT as usize * COLS;

/// Index dans la grille de blocs pleine hauteur, ordre `[y][x][z]`.
#[inline]
pub(super) fn grid_index(lx: usize, wy: i32, lz: usize) -> usize {
    ((wy - MIN_Y) as usize) * COLS + lx * 16 + lz
}

/// Largeur d'une cellule horizontale = `size_horizontal(1) * 4`.
const CELL_W: i32 = 4;
/// Hauteur d'une cellule = `size_vertical(2) * 4`.
const CELL_H: i32 = 8;

/// Nombre de cellules horizontales par chunk (16 / 4).
const CELLS_XZ: usize = 16 / CELL_W as usize;
/// Nombre de cellules verticales (384 / 8).
const CELLS_Y: usize = HEIGHT as usize / CELL_H as usize;
/// Nombre de sub-chunks émis (384 / 16).
const SUB_CHUNK_COUNT: usize = HEIGHT as usize / 16;

/// Nombre de coins de cellule par axe horizontal (CELLS_XZ + 1).
const NX: usize = CELLS_XZ + 1;
/// Nombre de coins de cellule sur l'axe vertical (CELLS_Y + 1).
const NY: usize = CELLS_Y + 1;

/// Router caché par seed — instancié une fois (équiv. `RandomState` vanilla).
static ROUTER: LazyLock<Mutex<(u64, Option<NoiseRouter>)>> =
    LazyLock::new(|| Mutex::new((0, None)));

fn with_router<R>(seed: u64, f: impl FnOnce(&NoiseRouter) -> R) -> R {
    let mut guard = ROUTER.lock().unwrap();
    if guard.1.is_none() || guard.0 != seed {
        *guard = (seed, Some(density::build_overworld(seed)));
    }
    f(guard.1.as_ref().unwrap())
}

#[inline]
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

#[inline]
fn palette_index(palette: &mut Vec<u32>, id: u32) -> u32 {
    if let Some(p) = palette.iter().position(|&b| b == id) {
        p as u32
    } else {
        palette.push(id);
        (palette.len() - 1) as u32
    }
}

/// Échantillonne la densité aux coins des cellules du chunk.
/// Indexé `[cell_x][cell_y][cell_z]` (coins, pas blocs).
fn sample_corners(base_x: i32, base_z: i32, router: &NoiseRouter) -> Box<[[[f64; NX]; NY]; NX]> {
    let mut corners = Box::new([[[0.0f64; NX]; NY]; NX]);
    let fd = &router.final_density;
    for (cx, plane) in corners.iter_mut().enumerate() {
        let wx = base_x + cx as i32 * CELL_W;
        for (cy, row) in plane.iter_mut().enumerate() {
            let wy = MIN_Y + cy as i32 * CELL_H;
            for (cz, slot) in row.iter_mut().enumerate() {
                let wz = base_z + cz as i32 * CELL_W;
                *slot = fd.compute(wx, wy, wz);
            }
        }
    }
    corners
}

/// Densité interpolée trilinéairement en un bloc, à partir des coins de cellule.
#[inline]
fn density_at(corners: &[[[f64; NX]; NY]; NX], lx: usize, wy: i32, lz: usize) -> f64 {
    let cx = lx / CELL_W as usize;
    let cz = lz / CELL_W as usize;
    let cy = ((wy - MIN_Y) / CELL_H) as usize;
    let dx = (lx % CELL_W as usize) as f64 / CELL_W as f64;
    let dz = (lz % CELL_W as usize) as f64 / CELL_W as f64;
    let dy = ((wy - MIN_Y) % CELL_H) as f64 / CELL_H as f64;

    let v000 = corners[cx][cy][cz];
    let v100 = corners[cx + 1][cy][cz];
    let v010 = corners[cx][cy + 1][cz];
    let v110 = corners[cx + 1][cy + 1][cz];
    let v001 = corners[cx][cy][cz + 1];
    let v101 = corners[cx + 1][cy][cz + 1];
    let v011 = corners[cx][cy + 1][cz + 1];
    let v111 = corners[cx + 1][cy + 1][cz + 1];

    // Interpolation x, puis y, puis z.
    let v00 = lerp(dx, v000, v100);
    let v10 = lerp(dx, v010, v110);
    let v01 = lerp(dx, v001, v101);
    let v11 = lerp(dx, v011, v111);
    let v0 = lerp(dy, v00, v10);
    let v1 = lerp(dy, v01, v11);
    lerp(dz, v0, v1)
}

/// Génère un chunk via le générateur de bruit moderne.
/// Retourne `(sub_chunk_count, payload réseau)` — même contrat que
/// `terrain_generator::generate_terrain_chunk`.
pub fn generate_noise_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> (u32, Vec<u8>) {
    let base_x = chunk_x * 16;
    let base_z = chunk_z * 16;

    let (corners, climate) = with_router(seed, |router| {
        (
            sample_corners(base_x, base_z, router),
            super::climate::ClimateSampler::from_router(router),
        )
    });

    // 1) Forme du terrain : grille pleine hauteur (stone / water / air).
    let mut grid = vec![BLOCKS.air; GRID_LEN].into_boxed_slice();
    for lx in 0..16usize {
        for lz in 0..16usize {
            for wy in MIN_Y..MAX_Y {
                let d = density_at(&corners, lx, wy, lz);
                if d > 0.0 {
                    grid[grid_index(lx, wy, lz)] = BLOCKS.stone;
                } else if wy <= SEA_LEVEL {
                    grid[grid_index(lx, wy, lz)] = BLOCKS.water;
                }
            }
        }
    }

    // 2) Habillage de surface (grass/dirt/gravel + bedrock/deepslate).
    super::surface::apply(&mut grid, seed, base_x, base_z);

    // 3) Sérialisation sub-chunk par sub-chunk.
    let mut payload = Vec::with_capacity(16384);
    for sub_idx in 0..SUB_CHUNK_COUNT {
        let sub_y_start = MIN_Y + sub_idx as i32 * 16;

        let mut blocks = [0u32; 4096];
        let mut palette = vec![BLOCKS.air];

        for lx in 0..16usize {
            for lz in 0..16usize {
                for ly in 0..16usize {
                    let wy = sub_y_start + ly as i32;
                    let block = grid[grid_index(lx, wy, lz)];
                    if block == BLOCKS.air {
                        continue;
                    }
                    let pidx = palette_index(&mut palette, block);
                    blocks[(lx << 8) | (lz << 4) | ly] = pidx;
                }
            }
        }

        if palette.len() == 1 {
            // Sub-chunk entièrement air → forme compacte (version + 0 couche).
            payload.push(8);
            payload.push(0);
        } else {
            payload.extend_from_slice(&chunk_serializer::serialize_sub_chunk(&blocks, &palette));
        }
    }

    // Biomes : placement multi-noise 6D échantillonné à la surface de chaque
    // colonne, mappé vers les IDs Bedrock (Phase B). La carte reste 2D pour
    // l'instant (répétée verticalement par le sérialiseur).
    static BIOMES: LazyLock<super::climate::OverworldBiomes> =
        LazyLock::new(super::climate::load_overworld);
    let mut biome_ids = [[0u32; 16]; 16];
    for lx in 0..16usize {
        for lz in 0..16usize {
            let wx = base_x + lx as i32;
            let wz = base_z + lz as i32;
            // Surface = bloc le plus haut qui n'est ni air ni eau.
            let mut sy = SEA_LEVEL;
            for wy in (MIN_Y..MAX_Y).rev() {
                let b = grid[grid_index(lx, wy, lz)];
                if b != BLOCKS.air && b != BLOCKS.water {
                    sy = wy;
                    break;
                }
            }
            let target = climate.sample(wx >> 2, sy >> 2, wz >> 2);
            let idx = BIOMES.params.find(&target);
            biome_ids[lx][lz] = BIOMES.bedrock_ids[idx as usize];
        }
    }
    let biome_data =
        chunk_serializer::serialize_biome_sections_from_columns(&biome_ids, SUB_CHUNK_COUNT);
    payload.extend_from_slice(&biome_data);

    // Border blocks count.
    payload.push(0);

    (SUB_CHUNK_COUNT as u32, payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_nonempty_chunk() {
        let (count, payload) = generate_noise_chunk(0, 0, 42);
        assert_eq!(count, SUB_CHUNK_COUNT as u32);
        assert!(!payload.is_empty());
    }

    #[test]
    fn deterministic_same_seed() {
        let a = generate_noise_chunk(3, -7, 123);
        let b = generate_noise_chunk(3, -7, 123);
        assert_eq!(a, b);
    }

    #[test]
    fn has_ground_and_air() {
        // Sur une colonne, il doit exister du solide en profondeur et de l'air
        // tout en haut (terrain plausible, pas une colonne pleine ou vide).
        let corners = with_router(42, |r| sample_corners(0, 0, r));
        let solid_deep = density_at(&corners, 0, -60, 0) > 0.0;
        let air_high = density_at(&corners, 0, 300, 0) <= 0.0;
        assert!(solid_deep, "le sous-sol profond devrait être solide");
        assert!(air_high, "le ciel devrait être de l'air");
    }

    #[test]
    fn surface_in_plausible_range() {
        let corners = with_router(42, |r| sample_corners(0, 0, r));
        let mut surface = None;
        for wy in (MIN_Y..(MIN_Y + HEIGHT)).rev() {
            if density_at(&corners, 8, wy, 8) > 0.0 {
                surface = Some(wy);
                break;
            }
        }
        let s = surface.expect("une surface solide doit exister");
        assert!(
            (-64..=200).contains(&s),
            "surface hors plage plausible: y={s}"
        );
    }
}
