//! Décoration de l'End : chorus trees sur les **îles externes** (au-delà de
//! ~1024 blocs du centre, là où vanilla place `end_highlands`/`end_midlands`).
//!
//! Bedrock ne distingue pas les biomes de l'End (tout est `the_end`), mais la
//! distribution des features suit la classification Java : l'île **centrale** est
//! nue, l'**anneau de vide** (~100..1024) est vide, et les **îles externes** portent
//! les chorus. On classe chaque colonne par distance + relief généré.
//!
//! Réf : minecraft.wiki/w/The_End (chorus plants ubiquitaires sur les îles externes)
//! + `ChorusPlantFeature`. L'arbre est une version branchée compacte (chorus_plant
//! + chorus_flower aux extrémités).

use std::sync::LazyLock;

use super::block_registry::BLOCKS;
use super::worldgen::dim_noise::DimGrid;

/// Distance² (blocs) au-delà de laquelle commencent les îles externes (~1024).
const OUTER_DIST2: i64 = 1024 * 1024;

struct EndBlocks {
    air: u32,
    end_stone: u32,
    chorus_plant: u32,
    chorus_flower: u32,
}

static EB: LazyLock<EndBlocks> = LazyLock::new(|| EndBlocks {
    air: BLOCKS.air,
    end_stone: BLOCKS.get("minecraft:end_stone"),
    chorus_plant: BLOCKS.get("minecraft:chorus_plant"),
    chorus_flower: BLOCKS.get("minecraft:chorus_flower"),
});

/// RNG positionnel déterministe (splitmix64).
struct Rng(u64);
impl Rng {
    fn new(wx: i32, wz: i32, seed: u64) -> Self {
        let mut h = seed
            ^ (wx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (wz as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            ^ 0x0E4D_5EED;
        h ^= h >> 30;
        Rng(h | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn range(&mut self, n: i32) -> i32 {
        (self.next() % n.max(1) as u64) as i32
    }
    fn chance(&mut self, p: f64) -> bool {
        ((self.next() >> 11) as f64 / ((1u64 << 53) as f64)) < p
    }
}

/// Décore un chunk End déjà rempli (terrain + piliers). `base_x/base_z` = coin.
/// Pose des chorus trees sur la surface end_stone des îles externes.
pub fn decorate(grid: &mut DimGrid, base_x: i32, base_z: i32, seed: u64) {
    let eb = &*EB;
    for lx in 0..16i32 {
        for lz in 0..16i32 {
            let wx = base_x + lx;
            let wz = base_z + lz;
            // Seules les îles externes portent des chorus (centre nu, anneau vide).
            let dist2 = (wx as i64) * (wx as i64) + (wz as i64) * (wz as i64);
            if dist2 < OUTER_DIST2 {
                continue;
            }
            let mut rng = Rng::new(wx, wz, seed);
            // Surface : premier end_stone (de haut en bas) avec de l'air au-dessus.
            let mut surface = None;
            for y in (2..250).rev() {
                if grid.get_world(wx, y, wz) == eb.end_stone
                    && grid.get_world(wx, y + 1, wz) == eb.air
                {
                    surface = Some(y);
                    break;
                }
            }
            if let Some(y) = surface {
                if rng.chance(0.012) {
                    grow_chorus(grid, eb, wx, y + 1, wz, &mut rng);
                }
            }
        }
    }
}

/// Chorus tree compact : tronc de `chorus_plant` + quelques branches, extrémités
/// coiffées de `chorus_flower`.
fn grow_chorus(grid: &mut DimGrid, eb: &EndBlocks, wx: i32, y0: i32, wz: i32, rng: &mut Rng) {
    let trunk = 2 + rng.range(7); // 2..8
    for i in 0..trunk {
        if grid.get_world(wx, y0 + i, wz) != eb.air {
            return;
        }
        grid.set(wx, y0 + i, wz, eb.chorus_plant);
    }
    grid.set(wx, y0 + trunk, wz, eb.chorus_flower);

    // Quelques branches : à partir d'une hauteur, sortir d'1 bloc puis monter.
    let branches = rng.range(4);
    const DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for _ in 0..branches {
        let bh = y0 + 1 + rng.range(trunk.max(1));
        let (dx, dz) = DIRS[rng.range(4) as usize];
        let (bx, bz) = (wx + dx, wz + dz);
        if grid.get_world(bx, bh, wz + dz) != eb.air {
            continue;
        }
        grid.set(bx, bh, bz, eb.chorus_plant);
        let up = 1 + rng.range(3);
        for i in 1..=up {
            if grid.get_world(bx, bh + i, bz) != eb.air {
                break;
            }
            grid.set(bx, bh + i, bz, eb.chorus_plant);
        }
        grid.set(bx, bh + up + 1, bz, eb.chorus_flower);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_chorus_near_center() {
        // Une colonne proche du centre (île centrale) ne reçoit pas de chorus.
        let eb = &*EB;
        let mut grid = DimGrid::test_floor(0, 0, 0, 256, eb.end_stone, 60);
        decorate(&mut grid, 0, 0, 42);
        assert_eq!(grid.count(eb.chorus_plant), 0, "pas de chorus au centre");
    }

    #[test]
    fn chorus_on_outer_islands() {
        // Un large pan d'île externe (dist > 1024) doit porter des chorus.
        let eb = &*EB;
        let mut found = 0usize;
        for cx in 80..96 {
            for cz in 80..96 {
                let (bx, bz) = (cx * 16, cz * 16);
                let mut grid = DimGrid::test_floor(bx, bz, 0, 256, eb.end_stone, 60);
                decorate(&mut grid, bx, bz, 42);
                found += grid.count(eb.chorus_flower);
            }
        }
        assert!(found > 0, "chorus attendus sur les îles externes");
    }
}
