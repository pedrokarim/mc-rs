//! Phase D — aquifères (remplissage des grottes en eau/lave).
//!
//! Port de vanilla `Aquifer.NoiseBasedAquifer` (1.18+). Pour chaque bloc de grotte (densité ≤ 0), on
//! cherche les trois sources d'aquifère les plus proches (grille jitterée 16×12×16), on calcule leurs
//! statuts de fluide (niveau + type, à partir de la surface préliminaire et des bruits
//! `fluid_level_floodedness` / `fluid_level_spread` / `lava`), puis la pression entre elles : une
//! pression trop forte (deux niveaux différents qui se touchent) pose une **barrière de pierre**, sinon le
//! bloc prend le fluide de la source la plus proche (ou reste de l'air au-dessus de son niveau).
//!
//! deepslate, d'où venait la première version, garde son aquifère désactivé (« WIP: Noise aquifers don't
//! work yet ») ; ce port suit vanilla et se vérifie contre les chunks enregistrés par BDS
//! (dev-docs/14-WORLD-GENERATION.md, « Parité avec BDS »).

use std::collections::HashMap;

use super::density::NoiseRouter;
use super::rng::{PositionalRandomFactory, XoroshiroRandom};

const SEA_LEVEL: i32 = 63;
const X_SPACING: i32 = 16;
const Y_SPACING: i32 = 12;
const Z_SPACING: i32 = 16;
/// Niveau « très en dessous du monde » (vanilla `DimensionType.WAY_BELOW_MIN_Y`) : aquifère vide.
const WAY_BELOW_MIN_Y: i32 = -2032;

/// `[xOffset, zOffset]` (en chunks) d'échantillonnage de surface vanilla.
const SURFACE_SAMPLING: [(i32, i32); 13] = [
    (-2, -1),
    (-1, -1),
    (0, -1),
    (1, -1),
    (-3, 0),
    (-2, 0),
    (-1, 0),
    (0, 0),
    (1, 0),
    (-2, 1),
    (-1, 1),
    (0, 1),
    (1, 1),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fluid {
    Air,
    Water,
    Lava,
}

#[derive(Clone, Copy)]
struct FluidStatus {
    level: i32,
    fluid: Fluid,
}

impl FluidStatus {
    #[inline]
    fn at(&self, y: i32) -> Fluid {
        if y < self.level {
            self.fluid
        } else {
            Fluid::Air
        }
    }
}

/// Fluide global overworld (vanilla `globalFluidPicker`) : lave sous y = -54, eau sous le niveau de la mer.
#[inline]
fn global_status(y: i32) -> FluidStatus {
    if y < -54 {
        FluidStatus {
            level: -54,
            fluid: Fluid::Lava,
        }
    } else {
        FluidStatus {
            level: SEA_LEVEL,
            fluid: Fluid::Water,
        }
    }
}

#[inline]
fn fdiv(a: i32, b: i32) -> i32 {
    a.div_euclid(b)
}

/// `Mth.map`.
#[inline]
fn map(v: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    c + (v - a) / (b - a) * (d - c)
}

/// `Mth.clampedMap`.
#[inline]
fn clamped_map(v: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    let t = ((v - a) / (b - a)).clamp(0.0, 1.0);
    c + t * (d - c)
}

/// `NoiseBasedAquifer.similarity` : proximité de deux distances au carré (1 = égales, ≤ 0 = éloignées).
#[inline]
fn similarity(a: i32, b: i32) -> f64 {
    1.0 - (b - a).abs() as f64 / 25.0
}

/// Aquifère (caches de sources + statuts), partagé entre les chunks d'une même génération.
pub struct Aquifer<'a> {
    router: &'a NoiseRouter,
    factory: PositionalRandomFactory,
    loc_cache: HashMap<(i32, i32, i32), (i32, i32, i32)>,
    status_cache: HashMap<(i32, i32, i32), FluidStatus>,
    prelim_cache: HashMap<(i32, i32), i32>,
}

impl<'a> Aquifer<'a> {
    pub fn new(router: &'a NoiseRouter, seed: u64) -> Self {
        // Vanilla `RandomState.aquiferRandom` : `random.fromHashOf("minecraft:aquifer").forkPositional()`.
        let mut base = XoroshiroRandom::from_seed(seed);
        let dev = base.fork_positional();
        let factory = dev.from_hash_of("minecraft:aquifer").fork_positional();
        Aquifer {
            router,
            factory,
            loc_cache: HashMap::new(),
            status_cache: HashMap::new(),
            prelim_cache: HashMap::new(),
        }
    }

    /// Surface préliminaire (vanilla `NoiseChunk.preliminarySurfaceLevel`) : fonction
    /// `preliminary_surface_level` du router, évaluée au coin de la case de 4 × 4 blocs, à y = 0.
    fn prelim(&mut self, x: i32, z: i32) -> i32 {
        let key = (x >> 2, z >> 2);
        if let Some(&v) = self.prelim_cache.get(&key) {
            return v;
        }
        let v = self
            .router
            .preliminary_surface_level
            .compute(key.0 << 2, 0, key.1 << 2)
            .floor() as i32;
        self.prelim_cache.insert(key, v);
        v
    }

    /// Hauteur au-dessus de laquelle tout vide de la zone `[x0, x1] × [z0, z1]` est de l'air, sans calcul
    /// (raccourci exact de `compute`). Les trois sources consultées pour un bloc (x, y, z) sont à moins de
    /// 20 blocs à l'horizontale et à y ≥ y - 22 ; une source dont l'altitude dépasse sa surface préliminaire
    /// de plus de 20 prend le statut global (eau sous y = 63), qui donne de l'air à partir de y = 63 sans
    /// pression entre sources. D'où : y > max(surface préliminaire à 20 blocs près) + 42 et y ≥ 63 ⇒ air.
    pub fn dry_above(&mut self, x0: i32, z0: i32, x1: i32, z1: i32) -> i32 {
        let mut max_surface = i32::MIN;
        for qx in ((x0 - 20) >> 2)..=((x1 + 20) >> 2) {
            for qz in ((z0 - 20) >> 2)..=((z1 + 20) >> 2) {
                max_surface = max_surface.max(self.prelim(qx << 2, qz << 2));
            }
        }
        (max_surface + 42).max(SEA_LEVEL - 1)
    }

    /// Substance d'un bloc de grotte (densité ≤ 0), vanilla `computeSubstance` : `None` = barrière
    /// (le bloc reste de la pierre), sinon le fluide ou l'air.
    pub fn compute(&mut self, x: i32, y: i32, z: i32, density: f64) -> Option<Fluid> {
        if density > 0.0 {
            return None;
        }
        if global_status(y).at(y) == Fluid::Lava {
            return Some(Fluid::Lava);
        }
        let grid_x = fdiv(x - 5, X_SPACING);
        let grid_y = fdiv(y + 1, Y_SPACING);
        let grid_z = fdiv(z - 5, Z_SPACING);

        // Trois sources les plus proches ; à distance égale, la dernière visitée l'emporte (`>=` vanilla).
        let mut dist = [i32::MAX; 3];
        let mut loc = [(0, 0, 0); 3];
        for xo in 0..=1 {
            for yo in -1..=1 {
                for zo in 0..=1 {
                    let l = self.location(grid_x + xo, grid_y + yo, grid_z + zo);
                    let (dx, dy, dz) = (l.0 - x, l.1 - y, l.2 - z);
                    let d = dx * dx + dy * dy + dz * dz;
                    if dist[0] >= d {
                        loc = [l, loc[0], loc[1]];
                        dist = [d, dist[0], dist[1]];
                    } else if dist[1] >= d {
                        loc = [loc[0], l, loc[1]];
                        dist = [dist[0], d, dist[1]];
                    } else if dist[2] >= d {
                        loc[2] = l;
                        dist[2] = d;
                    }
                }
            }
        }

        let status1 = self.status(loc[0]);
        let similarity12 = similarity(dist[0], dist[1]);
        let fluid = status1.at(y);
        if similarity12 <= 0.0 {
            return Some(fluid);
        }
        if fluid == Fluid::Water && global_status(y - 1).at(y - 1) == Fluid::Lava {
            return Some(fluid);
        }

        let mut barrier = None;
        let status2 = self.status(loc[1]);
        let p12 = similarity12 * self.pressure(x, y, z, &mut barrier, &status1, &status2);
        if density + p12 > 0.0 {
            return None;
        }
        let status3 = self.status(loc[2]);
        let similarity13 = similarity(dist[0], dist[2]);
        if similarity13 > 0.0 {
            let p13 = similarity12
                * similarity13
                * self.pressure(x, y, z, &mut barrier, &status1, &status3);
            if density + p13 > 0.0 {
                return None;
            }
        }
        let similarity23 = similarity(dist[1], dist[2]);
        if similarity23 > 0.0 {
            let p23 = similarity12
                * similarity23
                * self.pressure(x, y, z, &mut barrier, &status2, &status3);
            if density + p23 > 0.0 {
                return None;
            }
        }
        Some(fluid)
    }

    /// Vanilla `calculatePressure` : pression entre deux aquifères au niveau `y`, plus le bruit de barrière
    /// (évalué au plus une fois par bloc) quand elle reste modérée.
    fn pressure(
        &self,
        x: i32,
        y: i32,
        z: i32,
        barrier: &mut Option<f64>,
        s1: &FluidStatus,
        s2: &FluidStatus,
    ) -> f64 {
        let (f1, f2) = (s1.at(y), s2.at(y));
        if (f1 == Fluid::Lava && f2 == Fluid::Water) || (f1 == Fluid::Water && f2 == Fluid::Lava) {
            return 2.0;
        }
        let level_diff = (s1.level - s2.level).abs();
        if level_diff == 0 {
            return 0.0;
        }
        let level_avg = 0.5 * (s1.level + s2.level) as f64;
        let level_avg_diff = y as f64 + 0.5 - level_avg;
        let half_diff = level_diff as f64 / 2.0;
        let o = half_diff - level_avg_diff.abs();
        let q = if level_avg_diff > 0.0 {
            let p = o;
            if p > 0.0 {
                p / 1.5
            } else {
                p / 2.5
            }
        } else {
            let p = 3.0 + o;
            if p > 0.0 {
                p / 3.0
            } else {
                p / 10.0
            }
        };
        let r = if (-2.0..=2.0).contains(&q) {
            *barrier.get_or_insert_with(|| self.router.barrier.compute(x, y, z))
        } else {
            0.0
        };
        2.0 * (r + q)
    }

    fn location(&mut self, gx: i32, gy: i32, gz: i32) -> (i32, i32, i32) {
        if let Some(&l) = self.loc_cache.get(&(gx, gy, gz)) {
            return l;
        }
        let mut r = self.factory.at(gx, gy, gz);
        let loc = (
            gx * X_SPACING + r.next_int_bound(10),
            gy * Y_SPACING + r.next_int_bound(9),
            gz * Z_SPACING + r.next_int_bound(10),
        );
        self.loc_cache.insert((gx, gy, gz), loc);
        loc
    }

    fn status(&mut self, loc: (i32, i32, i32)) -> FluidStatus {
        let key = (
            fdiv(loc.0, X_SPACING),
            fdiv(loc.1, Y_SPACING),
            fdiv(loc.2, Z_SPACING),
        );
        if let Some(&s) = self.status_cache.get(&key) {
            return s;
        }
        let s = self.compute_status(loc.0, loc.1, loc.2);
        self.status_cache.insert(key, s);
        s
    }

    /// Vanilla `computeFluid` : statut d'un aquifère à sa source.
    fn compute_status(&mut self, x: i32, y: i32, z: i32) -> FluidStatus {
        let global = global_status(y);
        let mut min_surface = i32::MAX;
        let above = y + 12;
        let below = y - 12;
        let mut fluid_present = false;
        for (xo, zo) in SURFACE_SAMPLING {
            let bx = x + (xo << 4);
            let bz = z + (zo << 4);
            let surface = self.prelim(bx, bz);
            let surface_fluid_y = surface + 8;
            let at_source = xo == 0 && zo == 0;
            if at_source && below > surface_fluid_y {
                return global;
            }
            let reaches = above > surface_fluid_y;
            if reaches || at_source {
                let s = global_status(surface_fluid_y);
                if s.at(surface_fluid_y) != Fluid::Air {
                    if at_source {
                        fluid_present = true;
                    }
                    if reaches {
                        return s;
                    }
                }
            }
            min_surface = min_surface.min(surface);
        }
        let level = self.surface_level(x, y, z, &global, min_surface, fluid_present);
        FluidStatus {
            level,
            fluid: self.fluid_type(x, y, z, &global, level),
        }
    }

    /// Vanilla `computeSurfaceLevel`.
    fn surface_level(
        &self,
        x: i32,
        y: i32,
        z: i32,
        global: &FluidStatus,
        max_surface: i32,
        fluid_present: bool,
    ) -> i32 {
        // Région du deep dark (`OverworldBiomeBuilder.isDeepDarkRegion`) : jamais inondée.
        let deep_dark = self.router.erosion.compute(x, y, z) < -0.225
            && self.router.depth.compute(x, y, z) > 0.9;
        let (partially, fully) = if deep_dark {
            (-1.0, -1.0)
        } else {
            let allowed = if fluid_present {
                clamped_map((max_surface + 8 - y) as f64, 0.0, 64.0, 1.0, 0.0)
            } else {
                0.0
            };
            let floodedness = self
                .router
                .fluid_level_floodedness
                .compute(x, y, z)
                .clamp(-1.0, 1.0);
            let fully_threshold = map(allowed, 1.0, 0.0, -0.3, 0.8);
            let partially_threshold = map(allowed, 1.0, 0.0, -0.8, 0.4);
            (
                floodedness - partially_threshold,
                floodedness - fully_threshold,
            )
        };
        if fully > 0.0 {
            global.level
        } else if partially > 0.0 {
            self.randomized_level(x, y, z, max_surface)
        } else {
            WAY_BELOW_MIN_Y
        }
    }

    /// Vanilla `computeRandomizedFluidSurfaceLevel` : niveau par cellule de 16 × 40 × 16 blocs.
    fn randomized_level(&self, x: i32, y: i32, z: i32, max_surface: i32) -> i32 {
        let grid_y = fdiv(y, 40);
        let base = grid_y * 40 + 20;
        let spread = self
            .router
            .fluid_level_spread
            .compute(fdiv(x, 16), grid_y, fdiv(z, 16))
            * 10.0;
        let offset = (spread / 3.0).floor() as i32 * 3;
        max_surface.min(base + offset)
    }

    /// Vanilla `computeFluidType` : lave dans les aquifères profonds selon le bruit `lava`.
    fn fluid_type(&self, x: i32, y: i32, z: i32, global: &FluidStatus, level: i32) -> Fluid {
        if level <= -10 && level != WAY_BELOW_MIN_Y && global.fluid != Fluid::Lava {
            let lava = self
                .router
                .lava
                .compute(fdiv(x, 64), fdiv(y, 40), fdiv(z, 64));
            if lava.abs() > 0.3 {
                return Fluid::Lava;
            }
        }
        global.fluid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::worldgen::density;

    #[test]
    fn aquifer_produces_water_air_and_barriers() {
        // Sur une grande zone de grottes, l'aquifère doit produire de l'eau (zones inondées), de l'air
        // (zones sèches) et des barrières de pierre entre aquifères de niveaux différents.
        let router = density::build_overworld(42);
        let mut aq = Aquifer::new(&router, 42);
        let (mut water, mut air, mut barrier) = (0, 0, 0);
        for x in (-400..400).step_by(7) {
            for y in (-50..60).step_by(3) {
                match aq.compute(x, y, 8, -0.05) {
                    Some(Fluid::Water) => water += 1,
                    Some(Fluid::Air) => air += 1,
                    Some(Fluid::Lava) => {}
                    None => barrier += 1,
                }
            }
        }
        assert!(water > 0, "aucune eau d'aquifère");
        assert!(air > 0, "aucune grotte sèche");
        assert!(barrier > 0, "aucune barrière");
    }

    #[test]
    fn deterministic_between_instances() {
        // Deux aquifères (ex. deux chunks) doivent renvoyer le MÊME fluide pour
        // la même position monde → pas de discontinuité aux frontières de chunk.
        let router = density::build_overworld(42);
        let mut a = Aquifer::new(&router, 42);
        let mut b = Aquifer::new(&router, 42);
        for x in (0..80).step_by(3) {
            for y in (-40..55).step_by(5) {
                assert_eq!(
                    a.compute(x, y, 11, -0.5),
                    b.compute(x, y, 11, -0.5),
                    "désaccord en ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn deep_is_lava() {
        let router = density::build_overworld(42);
        let mut aq = Aquifer::new(&router, 42);
        // Très profond → lave (global status).
        assert_eq!(aq.compute(8, -60, 8, -1.0), Some(Fluid::Lava));
    }

    /// Le raccourci `dry_above` ne change aucun résultat : au-dessus, `compute` rend toujours de l'air.
    #[test]
    fn dry_above_is_exact() {
        let router = density::build_overworld(4817747595384169343);
        let mut aq = Aquifer::new(&router, 4817747595384169343);
        for (cx, cz) in [(0, 0), (-3, -4), (5, 2), (-20, 11)] {
            let (x0, z0) = (cx * 16, cz * 16);
            let dry = aq.dry_above(x0, z0, x0 + 15, z0 + 15);
            for x in (x0..x0 + 16).step_by(3) {
                for z in (z0..z0 + 16).step_by(3) {
                    for y in (dry + 1..dry + 60).step_by(2) {
                        assert_eq!(
                            aq.compute(x, y, z, -0.01),
                            Some(Fluid::Air),
                            "({x}, {y}, {z})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn solid_stays_solid() {
        let router = density::build_overworld(42);
        let mut aq = Aquifer::new(&router, 42);
        assert_eq!(aq.compute(8, 0, 8, 0.5), None);
    }
}
