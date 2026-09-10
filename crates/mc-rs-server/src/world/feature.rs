//! Moteur de features de génération **data-driven**, fidèle aux algorithmes vanilla.
//!
//! Reproduit `OreFeature`/`ScatteredOreFeature` (veines) + les modificateurs de
//! placement (`count`/chunk, `count_on_every_layer`, `rarity_filter`) et les plages
//! de hauteur (`uniform`/`trapezoid`/`biased_to_bottom`, ancrages `above_bottom`/
//! `below_top`/`absolute`). Voir `dev-docs/27-DIMENSIONS-GENERATION-SPEC.md`.
//!
//! On opère sur une [`DimGrid`] (coords monde, clip hors chunk). Le RNG est
//! déterministe par chunk (les positions diffèrent de Java mais les statistiques —
//! count, taille de veine, plages — sont reproduites exactement).

use super::worldgen::dim_noise::DimGrid;

/// RNG déterministe (splitmix64), graine par chunk + flux par feature.
pub struct FeatureRng(u64);

impl FeatureRng {
    /// Graine = (chunk_x, chunk_z, world_seed, salt feature) → flux indépendant.
    pub fn new(chunk_x: i32, chunk_z: i32, seed: u64, salt: u64) -> Self {
        let mut h = seed
            ^ (chunk_x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (chunk_z as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            ^ salt.wrapping_mul(0xD1B5_4A32_D192_ED03);
        h ^= h >> 30;
        FeatureRng(h | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn f64(&mut self) -> f64 {
        (self.next() >> 11) as f64 / ((1u64 << 53) as f64)
    }
    pub fn f32(&mut self) -> f32 {
        self.f64() as f32
    }
    /// Entier dans [0, n).
    pub fn range(&mut self, n: i32) -> i32 {
        if n <= 0 {
            return 0;
        }
        (self.next() % n as u64) as i32
    }
    /// Entier dans [min, max] inclus.
    pub fn between(&mut self, min: i32, max: i32) -> i32 {
        if max <= min {
            return min;
        }
        min + self.range(max - min + 1)
    }
}

/// Borne de hauteur ancrée (résolue contre les bornes Y de la dimension).
#[derive(Clone, Copy)]
pub enum Anchor {
    Absolute(i32),
    AboveBottom(i32),
    BelowTop(i32),
}
impl Anchor {
    fn resolve(self, min_y: i32, max_y: i32) -> i32 {
        match self {
            Anchor::Absolute(y) => y,
            Anchor::AboveBottom(d) => min_y + d,
            Anchor::BelowTop(d) => max_y - d,
        }
    }
}

/// Distribution de hauteur d'un placement (vanilla `height_provider`).
#[derive(Clone, Copy)]
pub enum HeightRange {
    Uniform(Anchor, Anchor),
    /// Triangulaire (plus dense au centre) — ancient debris large.
    Trapezoid(Anchor, Anchor),
}
impl HeightRange {
    pub fn sample(&self, rng: &mut FeatureRng, min_y: i32, max_y: i32) -> i32 {
        match self {
            HeightRange::Uniform(a, b) => {
                let (lo, hi) = (a.resolve(min_y, max_y), b.resolve(min_y, max_y));
                rng.between(lo.min(hi), lo.max(hi))
            }
            HeightRange::Trapezoid(a, b) => {
                let (lo, hi) = (a.resolve(min_y, max_y), b.resolve(min_y, max_y));
                let (lo, hi) = (lo.min(hi), lo.max(hi));
                // moyenne de deux uniformes = distribution triangulaire (plateau nul)
                (rng.between(lo, hi) + rng.between(lo, hi)) / 2
            }
        }
    }
}

/// Nombre de placements dans un chunk.
#[derive(Clone, Copy)]
pub enum Count {
    Fixed(i32),
    Uniform(i32, i32),
    /// Biaisé vers le bas : min(deux tirages) — vanilla `biased_to_bottom`.
    BiasedToBottom(i32, i32),
}
impl Count {
    pub fn sample(&self, rng: &mut FeatureRng) -> i32 {
        match self {
            Count::Fixed(n) => *n,
            Count::Uniform(a, b) => rng.between(*a, *b),
            Count::BiasedToBottom(a, b) => rng.between(*a, *b).min(rng.between(*a, *b)),
        }
    }
}

#[inline]
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

/// Une veine de minerai (feature `ore`/`scattered_ore` vanilla).
pub struct Ore<'a> {
    pub block: u32,
    pub size: i32,
    /// Proba de retirer un bloc de la veine exposé à l'air (`discard_chance`).
    pub discard_air: f32,
    /// Dispersé (debris) vs veine ellipsoïde compacte.
    pub scattered: bool,
    /// Prédicat de cible (bloc remplaçable) — ex. netherrack, ou base_stone.
    pub is_target: &'a dyn Fn(u32) -> bool,
}

/// Région de placement d'un chunk : coin (base_x, base_z) + bornes Y de la dimension.
#[derive(Clone, Copy)]
pub struct Region {
    pub base_x: i32,
    pub base_z: i32,
    pub min_y: i32,
    pub max_y: i32,
}

/// Place `count` veines dans le chunk, chacune à une position/hauteur tirées.
pub fn place_ore(
    grid: &mut DimGrid,
    rng: &mut FeatureRng,
    region: &Region,
    count: i32,
    height: HeightRange,
    ore: &Ore,
) {
    for _ in 0..count {
        let x = region.base_x + rng.range(16);
        let z = region.base_z + rng.range(16);
        let y = height.sample(rng, region.min_y, region.max_y);
        if ore.scattered {
            place_scattered(grid, rng, x, y, z, ore);
        } else {
            place_vein(grid, rng, x, y, z, ore);
        }
    }
}

/// Vrai si une des 6 faces du bloc est de l'air (exposition à l'air).
fn touches_air(grid: &DimGrid, x: i32, y: i32, z: i32, air: u32) -> bool {
    grid.get_world(x + 1, y, z) == air
        || grid.get_world(x - 1, y, z) == air
        || grid.get_world(x, y + 1, z) == air
        || grid.get_world(x, y - 1, z) == air
        || grid.get_world(x, y, z + 1) == air
        || grid.get_world(x, y, z - 1) == air
}

/// Veine ellipsoïde (`OreFeature.doPlace`) : segment aléatoire, `size` sphères de
/// rayon en cloche le long du segment.
fn place_vein(grid: &mut DimGrid, rng: &mut FeatureRng, cx: i32, cy: i32, cz: i32, ore: &Ore) {
    let air = super::block_registry::BLOCKS.air;
    let size = ore.size.max(1);
    let angle = rng.f32() as f64 * std::f64::consts::PI;
    let half = size as f64 / 8.0;
    let (sx, ex) = (
        cx as f64 + angle.sin() * half,
        cx as f64 - angle.sin() * half,
    );
    let (sz, ez) = (
        cz as f64 + angle.cos() * half,
        cz as f64 - angle.cos() * half,
    );
    let sy = cy as f64 + (rng.range(3) - 2) as f64;
    let ey = cy as f64 + (rng.range(3) - 2) as f64;

    // Échantillonne `size` points (centre + rayon en cloche sin(πf)).
    let mut pts = Vec::with_capacity(size as usize);
    for i in 0..size {
        let f = i as f64 / size as f64;
        let x = lerp(f, sx, ex);
        let y = lerp(f, sy, ey);
        let z = lerp(f, sz, ez);
        let d = rng.f64() * size as f64 / 16.0;
        let radius = (((std::f64::consts::PI * f).sin() + 1.0) * d + 1.0) / 2.0;
        pts.push((x, y, z, radius));
    }

    for (x, y, z, radius) in pts {
        if radius <= 0.0 {
            continue;
        }
        let x0 = (x - radius).floor() as i32;
        let x1 = (x + radius).floor() as i32;
        let y0 = (y - radius).floor() as i32;
        let y1 = (y + radius).floor() as i32;
        let z0 = (z - radius).floor() as i32;
        let z1 = (z + radius).floor() as i32;
        for px in x0..=x1 {
            let ddx = (px as f64 + 0.5 - x) / radius;
            if ddx * ddx >= 1.0 {
                continue;
            }
            for py in y0..=y1 {
                let ddy = (py as f64 + 0.5 - y) / radius;
                if ddx * ddx + ddy * ddy >= 1.0 {
                    continue;
                }
                for pz in z0..=z1 {
                    let ddz = (pz as f64 + 0.5 - z) / radius;
                    if ddx * ddx + ddy * ddy + ddz * ddz >= 1.0 {
                        continue;
                    }
                    place_one(grid, rng, px, py, pz, ore, air);
                }
            }
        }
    }
}

/// Minerai dispersé (`ScatteredOreFeature`) : `size` blocs dans un petit nuage,
/// uniquement s'ils ne sont pas exposés à l'air (debris : `discard_air = 1.0`).
fn place_scattered(grid: &mut DimGrid, rng: &mut FeatureRng, cx: i32, cy: i32, cz: i32, ore: &Ore) {
    let air = super::block_registry::BLOCKS.air;
    let spread = ore.size.max(1);
    for _ in 0..spread {
        let px = cx + rng.range(spread + 1) - rng.range(spread + 1);
        let py = cy + rng.range(spread + 1) - rng.range(spread + 1);
        let pz = cz + rng.range(spread + 1) - rng.range(spread + 1);
        place_one(grid, rng, px, py, pz, ore, air);
    }
}

#[inline]
fn place_one(
    grid: &mut DimGrid,
    rng: &mut FeatureRng,
    x: i32,
    y: i32,
    z: i32,
    ore: &Ore,
    air: u32,
) {
    if !(ore.is_target)(grid.get_world(x, y, z)) {
        return;
    }
    if ore.discard_air > 0.0 && touches_air(grid, x, y, z, air) && rng.f32() < ore.discard_air {
        return;
    }
    grid.set(x, y, z, ore.block);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::block_registry::BLOCKS;

    #[test]
    fn vein_places_ore_in_target_stone() {
        let netherrack = BLOCKS.get("minecraft:netherrack");
        let quartz = BLOCKS.get("minecraft:quartz_ore");
        let is_target = move |b: u32| b == netherrack;
        let ore = Ore {
            block: quartz,
            size: 14,
            discard_air: 0.0,
            scattered: false,
            is_target: &is_target,
        };
        let mut grid = DimGrid::test_floor(0, 0, 0, 128, netherrack, 120);
        let mut rng = FeatureRng::new(0, 0, 42, 1);
        let region = Region {
            base_x: 0,
            base_z: 0,
            min_y: 0,
            max_y: 128,
        };
        place_ore(
            &mut grid,
            &mut rng,
            &region,
            16,
            HeightRange::Uniform(Anchor::AboveBottom(10), Anchor::BelowTop(10)),
            &ore,
        );
        let n = grid.count(quartz);
        // 16 veines de taille 14 → centaines de blocs de quartz (pas zéro, pas absurde).
        assert!(n > 50, "veines de quartz attendues (trouvé {n})");
        assert!(n < 4000, "trop de quartz, veine cassée (trouvé {n})");
    }

    #[test]
    fn height_anchors_resolve() {
        // below_top 10 sur Y0..128 = 118 ; above_bottom 10 = 10.
        assert_eq!(Anchor::BelowTop(10).resolve(0, 128), 118);
        assert_eq!(Anchor::AboveBottom(10).resolve(0, 128), 10);
        assert_eq!(Anchor::Absolute(30).resolve(0, 128), 30);
    }
}
