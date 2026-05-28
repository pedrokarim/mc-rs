//! Phase C — Surface rules (habillage du terrain).
//!
//! Porte le **sous-ensemble générique** des surface rules vanilla overworld
//! (`noise_settings/overworld.json` → `surface_rule`) : la couche visible
//! grass/dirt sur la terre exposée, gravier sur les fonds marins, plancher de
//! bedrock et transition deepslate. Les paramètres (niveau de la mer 63,
//! profondeur de surface via le bruit `minecraft:surface`) sont vanilla.
//!
//! Limites assumées à ce stade (levées avec la Phase B = biomes) :
//! - Les règles biome-spécifiques (sable de désert/plage, terracotta des
//!   badlands, neige, mud des mangroves…) ne sont pas appliquées : sans
//!   placement de biomes, les conditions `minecraft:biome` seraient toujours
//!   fausses et retomberaient sur cette surface générique. C'est exactement ce
//!   que fait cette passe.
//! - On habille le bloc solide le plus haut de chaque colonne (proxy fidèle de
//!   la condition `above_preliminary_surface` pour la surface visible) ; les
//!   sols de grottes et le dessus des surplombs restent en pierre.
//! - Bedrock sur une seule couche (pas la frange probabiliste vanilla -64→-59),
//!   deepslate en coupe nette à y≤0 (pas la frange 0→8).
//!
//! Remplacé à terme par l'interpréteur complet du `surface_rule` data-driven,
//! une fois les biomes disponibles.

use super::super::block_registry::BLOCKS;
use super::data;
use super::noise_chunk::{grid_index, MAX_Y, MIN_Y};
use super::perlin::NormalNoise;
use super::rng::XoroshiroRandom;

/// Bruit de profondeur de surface (équiv. `surfaceNoise` vanilla), seedé une
/// fois par seed comme le reste du worldgen.
struct SurfaceContext {
    surface_noise: NormalNoise,
}

impl SurfaceContext {
    fn new(seed: u64) -> Self {
        let mut base = XoroshiroRandom::from_seed(seed);
        let deriver = base.fork_positional();
        let mut rng = deriver.from_hash_of("minecraft:surface");
        let params = data::noise_params("minecraft:surface").expect("bruit minecraft:surface");
        SurfaceContext {
            surface_noise: NormalNoise::create(&mut rng, &params),
        }
    }

    /// Profondeur de la couche de surface en un point (vanilla :
    /// `surfaceNoise * 2.75 + 3`, le terme aléatoire ±0.25 est négligé).
    fn surface_depth(&self, wx: i32, wz: i32) -> i32 {
        (self.surface_noise.get_value(wx as f64, 0.0, wz as f64) * 2.75 + 3.0).floor() as i32
    }
}

/// Applique l'habillage de surface sur une grille de blocs pleine hauteur
/// (déjà remplie en stone/water/air par `noise_chunk`).
pub fn apply(grid: &mut [u32], seed: u64, base_x: i32, base_z: i32) {
    let ctx = SurfaceContext::new(seed);

    for lx in 0..16usize {
        for lz in 0..16usize {
            // Bloc solide le plus haut de la colonne = la surface.
            let mut top = None;
            for wy in (MIN_Y..MAX_Y).rev() {
                if grid[grid_index(lx, wy, lz)] == BLOCKS.stone {
                    top = Some(wy);
                    break;
                }
            }
            let Some(ty) = top else { continue };

            let above = if ty + 1 < MAX_Y {
                grid[grid_index(lx, ty + 1, lz)]
            } else {
                BLOCKS.air
            };

            if above == BLOCKS.water {
                // Fond marin : gravier (règle océan générique).
                grid[grid_index(lx, ty, lz)] = BLOCKS.gravel;
            } else {
                // Terre exposée : grass au sommet, dirt sur `surface_depth`.
                grid[grid_index(lx, ty, lz)] = BLOCKS.grass_block;
                let wx = base_x + lx as i32;
                let wz = base_z + lz as i32;
                let depth = ctx.surface_depth(wx, wz).max(0);
                for d in 1..=depth {
                    let y = ty - d;
                    if y < MIN_Y {
                        break;
                    }
                    let i = grid_index(lx, y, lz);
                    if grid[i] == BLOCKS.stone {
                        grid[i] = BLOCKS.dirt;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    // Plancher de bedrock + transition deepslate (biome-indépendant).
    for lx in 0..16usize {
        for lz in 0..16usize {
            grid[grid_index(lx, MIN_Y, lz)] = BLOCKS.bedrock;
            for wy in (MIN_Y + 1)..=0 {
                let i = grid_index(lx, wy, lz);
                if grid[i] == BLOCKS.stone {
                    grid[i] = BLOCKS.deepslate;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::noise_chunk::{GRID_LEN, SEA_LEVEL};
    use super::*;

    fn fill_column(grid: &mut [u32], lx: usize, lz: usize, top: i32, underwater: bool) {
        for wy in MIN_Y..=top {
            grid[grid_index(lx, wy, lz)] = BLOCKS.stone;
        }
        if underwater {
            for wy in (top + 1)..=SEA_LEVEL {
                grid[grid_index(lx, wy, lz)] = BLOCKS.water;
            }
        }
    }

    #[test]
    fn land_column_gets_grass_dirt_bedrock_deepslate() {
        let mut grid = vec![BLOCKS.air; GRID_LEN].into_boxed_slice();
        let top = 80;
        fill_column(&mut grid, 0, 0, top, false);

        apply(&mut grid, 42, 0, 0);

        assert_eq!(
            grid[grid_index(0, top, 0)],
            BLOCKS.grass_block,
            "sommet = grass"
        );
        assert_eq!(
            grid[grid_index(0, top - 1, 0)],
            BLOCKS.dirt,
            "sous le grass = dirt"
        );
        assert_eq!(
            grid[grid_index(0, MIN_Y, 0)],
            BLOCKS.bedrock,
            "plancher = bedrock"
        );
        assert_eq!(
            grid[grid_index(0, 0, 0)],
            BLOCKS.deepslate,
            "y=0 = deepslate"
        );
        // Loin sous la surface mais au-dessus de 0 → reste pierre.
        assert_eq!(
            grid[grid_index(0, 40, 0)],
            BLOCKS.stone,
            "profond mais >0 = stone"
        );
    }

    #[test]
    fn underwater_column_gets_gravel_floor() {
        let mut grid = vec![BLOCKS.air; GRID_LEN].into_boxed_slice();
        let top = 40; // sous le niveau de la mer
        fill_column(&mut grid, 5, 5, top, true);

        apply(&mut grid, 42, 0, 0);

        assert_eq!(
            grid[grid_index(5, top, 5)],
            BLOCKS.gravel,
            "fond marin = gravier"
        );
        assert_eq!(
            grid[grid_index(5, top + 1, 5)],
            BLOCKS.water,
            "eau au-dessus conservée"
        );
    }
}
