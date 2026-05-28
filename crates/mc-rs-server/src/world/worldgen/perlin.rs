//! Bruit Perlin amélioré vanilla 1.18+ : `ImprovedNoise` (1 octave),
//! `PerlinNoise` (multi-octaves), `NormalNoise` (2 perlins combinés).
//!
//! Porté depuis `net.minecraft.world.level.levelgen.synth.{ImprovedNoise,
//! PerlinNoise, NormalNoise}` et `SimplexNoise.GRADIENT`. Constantes exactes.

use serde::Deserialize;

use super::rng::XoroshiroRandom;

/// Table de gradients (16 vecteurs) partagée Perlin/Simplex vanilla.
#[rustfmt::skip]
const GRADIENT: [[f64; 3]; 16] = [
    [1.0, 1.0, 0.0], [-1.0, 1.0, 0.0], [1.0, -1.0, 0.0], [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0], [1.0, 0.0, -1.0], [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0], [0.0, -1.0, 1.0], [0.0, 1.0, -1.0], [0.0, -1.0, -1.0],
    [1.0, 1.0, 0.0], [0.0, -1.0, 1.0], [-1.0, 1.0, 0.0], [0.0, -1.0, -1.0],
];

#[inline]
fn grad_dot(grad_index: i32, x: f64, y: f64, z: f64) -> f64 {
    let g = GRADIENT[(grad_index & 15) as usize];
    g[0] * x + g[1] * y + g[2] * z
}

/// Fondu quintique vanilla `Mth.smoothstep` : 6t^5 - 15t^4 + 10t^3.
#[inline]
fn smoothstep(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn lerp3(
    tx: f64,
    ty: f64,
    tz: f64,
    v000: f64,
    v100: f64,
    v010: f64,
    v110: f64,
    v001: f64,
    v101: f64,
    v011: f64,
    v111: f64,
) -> f64 {
    lerp(
        tx,
        lerp(ty, lerp(tz, v000, v001), lerp(tz, v010, v011)),
        lerp(ty, lerp(tz, v100, v101), lerp(tz, v110, v111)),
    )
}

/// `Mth.floor` vanilla : floor vers le bas, y compris pour les négatifs.
#[inline]
fn mth_floor(d: f64) -> i32 {
    let i = d as i32;
    if d < i as f64 {
        i - 1
    } else {
        i
    }
}

/// `PerlinNoise.wrap` : repli sur [-2^24, 2^24) pour garder la précision f64.
#[inline]
pub fn wrap(value: f64) -> f64 {
    value - ((value / 3.355_443_2e7 + 0.5).floor()) * 3.355_443_2e7
}

/// Bruit Perlin amélioré sur un seul octave (permutation + offset aléatoire).
pub struct ImprovedNoise {
    xo: f64,
    yo: f64,
    zo: f64,
    p: [u8; 256],
}

impl ImprovedNoise {
    pub fn new(random: &mut XoroshiroRandom) -> Self {
        let xo = random.next_double() * 256.0;
        let yo = random.next_double() * 256.0;
        let zo = random.next_double() * 256.0;
        let mut p = [0u8; 256];
        for (i, slot) in p.iter_mut().enumerate() {
            *slot = i as u8;
        }
        for i in 0..256usize {
            let j = random.next_int_bound(256 - i as i32) as usize;
            p.swap(i, i + j);
        }
        Self { xo, yo, zo, p }
    }

    #[inline]
    fn p(&self, i: i32) -> i32 {
        self.p[(i & 255) as usize] as i32
    }

    /// Échantillon de base (sans correction verticale).
    #[inline]
    pub fn noise(&self, x: f64, y: f64, z: f64) -> f64 {
        self.noise_with_y(x, y, z, 0.0, 0.0)
    }

    /// Échantillon avec la correction verticale vanilla (y_scale / y_max).
    pub fn noise_with_y(&self, x: f64, y: f64, z: f64, y_scale: f64, y_max: f64) -> f64 {
        let dx = x + self.xo;
        let dy = y + self.yo;
        let dz = z + self.zo;
        let ix = mth_floor(dx);
        let iy = mth_floor(dy);
        let iz = mth_floor(dz);
        let fx = dx - ix as f64;
        let fy = dy - iy as f64;
        let fz = dz - iz as f64;

        let off_y = if y_scale != 0.0 {
            let p = if y_max >= 0.0 && y_max < fy {
                y_max
            } else {
                fy
            };
            (p / y_scale + 1.0e-7).floor() * y_scale
        } else {
            0.0
        };

        self.sample_and_lerp(ix, iy, iz, fx, fy - off_y, fz, fy)
    }

    #[allow(clippy::too_many_arguments)]
    fn sample_and_lerp(
        &self,
        ix: i32,
        iy: i32,
        iz: i32,
        dx: f64,
        grad_dy: f64,
        dz: f64,
        fade_dy: f64,
    ) -> f64 {
        let a = self.p(ix);
        let b = self.p(ix + 1);
        let aa = self.p(a + iy);
        let ab = self.p(a + iy + 1);
        let ba = self.p(b + iy);
        let bb = self.p(b + iy + 1);

        let v000 = grad_dot(self.p(aa + iz), dx, grad_dy, dz);
        let v100 = grad_dot(self.p(ba + iz), dx - 1.0, grad_dy, dz);
        let v010 = grad_dot(self.p(ab + iz), dx, grad_dy - 1.0, dz);
        let v110 = grad_dot(self.p(bb + iz), dx - 1.0, grad_dy - 1.0, dz);
        let v001 = grad_dot(self.p(aa + iz + 1), dx, grad_dy, dz - 1.0);
        let v101 = grad_dot(self.p(ba + iz + 1), dx - 1.0, grad_dy, dz - 1.0);
        let v011 = grad_dot(self.p(ab + iz + 1), dx, grad_dy - 1.0, dz - 1.0);
        let v111 = grad_dot(self.p(bb + iz + 1), dx - 1.0, grad_dy - 1.0, dz - 1.0);

        let tx = smoothstep(dx);
        let ty = smoothstep(fade_dy);
        let tz = smoothstep(dz);
        lerp3(tx, ty, tz, v000, v100, v010, v110, v001, v101, v011, v111)
    }
}

/// Paramètres d'un bruit (fichier `data/worldgen/noise/*.json`).
#[derive(Deserialize, Clone, Debug)]
pub struct NoiseParameters {
    #[serde(rename = "firstOctave")]
    pub first_octave: i32,
    pub amplitudes: Vec<f64>,
}

/// Bruit Perlin multi-octaves.
pub struct PerlinNoise {
    octaves: Vec<Option<ImprovedNoise>>,
    amplitudes: Vec<f64>,
    lowest_freq_input_factor: f64,
    lowest_freq_value_factor: f64,
}

impl PerlinNoise {
    pub fn create(random: &mut XoroshiroRandom, first_octave: i32, amplitudes: &[f64]) -> Self {
        let n = amplitudes.len();
        let factory = random.fork_positional();
        let mut octaves: Vec<Option<ImprovedNoise>> = Vec::with_capacity(n);
        for (k, &amp) in amplitudes.iter().enumerate() {
            if amp != 0.0 {
                let octave = first_octave + k as i32;
                let mut octave_rng = factory.from_hash_of(&format!("octave_{octave}"));
                octaves.push(Some(ImprovedNoise::new(&mut octave_rng)));
            } else {
                octaves.push(None);
            }
        }
        let lowest_freq_input_factor = 2f64.powi(first_octave);
        let lowest_freq_value_factor = 2f64.powi(n as i32 - 1) / (2f64.powi(n as i32) - 1.0);
        Self {
            octaves,
            amplitudes: amplitudes.to_vec(),
            lowest_freq_input_factor,
            lowest_freq_value_factor,
        }
    }

    /// Constructeur *legacy* utilisé par `old_blended_noise` (toutes amplitudes
    /// = 1, donc aucun octave sauté). Vanilla crée l'octave d'indice `j` en
    /// premier, puis `j-1`…0, en consommant la source séquentiellement.
    pub fn create_legacy_blended(random: &mut XoroshiroRandom, first_octave: i32) -> Self {
        let n = (-first_octave + 1) as usize;
        let j = (-first_octave) as usize;
        let mut octaves: Vec<Option<ImprovedNoise>> = (0..n).map(|_| None).collect();
        octaves[j] = Some(ImprovedNoise::new(random));
        for k in (0..j).rev() {
            octaves[k] = Some(ImprovedNoise::new(random));
        }
        let lowest_freq_input_factor = 2f64.powi(first_octave);
        let lowest_freq_value_factor = 2f64.powi(n as i32 - 1) / (2f64.powi(n as i32) - 1.0);
        Self {
            octaves,
            amplitudes: vec![1.0; n],
            lowest_freq_input_factor,
            lowest_freq_value_factor,
        }
    }

    /// Octave `i` indexé du plus haute fréquence (i=0) vers la plus basse,
    /// comme `PerlinNoise.getOctaveNoise` vanilla.
    #[inline]
    pub fn get_octave_noise(&self, i: usize) -> Option<&ImprovedNoise> {
        self.octaves[self.octaves.len() - 1 - i].as_ref()
    }

    pub fn get_value(&self, x: f64, y: f64, z: f64) -> f64 {
        let mut value = 0.0;
        let mut input_factor = self.lowest_freq_input_factor;
        let mut value_factor = self.lowest_freq_value_factor;
        for (i, octave) in self.octaves.iter().enumerate() {
            if let Some(noise) = octave {
                let g = noise.noise(
                    wrap(x * input_factor),
                    wrap(y * input_factor),
                    wrap(z * input_factor),
                );
                value += self.amplitudes[i] * g * value_factor;
            }
            input_factor *= 2.0;
            value_factor /= 2.0;
        }
        value
    }
}

/// Bruit « normal » vanilla : deux Perlin à des fréquences décalées,
/// normalisé par `value_factor`. C'est la brique utilisée par les density
/// functions (`minecraft:noise`, `shifted_noise`, …).
pub struct NormalNoise {
    first: PerlinNoise,
    second: PerlinNoise,
    value_factor: f64,
}

/// Décalage d'entrée entre les deux Perlin (vanilla `INPUT_FACTOR`).
const INPUT_FACTOR: f64 = 1.018_126_888_217_522_7;

impl NormalNoise {
    pub fn create(random: &mut XoroshiroRandom, params: &NoiseParameters) -> Self {
        let first = PerlinNoise::create(random, params.first_octave, &params.amplitudes);
        let second = PerlinNoise::create(random, params.first_octave, &params.amplitudes);

        let mut min = i32::MAX;
        let mut max = i32::MIN;
        for (idx, &amp) in params.amplitudes.iter().enumerate() {
            if amp != 0.0 {
                min = min.min(idx as i32);
                max = max.max(idx as i32);
            }
        }
        let value_factor = 0.166_666_666_666_666_66 / expected_deviation(max - min);
        Self {
            first,
            second,
            value_factor,
        }
    }

    #[inline]
    pub fn get_value(&self, x: f64, y: f64, z: f64) -> f64 {
        (self.first.get_value(x, y, z)
            + self
                .second
                .get_value(x * INPUT_FACTOR, y * INPUT_FACTOR, z * INPUT_FACTOR))
            * self.value_factor
    }
}

#[inline]
fn expected_deviation(i: i32) -> f64 {
    0.1 * (1.0 + 1.0 / (i + 1) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deriver(seed: u64) -> super::super::rng::PositionalRandomFactory {
        XoroshiroRandom::from_seed(seed).fork_positional()
    }

    #[test]
    fn improved_noise_deterministic_and_bounded() {
        let mut r = XoroshiroRandom::from_seed(123);
        let n = ImprovedNoise::new(&mut r);
        let mut max_abs: f64 = 0.0;
        for i in 0..50 {
            let v = n.noise(i as f64 * 0.1, 7.0, i as f64 * 0.3);
            max_abs = max_abs.max(v.abs());
        }
        // Le Perlin amélioré est borné ~[-1, 1].
        assert!(max_abs <= 1.0001, "valeur Perlin hors borne: {max_abs}");
    }

    #[test]
    fn normal_noise_deterministic() {
        let params = NoiseParameters {
            first_octave: -8,
            amplitudes: vec![0.5, 1.0, 2.0, 1.0, 2.0, 1.0, 0.0, 2.0, 0.0],
        };
        let f = deriver(42);
        let mut r1 = f.from_hash_of("minecraft:cave_cheese");
        let mut r2 = f.from_hash_of("minecraft:cave_cheese");
        let a = NormalNoise::create(&mut r1, &params);
        let b = NormalNoise::create(&mut r2, &params);
        for i in 0..20 {
            let p = i as f64 * 1.7;
            assert_eq!(a.get_value(p, p * 0.5, -p), b.get_value(p, p * 0.5, -p));
        }
    }

    #[test]
    fn normal_noise_varies_across_space() {
        let params = NoiseParameters {
            first_octave: -7,
            amplitudes: vec![1.0, 1.0, 1.0, 1.0],
        };
        let mut r = deriver(7).from_hash_of("minecraft:continentalness");
        let noise = NormalNoise::create(&mut r, &params);
        let mut distinct = std::collections::HashSet::new();
        for x in 0..40 {
            let v = noise.get_value(x as f64 * 30.0, 0.0, 0.0);
            distinct.insert((v * 1e6) as i64);
        }
        assert!(
            distinct.len() > 30,
            "bruit trop constant: {}",
            distinct.len()
        );
    }
}
