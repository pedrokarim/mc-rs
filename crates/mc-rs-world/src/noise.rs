//! Perlin noise implementation for terrain generation.
//!
//! Provides seed-based 2D and 3D Perlin noise with octave (fBm) support.

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

/// 3D gradient vectors for Perlin noise (12 directions).
const GRAD3: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

/// Improved Perlin fade function: 6t^5 - 15t^4 + 10t^3.
#[inline]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Linear interpolation.
#[inline]
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

/// Seed-based Perlin noise generator.
pub struct PerlinNoise {
    perm: [u8; 512],
}

impl PerlinNoise {
    /// Create a new Perlin noise generator from a seed.
    pub fn new(seed: u64) -> Self {
        let mut table: Vec<u8> = (0..=255).collect();
        let mut rng = StdRng::seed_from_u64(seed);
        table.shuffle(&mut rng);

        let mut perm = [0u8; 512];
        perm[..256].copy_from_slice(&table);
        perm[256..].copy_from_slice(&table);
        Self { perm }
    }

    /// Hash helper using the permutation table.
    #[inline]
    fn hash(&self, x: i32, y: i32, z: i32) -> usize {
        let x = (x & 255) as usize;
        let y = (y & 255) as usize;
        let z = (z & 255) as usize;
        self.perm[self.perm[self.perm[x] as usize + y] as usize + z] as usize % 12
    }

    /// 2D Perlin noise at (x, z). Returns a value in approximately [-1, 1].
    pub fn noise_2d(&self, x: f64, z: f64) -> f64 {
        // Grid cell coordinates
        let xi = x.floor() as i32;
        let zi = z.floor() as i32;

        // Local coordinates within the cell [0, 1)
        let xf = x - x.floor();
        let zf = z - z.floor();

        // Fade curves
        let u = fade(xf);
        let v = fade(zf);

        // Hash the 4 corners (y=0 for 2D)
        let g00 = self.hash(xi, 0, zi);
        let g10 = self.hash(xi + 1, 0, zi);
        let g01 = self.hash(xi, 0, zi + 1);
        let g11 = self.hash(xi + 1, 0, zi + 1);

        // Dot products with gradient vectors (using x,z components)
        let d00 = GRAD3[g00][0] * xf + GRAD3[g00][2] * zf;
        let d10 = GRAD3[g10][0] * (xf - 1.0) + GRAD3[g10][2] * zf;
        let d01 = GRAD3[g01][0] * xf + GRAD3[g01][2] * (zf - 1.0);
        let d11 = GRAD3[g11][0] * (xf - 1.0) + GRAD3[g11][2] * (zf - 1.0);

        // Bilinear interpolation
        let x1 = lerp(u, d00, d10);
        let x2 = lerp(u, d01, d11);
        lerp(v, x1, x2)
    }

    /// 3D Perlin noise at (x, y, z). Returns a value in approximately [-1, 1].
    pub fn noise_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let zi = z.floor() as i32;

        let xf = x - x.floor();
        let yf = y - y.floor();
        let zf = z - z.floor();

        let u = fade(xf);
        let v = fade(yf);
        let w = fade(zf);

        // Hash the 8 corners
        let g000 = self.hash(xi, yi, zi);
        let g100 = self.hash(xi + 1, yi, zi);
        let g010 = self.hash(xi, yi + 1, zi);
        let g110 = self.hash(xi + 1, yi + 1, zi);
        let g001 = self.hash(xi, yi, zi + 1);
        let g101 = self.hash(xi + 1, yi, zi + 1);
        let g011 = self.hash(xi, yi + 1, zi + 1);
        let g111 = self.hash(xi + 1, yi + 1, zi + 1);

        // Dot products
        let d000 = dot3(g000, xf, yf, zf);
        let d100 = dot3(g100, xf - 1.0, yf, zf);
        let d010 = dot3(g010, xf, yf - 1.0, zf);
        let d110 = dot3(g110, xf - 1.0, yf - 1.0, zf);
        let d001 = dot3(g001, xf, yf, zf - 1.0);
        let d101 = dot3(g101, xf - 1.0, yf, zf - 1.0);
        let d011 = dot3(g011, xf, yf - 1.0, zf - 1.0);
        let d111 = dot3(g111, xf - 1.0, yf - 1.0, zf - 1.0);

        // Trilinear interpolation
        let x1 = lerp(u, d000, d100);
        let x2 = lerp(u, d010, d110);
        let y1 = lerp(v, x1, x2);

        let x3 = lerp(u, d001, d101);
        let x4 = lerp(u, d011, d111);
        let y2 = lerp(v, x3, x4);

        lerp(w, y1, y2)
    }
}

/// Dot product of a gradient vector with a distance vector.
#[inline]
fn dot3(grad_idx: usize, x: f64, y: f64, z: f64) -> f64 {
    let g = &GRAD3[grad_idx];
    g[0] * x + g[1] * y + g[2] * z
}

/// Multi-octave fractal Brownian motion noise.
pub struct OctaveNoise {
    octaves: Vec<PerlinNoise>,
    lacunarity: f64,
    persistence: f64,
}

impl OctaveNoise {
    /// Create octave noise with N layers. Each octave uses `seed + i`.
    pub fn new(seed: u64, num_octaves: usize, lacunarity: f64, persistence: f64) -> Self {
        let octaves = (0..num_octaves)
            .map(|i| PerlinNoise::new(seed.wrapping_add(i as u64)))
            .collect();
        Self {
            octaves,
            lacunarity,
            persistence,
        }
    }

    /// Sample 2D octave noise. Returns a value roughly in [-1, 1].
    pub fn sample_2d(&self, x: f64, z: f64) -> f64 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_amplitude = 0.0;

        for octave in &self.octaves {
            value += octave.noise_2d(x * frequency, z * frequency) * amplitude;
            max_amplitude += amplitude;
            amplitude *= self.persistence;
            frequency *= self.lacunarity;
        }

        value / max_amplitude
    }

    /// Sample 3D octave noise. Returns a value roughly in [-1, 1].
    pub fn sample_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_amplitude = 0.0;

        for octave in &self.octaves {
            value += octave.noise_3d(x * frequency, y * frequency, z * frequency) * amplitude;
            max_amplitude += amplitude;
            amplitude *= self.persistence;
            frequency *= self.lacunarity;
        }

        value / max_amplitude
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_2d_is_deterministic() {
        let n1 = PerlinNoise::new(42);
        let n2 = PerlinNoise::new(42);
        for i in 0..100 {
            let x = i as f64 * 0.37;
            let z = i as f64 * 0.53;
            assert_eq!(n1.noise_2d(x, z), n2.noise_2d(x, z));
        }
    }

    #[test]
    fn noise_2d_range() {
        let n = PerlinNoise::new(12345);
        for i in 0..10000 {
            let x = (i as f64 * 0.137) - 500.0;
            let z = (i as f64 * 0.251) - 300.0;
            let v = n.noise_2d(x, z);
            assert!(
                v >= -1.5 && v <= 1.5,
                "noise_2d out of range: {v} at ({x}, {z})"
            );
        }
    }

    #[test]
    fn noise_3d_range() {
        let n = PerlinNoise::new(54321);
        for i in 0..10000 {
            let x = (i as f64 * 0.1) - 500.0;
            let y = (i as f64 * 0.2) - 500.0;
            let z = (i as f64 * 0.3) - 500.0;
            let v = n.noise_3d(x, y, z);
            assert!(
                v >= -1.5 && v <= 1.5,
                "noise_3d out of range: {v} at ({x}, {y}, {z})"
            );
        }
    }

    #[test]
    fn different_seeds_different_output() {
        let n1 = PerlinNoise::new(42);
        let n2 = PerlinNoise::new(99999);
        let mut diff_count = 0;
        for i in 0..100 {
            let x = i as f64 * 1.7 + 0.5;
            let z = i as f64 * 2.3 + 0.5;
            if (n1.noise_2d(x, z) - n2.noise_2d(x, z)).abs() > 0.01 {
                diff_count += 1;
            }
        }
        assert!(
            diff_count > 20,
            "Different seeds should produce different output, got {diff_count} differences"
        );
    }

    #[test]
    fn octave_noise_deterministic() {
        let o1 = OctaveNoise::new(42, 6, 2.0, 0.5);
        let o2 = OctaveNoise::new(42, 6, 2.0, 0.5);
        for i in 0..50 {
            let x = i as f64 * 0.1;
            let z = i as f64 * 0.2;
            assert_eq!(o1.sample_2d(x, z), o2.sample_2d(x, z));
        }
    }

    #[test]
    fn noise_continuity() {
        let n = PerlinNoise::new(99);
        let step = 0.001;
        for i in 0..1000 {
            let x = i as f64 * 0.1;
            let z = 5.0;
            let v1 = n.noise_2d(x, z);
            let v2 = n.noise_2d(x + step, z);
            let diff = (v1 - v2).abs();
            assert!(diff < 0.1, "Noise not continuous: diff={diff} at x={x}");
        }
    }
}
