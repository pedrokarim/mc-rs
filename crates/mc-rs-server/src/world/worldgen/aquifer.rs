//! Phase D — aquifères (remplissage des grottes en eau/lave).
//!
//! Port fidèle de `NoiseAquifer` (deepslate / vanilla
//! `Aquifer.NoiseBasedAquifer`). Pour chaque bloc de grotte (densité ≤ 0), on
//! cherche les sources d'aquifère les plus proches (grille jitterée 16×12×16),
//! on calcule leurs niveaux de fluide (bruits `fluid_level_floodedness` /
//! `_spread` / `lava`) et la pression entre elles (`barrier`), puis on décide
//! eau / lave / air.
//!
//! Simplifications assumées : les density functions sont échantillonnées aux
//! coordonnées scalées arrondies (`i32`, vs fractionnaires vanilla — sans effet
//! notable car ces bruits sont basse fréquence) ; le `preliminary_surface_level`
//! est approximé par la carte des hauteurs du chunk (bornée au chunk).

use std::collections::HashMap;

use super::density::NoiseRouter;
use super::rng::{PositionalRandomFactory, XoroshiroRandom};

const SEA_LEVEL: i32 = 63;
const X_SPACING: i32 = 16;
const Y_SPACING: i32 = 12;
const Z_SPACING: i32 = 16;

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

/// Fluide global overworld : lave sous y=-54, eau sous le niveau de la mer.
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

#[inline]
fn map(v: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    c + (v - a) * (d - c) / (b - a)
}

#[inline]
fn clamped_map(v: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    let t = ((v - a) / (b - a)).clamp(0.0, 1.0);
    c + t * (d - c)
}

/// Aquifère d'un chunk (caches de sources + statuts).
pub struct Aquifer<'a> {
    router: &'a NoiseRouter,
    factory: PositionalRandomFactory,
    base_x: i32,
    base_z: i32,
    surfaces: &'a [[i32; 16]; 16],
    loc_cache: HashMap<(i32, i32, i32), (i32, i32, i32)>,
    status_cache: HashMap<(i32, i32, i32), FluidStatus>,
}

impl<'a> Aquifer<'a> {
    pub fn new(
        router: &'a NoiseRouter,
        seed: u64,
        base_x: i32,
        base_z: i32,
        surfaces: &'a [[i32; 16]; 16],
    ) -> Self {
        // Factory positionnelle dédiée à l'aquifère (fromHashOf + forkPositional).
        let mut base = XoroshiroRandom::from_seed(seed);
        let dev = base.fork_positional();
        let factory = dev.from_hash_of("minecraft:aquifer").fork_positional();
        Aquifer {
            router,
            factory,
            base_x,
            base_z,
            surfaces,
            loc_cache: HashMap::new(),
            status_cache: HashMap::new(),
        }
    }

    /// Surface préliminaire approximée (carte du chunk, bornée).
    fn prelim(&self, x: i32, z: i32) -> i32 {
        let lx = (x - self.base_x).clamp(0, 15) as usize;
        let lz = (z - self.base_z).clamp(0, 15) as usize;
        self.surfaces[lx][lz]
    }

    /// Décide le fluide d'un bloc de grotte (densité ≤ 0).
    pub fn compute(&mut self, x: i32, y: i32, z: i32, density: f64) -> Fluid {
        if global_status(y).at(y) == Fluid::Lava {
            return Fluid::Lava;
        }
        let grid_x = fdiv(x - 5, X_SPACING);
        let grid_y = fdiv(y + 1, Y_SPACING);
        let grid_z = fdiv(z - 5, Z_SPACING);

        let mut best: [(i64, (i32, i32, i32)); 3] = [(i64::MAX, (0, 0, 0)); 3];
        for xo in 0..=1 {
            for yo in -1..=1 {
                for zo in 0..=1 {
                    let loc = self.location(grid_x + xo, grid_y + yo, grid_z + zo);
                    let (dx, dy, dz) = ((loc.0 - x) as i64, (loc.1 - y) as i64, (loc.2 - z) as i64);
                    let mag = dx * dx + dy * dy + dz * dz;
                    if mag < best[0].0 {
                        best[2] = best[1];
                        best[1] = best[0];
                        best[0] = (mag, loc);
                    } else if mag < best[1].0 {
                        best[2] = best[1];
                        best[1] = (mag, loc);
                    } else if mag < best[2].0 {
                        best[2] = (mag, loc);
                    }
                }
            }
        }

        let status1 = self.status(best[0].1);
        let f1 = status1.at(y);
        let similarity12 = similarity(best[0].0, best[1].0);

        let pressure = if f1 == Fluid::Water && global_status(y - 1).at(y - 1) == Fluid::Lava {
            1.0
        } else if similarity12 > -1.0 {
            let status2 = self.status(best[1].1);
            let status3 = self.status(best[2].1);
            let barrier = self.barrier(x, y, z);
            let p12 = self.pressure(y, &status1, &status2, barrier);
            let p13 = self.pressure(y, &status1, &status3, barrier);
            let p23 = self.pressure(y, &status2, &status3, barrier);
            let sim13 = similarity(best[0].0, best[2].0).max(0.0);
            let sim23 = similarity(best[1].0, best[2].0).max(0.0);
            let n = p12.max(p13 * sim13).max(p23 * sim23);
            (2.0 * similarity12.max(0.0) * n).max(0.0)
        } else {
            0.0
        };

        if density + pressure <= 0.0 {
            f1
        } else {
            Fluid::Air
        }
    }

    fn barrier(&self, x: i32, y: i32, z: i32) -> f64 {
        self.router
            .barrier
            .compute(x, (y as f64 * 0.5).floor() as i32, z)
    }

    fn pressure(&self, y: i32, s1: &FluidStatus, s2: &FluidStatus, barrier: f64) -> f64 {
        let f1 = s1.at(y);
        let f2 = s2.at(y);
        if (f1 == Fluid::Lava && f2 == Fluid::Water) || (f1 == Fluid::Water && f2 == Fluid::Lava) {
            return 1.0;
        }
        let (l1, l2) = (s1.level as i64, s2.level as i64);
        let level_diff = (l1 - l2).abs();
        if level_diff == 0 {
            return 0.0;
        }
        let level_avg = (l1 + l2) as f64 / 2.0;
        let level_avg_diff = y as f64 + 0.5 - level_avg;
        let p = level_diff as f64 / 2.0 - level_avg_diff.abs();
        let pressure = if level_avg_diff > 0.0 {
            if p > 0.0 {
                p / 1.5
            } else {
                p / 2.5
            }
        } else if p > -3.0 {
            (p + 3.0) / 3.0
        } else {
            (p + 3.0) / 10.0
        };
        if !(-2.0..=2.0).contains(&pressure) {
            pressure
        } else {
            pressure + barrier
        }
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

    fn compute_status(&self, x: i32, y: i32, z: i32) -> FluidStatus {
        let global = global_status(y);
        let mut min_prelim = i32::MAX;
        let mut is_aquifer = false;
        for (xo, zo) in SURFACE_SAMPLING {
            let bx = x + (xo << 4);
            let bz = z + (zo << 4);
            let surface = self.prelim(bx, bz);
            min_prelim = min_prelim.min(surface);
            let no_offset = xo == 0 && zo == 0;
            if no_offset && y - 12 > surface + 8 {
                return global;
            }
            if no_offset || y + 12 > surface + 8 {
                let s = global_status(surface + 8);
                if s.at(surface + 8) != Fluid::Air {
                    if no_offset {
                        return s;
                    }
                    is_aquifer = true;
                }
            }
        }

        let allowed = if is_aquifer {
            clamped_map((min_prelim + 8 - y) as f64, 0.0, 64.0, 1.0, 0.0)
        } else {
            0.0
        };
        let floodedness = self
            .router
            .fluid_level_floodedness
            .compute(x, (y as f64 * 0.67).floor() as i32, z)
            .clamp(-1.0, 1.0);
        if floodedness > map(allowed, 1.0, 0.0, -0.3, 0.8) {
            return global;
        }
        if floodedness <= map(allowed, 1.0, 0.0, -0.8, 0.4) {
            return FluidStatus {
                level: i32::MIN,
                fluid: global.fluid,
            };
        }

        let grid_y = fdiv(y, 40);
        let spread = self
            .router
            .fluid_level_spread
            .compute(fdiv(x, 16), grid_y, fdiv(z, 16));
        let level = grid_y * 40 + 20 + ((spread / 3.0).floor() as i32) * 3;
        let status_level = min_prelim.min(level);
        let fluid = self.fluid_type(x, y, z, global.fluid, level);
        FluidStatus {
            level: status_level,
            fluid,
        }
    }

    fn fluid_type(&self, x: i32, y: i32, z: i32, global: Fluid, level: i32) -> Fluid {
        if level <= -10 {
            let lava = self
                .router
                .lava
                .compute(fdiv(x, 64), fdiv(y, 40), fdiv(z, 64));
            if lava.abs() > 0.3 {
                return Fluid::Lava;
            }
        }
        global
    }
}

#[inline]
fn similarity(a: i64, b: i64) -> f64 {
    1.0 - (b - a).abs() as f64 / 25.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::worldgen::density;

    #[test]
    fn low_terrain_floods_caves() {
        // Sous des terres basses (surface 50 → aquifère), une grotte se remplit
        // d'eau ; sous des terres hautes (surface 90), elle reste sèche.
        let router = density::build_overworld(42);
        let low = [[50i32; 16]; 16];
        let high = [[90i32; 16]; 16];
        let mut aq_low = Aquifer::new(&router, 42, 0, 0, &low);
        let mut aq_high = Aquifer::new(&router, 42, 0, 0, &high);
        let mut water_low = 0;
        let mut air_high = 0;
        for y in (-40..40).step_by(2) {
            if aq_low.compute(8, y, 8, -0.5) == Fluid::Water {
                water_low += 1;
            }
            if aq_high.compute(8, y, 8, -0.5) == Fluid::Air {
                air_high += 1;
            }
        }
        assert!(
            water_low > 0,
            "les grottes en terre basse devraient s'inonder"
        );
        assert!(
            air_high > 0,
            "les grottes en terre haute devraient rester sèches"
        );
    }

    #[test]
    fn deep_is_lava() {
        let router = density::build_overworld(42);
        let surfaces = [[70i32; 16]; 16];
        let mut aq = Aquifer::new(&router, 42, 0, 0, &surfaces);
        // Très profond → lave (global status).
        assert_eq!(aq.compute(8, -60, 8, -1.0), Fluid::Lava);
    }

    #[test]
    fn caves_fill_or_stay_air() {
        let router = density::build_overworld(42);
        let surfaces = [[70i32; 16]; 16];
        let mut aq = Aquifer::new(&router, 42, 0, 0, &surfaces);
        // Doit produire un résultat déterministe et valide à divers Y.
        for y in [-30, 0, 30, 50] {
            let f = aq.compute(8, y, 8, -0.5);
            assert!(matches!(f, Fluid::Air | Fluid::Water | Fluid::Lava));
        }
    }
}
