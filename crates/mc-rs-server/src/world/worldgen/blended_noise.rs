//! `minecraft:old_blended_noise` — le bruit 3D terrain hérité de la 1.16,
//! toujours utilisé pour la densité de base (surplombs, falaises).
//!
//! Porté depuis `net.minecraft.world.level.levelgen.synth.BlendedNoise`.

use super::perlin::{wrap, PerlinNoise};
use super::rng::XoroshiroRandom;

#[inline]
fn clamped_lerp(a: f64, b: f64, t: f64) -> f64 {
    if t < 0.0 {
        a
    } else if t > 1.0 {
        b
    } else {
        a + t * (b - a)
    }
}

pub struct BlendedNoise {
    min_limit_noise: PerlinNoise,
    max_limit_noise: PerlinNoise,
    main_noise: PerlinNoise,
    xz_multiplier: f64,
    y_multiplier: f64,
    xz_factor: f64,
    y_factor: f64,
    smear_scale_multiplier: f64,
}

impl BlendedNoise {
    /// `xz_scale`, `y_scale`, `xz_factor`, `y_factor`, `smear_scale_multiplier`
    /// proviennent du nœud `old_blended_noise`. La source est dérivée de
    /// `"minecraft:terrain"`.
    pub fn new(
        random: &mut XoroshiroRandom,
        xz_scale: f64,
        y_scale: f64,
        xz_factor: f64,
        y_factor: f64,
        smear_scale_multiplier: f64,
    ) -> Self {
        let min_limit_noise = PerlinNoise::create_legacy_blended(random, -15);
        let max_limit_noise = PerlinNoise::create_legacy_blended(random, -15);
        let main_noise = PerlinNoise::create_legacy_blended(random, -7);
        Self {
            min_limit_noise,
            max_limit_noise,
            main_noise,
            xz_multiplier: 684.412 * xz_scale,
            y_multiplier: 684.412 * y_scale,
            xz_factor,
            y_factor,
            smear_scale_multiplier,
        }
    }

    pub fn compute(&self, block_x: i32, block_y: i32, block_z: i32) -> f64 {
        let d = block_x as f64 * self.xz_multiplier;
        let e = block_y as f64 * self.y_multiplier;
        let f = block_z as f64 * self.xz_multiplier;
        let g = d / self.xz_factor;
        let h = e / self.y_factor;
        let i = f / self.xz_factor;
        let y_smear = self.y_multiplier * self.smear_scale_multiplier;
        let k = y_smear / self.y_factor;

        // Bruit principal → facteur de mélange entre les deux limites.
        let mut main = 0.0;
        let mut o = 1.0;
        for p in 0..8 {
            if let Some(noise) = self.main_noise.get_octave_noise(p) {
                main += noise.noise_with_y(wrap(g * o), wrap(h * o), wrap(i * o), k * o, h * o) / o;
            }
            o /= 2.0;
        }
        let blend = (main / 10.0 + 1.0) / 2.0;
        let skip_min = blend >= 1.0;
        let skip_max = blend <= 0.0;

        let mut min = 0.0;
        let mut max = 0.0;
        o = 1.0;
        for q in 0..16 {
            let s = wrap(d * o);
            let t = wrap(e * o);
            let u = wrap(f * o);
            let v = k * o;
            if !skip_min {
                if let Some(noise) = self.min_limit_noise.get_octave_noise(q) {
                    min += noise.noise_with_y(s, t, u, v, e * o) / o;
                }
            }
            if !skip_max {
                if let Some(noise) = self.max_limit_noise.get_octave_noise(q) {
                    max += noise.noise_with_y(s, t, u, v, e * o) / o;
                }
            }
            o /= 2.0;
        }
        clamped_lerp(min / 512.0, max / 512.0, blend) / 128.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make() -> BlendedNoise {
        let mut r = XoroshiroRandom::from_seed(123)
            .fork_positional()
            .from_hash_of("minecraft:terrain");
        BlendedNoise::new(&mut r, 0.25, 0.125, 80.0, 160.0, 8.0)
    }

    #[test]
    fn deterministic() {
        let a = make();
        let b = make();
        for i in 0..20 {
            let (x, y, z) = (i * 3, i - 10, i * 2);
            assert_eq!(a.compute(x, y, z), b.compute(x, y, z));
        }
    }

    #[test]
    fn bounded_and_varied() {
        let n = make();
        let mut distinct = std::collections::HashSet::new();
        let mut max_abs: f64 = 0.0;
        for x in 0..32 {
            let v = n.compute(x * 4, 40, x * 7);
            distinct.insert((v * 1e6) as i64);
            max_abs = max_abs.max(v.abs());
        }
        assert!(distinct.len() > 20, "bruit trop constant");
        assert!(max_abs < 10.0, "amplitude anormale: {max_abs}");
    }
}
