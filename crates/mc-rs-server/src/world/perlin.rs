//! Perlin noise generator for Bedrock Edition terrain.
//! Uses gradient noise with fade function interpolation.
//! Supports multi-octave fBm (fractional Brownian motion).

/// Standard Perlin gradient vectors (12 directions for 3D).
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

/// Perlin fade function: 6t⁵ - 15t⁴ + 10t³
#[inline]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Linear interpolation
#[inline]
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

/// Dot product of gradient and distance vector
#[inline]
fn grad(hash: usize, x: f64, y: f64, z: f64) -> f64 {
    let g = &GRAD3[hash % 12];
    g[0] * x + g[1] * y + g[2] * z
}

/// Single-layer Perlin noise generator.
pub struct PerlinNoise {
    perm: [u8; 512],
    offset_x: f64,
    offset_y: f64,
    offset_z: f64,
}

impl PerlinNoise {
    /// Create a new Perlin noise with the given seed.
    pub fn new(seed: i64) -> Self {
        let mut perm = [0u8; 512];

        // Initialize permutation table using a simple LCG seeded RNG
        // This produces the same table for the same seed
        let mut rng_state = seed as u64;

        let mut p = [0u8; 256];
        for (i, item) in p.iter_mut().enumerate() {
            *item = i as u8;
        }

        // Fisher-Yates shuffle
        for i in (1..256).rev() {
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = ((rng_state >> 33) as usize) % (i + 1);
            p.swap(i, j);
        }

        perm[..256].copy_from_slice(&p);
        perm[256..512].copy_from_slice(&p);

        // Generate offsets from seed
        rng_state = seed as u64;
        rng_state = rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let offset_x = (rng_state >> 33) as f64 / (1u64 << 31) as f64 * 256.0;
        rng_state = rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let offset_y = (rng_state >> 33) as f64 / (1u64 << 31) as f64 * 256.0;
        rng_state = rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let offset_z = (rng_state >> 33) as f64 / (1u64 << 31) as f64 * 256.0;

        Self {
            perm,
            offset_x,
            offset_y,
            offset_z,
        }
    }

    /// Sample raw 3D Perlin noise at the given coordinates.
    /// Returns a value roughly in [-1, 1].
    pub fn noise_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        let x = x + self.offset_x;
        let y = y + self.offset_y;
        let z = z + self.offset_z;

        // Find unit cube containing point
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let zi = z.floor() as i32;

        // Relative position within cube
        let xf = x - xi as f64;
        let yf = y - yi as f64;
        let zf = z - zi as f64;

        // Fade curves
        let u = fade(xf);
        let v = fade(yf);
        let w = fade(zf);

        // Hash coordinates of cube corners
        let xi = (xi & 255) as usize;
        let yi = (yi & 255) as usize;
        let zi = (zi & 255) as usize;

        let p = &self.perm;
        let aaa = p[p[p[xi] as usize + yi] as usize + zi] as usize;
        let aba = p[p[p[xi] as usize + yi + 1] as usize + zi] as usize;
        let aab = p[p[p[xi] as usize + yi] as usize + zi + 1] as usize;
        let abb = p[p[p[xi] as usize + yi + 1] as usize + zi + 1] as usize;
        let baa = p[p[p[xi + 1] as usize + yi] as usize + zi] as usize;
        let bba = p[p[p[xi + 1] as usize + yi + 1] as usize + zi] as usize;
        let bab = p[p[p[xi + 1] as usize + yi] as usize + zi + 1] as usize;
        let bbb = p[p[p[xi + 1] as usize + yi + 1] as usize + zi + 1] as usize;

        // Gradient dot products + trilinear interpolation
        lerp(
            w,
            lerp(
                v,
                lerp(u, grad(aaa, xf, yf, zf), grad(baa, xf - 1.0, yf, zf)),
                lerp(
                    u,
                    grad(aba, xf, yf - 1.0, zf),
                    grad(bba, xf - 1.0, yf - 1.0, zf),
                ),
            ),
            lerp(
                v,
                lerp(
                    u,
                    grad(aab, xf, yf, zf - 1.0),
                    grad(bab, xf - 1.0, yf, zf - 1.0),
                ),
                lerp(
                    u,
                    grad(abb, xf, yf - 1.0, zf - 1.0),
                    grad(bbb, xf - 1.0, yf - 1.0, zf - 1.0),
                ),
            ),
        )
    }
}

/// Multi-octave Perlin noise (fractional Brownian motion).
pub struct OctavePerlin {
    octaves: Vec<PerlinNoise>,
    persistence: f64,
    lacunarity: f64,
}

impl OctavePerlin {
    /// Create multi-octave Perlin noise.
    /// Each octave gets a different seed derived from the base seed.
    pub fn new(seed: i64, num_octaves: u32, persistence: f64, lacunarity: f64) -> Self {
        let mut octaves = Vec::with_capacity(num_octaves as usize);
        for i in 0..num_octaves {
            // Each octave gets a unique seed
            let octave_seed = seed.wrapping_add(i as i64 * 1000003);
            octaves.push(PerlinNoise::new(octave_seed));
        }
        Self {
            octaves,
            persistence,
            lacunarity,
        }
    }

    /// Sample multi-octave noise at the given coordinates.
    pub fn noise_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        let mut total = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_value = 0.0;

        for octave in &self.octaves {
            total += octave.noise_3d(x * frequency, y * frequency, z * frequency) * amplitude;
            max_value += amplitude;
            amplitude *= self.persistence;
            frequency *= self.lacunarity;
        }

        total / max_value
    }
}

/// Generate a 3D density grid with sparse sampling and trilinear interpolation.
/// Samples noise at a coarse grid (every x_rate/y_rate/z_rate blocks)
/// then interpolates to fill the full grid.
#[allow(clippy::too_many_arguments)]
pub fn sample_density_grid(
    noise_low: &OctavePerlin,
    noise_high: &OctavePerlin,
    noise_selector: &OctavePerlin,
    chunk_x: i32,
    chunk_z: i32,
    coord_scale: f64,
    height_scale: f64,
    base_height: f64,
    avg_scale: f64,
    stretch_y: f64,
) -> [[[f64; 33]; 5]; 5] {
    let mut grid = [[[0.0f64; 33]; 5]; 5];

    let base_x = chunk_x as f64 * 16.0;
    let base_z = chunk_z as f64 * 16.0;

    #[allow(clippy::needless_range_loop)]
    for gx in 0..5 {
        for gz in 0..5 {
            let world_x = (base_x + gx as f64 * 4.0) / coord_scale;
            let world_z = (base_z + gz as f64 * 4.0) / coord_scale;

            for gy in 0..33 {
                let world_y = gy as f64 * 8.0 / height_scale;

                // Sample the 3 noise layers
                let low = noise_low.noise_3d(world_x, world_y, world_z);
                let high = noise_high.noise_3d(world_x, world_y, world_z);
                let sel = noise_selector.noise_3d(
                    world_x / (coord_scale / 80.0),
                    world_y / (height_scale / 160.0),
                    world_z / (coord_scale / 80.0),
                );

                // Interpolate between low and high based on selector
                let selector = sel.clamp(0.0, 1.0);
                let noise = low + (high - low) * selector;

                // Height adjustment — pushes density negative above base height
                let height_adj = (gy as f64 - base_height) * stretch_y * 128.0 / 256.0 / avg_scale;

                grid[gx][gz][gy] = noise - height_adj;
            }
        }
    }

    grid
}

/// Interpolate the 5x5x33 density grid to get density at any block position.
/// x, z are local chunk coordinates (0..15), y is world Y (0..255).
pub fn interpolate_density(
    grid: &[[[f64; 33]; 5]; 5],
    local_x: usize,
    local_z: usize,
    y: i32,
) -> f64 {
    if !(0..256).contains(&y) {
        return if y < 0 { 1.0 } else { -1.0 };
    }

    // Grid cell indices
    let gx = local_x / 4;
    let gz = local_z / 4;
    let gy = y as usize / 8;

    // Fractional position within grid cell
    let fx = (local_x % 4) as f64 / 4.0;
    let fz = (local_z % 4) as f64 / 4.0;
    let fy = (y as usize % 8) as f64 / 8.0;

    // Clamp grid indices to valid range
    let gx1 = (gx + 1).min(4);
    let gz1 = (gz + 1).min(4);
    let gy1 = (gy + 1).min(32);

    // Trilinear interpolation
    let c000 = grid[gx][gz][gy];
    let c100 = grid[gx1][gz][gy];
    let c010 = grid[gx][gz1][gy];
    let c110 = grid[gx1][gz1][gy];
    let c001 = grid[gx][gz][gy1];
    let c101 = grid[gx1][gz][gy1];
    let c011 = grid[gx][gz1][gy1];
    let c111 = grid[gx1][gz1][gy1];

    let x00 = lerp(fx, c000, c100);
    let x10 = lerp(fx, c010, c110);
    let x01 = lerp(fx, c001, c101);
    let x11 = lerp(fx, c011, c111);

    let z0 = lerp(fz, x00, x10);
    let z1 = lerp(fz, x01, x11);

    lerp(fy, z0, z1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perlin_deterministic() {
        let p1 = PerlinNoise::new(42);
        let p2 = PerlinNoise::new(42);
        for i in 0..20 {
            let x = i as f64 * 0.3;
            assert_eq!(p1.noise_3d(x, 0.0, 0.0), p2.noise_3d(x, 0.0, 0.0));
        }
    }

    #[test]
    fn test_perlin_range() {
        let p = PerlinNoise::new(12345);
        for x in 0..100 {
            for z in 0..100 {
                let v = p.noise_3d(x as f64 * 0.1, 0.0, z as f64 * 0.1);
                assert!(
                    v >= -1.5 && v <= 1.5,
                    "Perlin out of range: {v} at ({x}, 0, {z})"
                );
            }
        }
    }

    #[test]
    fn test_perlin_different_seeds() {
        let p1 = PerlinNoise::new(1);
        let p2 = PerlinNoise::new(2);
        let mut different = false;
        for i in 0..10 {
            if p1.noise_3d(i as f64, 0.0, 0.0) != p2.noise_3d(i as f64, 0.0, 0.0) {
                different = true;
                break;
            }
        }
        assert!(different, "Different seeds should produce different noise");
    }

    #[test]
    fn test_octave_perlin() {
        let op = OctavePerlin::new(42, 8, 0.5, 2.0);
        let v = op.noise_3d(1.0, 2.0, 3.0);
        assert!(v.is_finite());
        assert!(v.abs() <= 2.0, "Octave noise out of range: {v}");
    }

    #[test]
    fn test_density_grid_shape() {
        let low = OctavePerlin::new(1, 4, 0.5, 2.0);
        let high = OctavePerlin::new(2, 4, 0.5, 2.0);
        let sel = OctavePerlin::new(3, 4, 0.5, 2.0);

        let grid = sample_density_grid(&low, &high, &sel, 0, 0, 684.412, 684.412, 8.5, 0.2, 12.0);

        // Grid should be 5x5x33
        assert_eq!(grid.len(), 5);
        assert_eq!(grid[0].len(), 5);
        assert_eq!(grid[0][0].len(), 33);
    }

    #[test]
    fn test_interpolate_density() {
        let low = OctavePerlin::new(42, 4, 0.5, 2.0);
        let high = OctavePerlin::new(43, 4, 0.5, 2.0);
        let sel = OctavePerlin::new(44, 4, 0.5, 2.0);

        let grid = sample_density_grid(&low, &high, &sel, 0, 0, 684.412, 684.412, 8.5, 0.2, 12.0);

        // Should be able to interpolate at any block position
        let d = interpolate_density(&grid, 8, 8, 64);
        assert!(d.is_finite());
    }

    #[test]
    fn test_terrain_has_surface() {
        let low = OctavePerlin::new(42, 8, 0.5, 2.0);
        let high = OctavePerlin::new(43, 8, 0.5, 2.0);
        let sel = OctavePerlin::new(44, 4, 0.5, 2.0);

        let grid = sample_density_grid(&low, &high, &sel, 0, 0, 684.412, 684.412, 8.5, 0.2, 12.0);

        // At Y=0, density should be positive (underground)
        let d_bottom = interpolate_density(&grid, 8, 8, 0);
        // At Y=200, density should be negative (sky)
        let d_top = interpolate_density(&grid, 8, 8, 200);

        assert!(
            d_bottom > d_top,
            "Bottom should be denser than top: bottom={d_bottom}, top={d_top}"
        );
    }
}
