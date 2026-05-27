use super::random::Random;

/// Simplex noise generator.
/// Port of PocketMine-MP's `Simplex` + `Noise` classes.
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

const SQRT3: f64 = 1.732_050_808_068_872_5;
const F2: f64 = 0.5 * (SQRT3 - 1.0);
const G2: f64 = (3.0 - SQRT3) / 6.0;
const G22: f64 = G2 * 2.0 - 1.0;
const F3: f64 = 1.0 / 3.0;
const G3: f64 = 1.0 / 6.0;

pub struct Simplex {
    perm: [usize; 512],
    offset_x: f64,
    offset_y: f64,
    offset_z: f64,
    octaves: u32,
    persistence: f64,
    expansion: f64,
}

impl Simplex {
    pub fn new(random: &mut Random, octaves: u32, persistence: f64, expansion: f64) -> Self {
        let offset_x = random.next_float() * 256.0;
        let offset_y = random.next_float() * 256.0;
        let offset_z = random.next_float() * 256.0;

        let mut perm = [0usize; 512];

        for item in perm.iter_mut().take(256) {
            *item = random.next_bounded_int(256) as usize;
        }

        for i in 0..256 {
            let pos = random.next_bounded_int(256 - i as i32) as usize + i;
            perm.swap(i, pos);
            perm[i + 256] = perm[i];
        }

        // Dummy call to match PMMP's RNG state consumption
        random.next_signed_int();

        Self {
            perm,
            offset_x,
            offset_y,
            offset_z,
            octaves,
            persistence,
            expansion,
        }
    }

    /// Raw 3D simplex noise, returns value in roughly [-1, 1].
    pub fn get_noise_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        let x = x + self.offset_x;
        let y = y + self.offset_y;
        let z = z + self.offset_z;

        // Skew input space
        // PHP's (int) truncates toward zero, not floor()
        let s = (x + y + z) * F3;
        let i = (x + s) as i64;
        let j = (y + s) as i64;
        let k = (z + s) as i64;
        let t = (i + j + k) as f64 * G3;

        // Unskew cell origin
        let x0 = x - (i as f64 - t);
        let y0 = y - (j as f64 - t);
        let z0 = z - (k as f64 - t);

        // Determine simplex
        let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
            if y0 >= z0 {
                (1, 0, 0, 1, 1, 0) // X Y Z
            } else if x0 >= z0 {
                (1, 0, 0, 1, 0, 1) // X Z Y
            } else {
                (0, 0, 1, 1, 0, 1) // Z X Y
            }
        } else if y0 < z0 {
            (0, 0, 1, 0, 1, 1) // Z Y X
        } else if x0 < z0 {
            (0, 1, 0, 0, 1, 1) // Y Z X
        } else {
            (0, 1, 0, 1, 1, 0) // Y X Z
        };

        let x1 = x0 - i1 as f64 + G3;
        let y1 = y0 - j1 as f64 + G3;
        let z1 = z0 - k1 as f64 + G3;
        let x2 = x0 - i2 as f64 + 2.0 * G3;
        let y2 = y0 - j2 as f64 + 2.0 * G3;
        let z2 = z0 - k2 as f64 + 2.0 * G3;
        let x3 = x0 - 1.0 + 3.0 * G3;
        let y3 = y0 - 1.0 + 3.0 * G3;
        let z3 = z0 - 1.0 + 3.0 * G3;

        let ii = (i & 255) as usize;
        let jj = (j & 255) as usize;
        let kk = (k & 255) as usize;

        let perm = &self.perm;
        let mut n = 0.0;

        let t0 = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
        if t0 > 0.0 {
            let gi0 = GRAD3[perm[ii + perm[jj + perm[kk]]] % 12];
            n += t0 * t0 * t0 * t0 * (gi0[0] * x0 + gi0[1] * y0 + gi0[2] * z0);
        }

        let t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
        if t1 > 0.0 {
            let gi1 = GRAD3[perm[ii + i1 + perm[jj + j1 + perm[kk + k1]]] % 12];
            n += t1 * t1 * t1 * t1 * (gi1[0] * x1 + gi1[1] * y1 + gi1[2] * z1);
        }

        let t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
        if t2 > 0.0 {
            let gi2 = GRAD3[perm[ii + i2 + perm[jj + j2 + perm[kk + k2]]] % 12];
            n += t2 * t2 * t2 * t2 * (gi2[0] * x2 + gi2[1] * y2 + gi2[2] * z2);
        }

        let t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
        if t3 > 0.0 {
            let gi3 = GRAD3[perm[ii + 1 + perm[jj + 1 + perm[kk + 1]]] % 12];
            n += t3 * t3 * t3 * t3 * (gi3[0] * x3 + gi3[1] * y3 + gi3[2] * z3);
        }

        32.0 * n
    }

    /// Raw 2D simplex noise.
    #[allow(dead_code)]
    pub fn get_noise_2d(&self, x: f64, y: f64) -> f64 {
        let x = x + self.offset_x;
        let y = y + self.offset_y;

        let s = (x + y) * F2;
        // PHP's (int) truncates toward zero
        let i = (x + s) as i64;
        let j = (y + s) as i64;
        let t = (i + j) as f64 * G2;

        let x0 = x - (i as f64 - t);
        let y0 = y - (j as f64 - t);

        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };

        let x1 = x0 - i1 as f64 + G2;
        let y1 = y0 - j1 as f64 + G2;
        let x2 = x0 + G22;
        let y2 = y0 + G22;

        let ii = (i & 255) as usize;
        let jj = (j & 255) as usize;

        let mut n = 0.0;

        let t0 = 0.5 - x0 * x0 - y0 * y0;
        if t0 > 0.0 {
            let gi0 = GRAD3[self.perm[ii + self.perm[jj]] % 12];
            n += t0 * t0 * t0 * t0 * (gi0[0] * x0 + gi0[1] * y0);
        }

        let t1 = 0.5 - x1 * x1 - y1 * y1;
        if t1 > 0.0 {
            let gi1 = GRAD3[self.perm[ii + i1 + self.perm[jj + j1]] % 12];
            n += t1 * t1 * t1 * t1 * (gi1[0] * x1 + gi1[1] * y1);
        }

        let t2 = 0.5 - x2 * x2 - y2 * y2;
        if t2 > 0.0 {
            let gi2 = GRAD3[self.perm[ii + 1 + self.perm[jj + 1]] % 12];
            n += t2 * t2 * t2 * t2 * (gi2[0] * x2 + gi2[1] * y2);
        }

        70.0 * n
    }

    /// Multi-octave 2D noise.
    #[allow(dead_code)]
    pub fn noise_2d(&self, x: f64, z: f64, normalized: bool) -> f64 {
        let mut result = 0.0;
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut max = 0.0;

        let x = x * self.expansion;
        let z = z * self.expansion;

        for _ in 0..self.octaves {
            result += self.get_noise_2d(x * freq, z * freq) * amp;
            max += amp;
            freq *= 2.0;
            amp *= self.persistence;
        }

        if normalized {
            result /= max;
        }

        result
    }

    /// Multi-octave 3D noise.
    pub fn noise_3d(&self, x: f64, y: f64, z: f64, normalized: bool) -> f64 {
        let mut result = 0.0;
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut max = 0.0;

        let x = x * self.expansion;
        let y = y * self.expansion;
        let z = z * self.expansion;

        for _ in 0..self.octaves {
            result += self.get_noise_3d(x * freq, y * freq, z * freq) * amp;
            max += amp;
            freq *= 2.0;
            amp *= self.persistence;
        }

        if normalized {
            result /= max;
        }

        result
    }

    /// Generate a 3D noise grid with sparse sampling and trilinear interpolation.
    /// Matches PMMP's `Noise::getFastNoise3D`.
    #[allow(clippy::too_many_arguments)]
    pub fn get_fast_noise_3d(
        &self,
        x_size: usize,
        y_size: usize,
        z_size: usize,
        x_sampling_rate: usize,
        y_sampling_rate: usize,
        z_sampling_rate: usize,
        x: i32,
        y: i32,
        z: i32,
    ) -> Vec<Vec<Vec<f64>>> {
        // Allocate 3D array [x_size+1][z_size+1][y_size+1]
        let mut noise = vec![vec![vec![0.0f64; y_size + 1]; z_size + 1]; x_size + 1];

        // Sample at sparse grid points
        let mut xx = 0;
        while xx <= x_size {
            let mut zz = 0;
            while zz <= z_size {
                let mut yy = 0;
                while yy <= y_size {
                    noise[xx][zz][yy] = self.noise_3d(
                        (x as i64 + xx as i64) as f64,
                        (y as i64 + yy as i64) as f64,
                        (z as i64 + zz as i64) as f64,
                        true,
                    );
                    yy += y_sampling_rate;
                }
                zz += z_sampling_rate;
            }
            xx += x_sampling_rate;
        }

        // Trilinear interpolation (optimized version from PMMP)
        let x_lerp_step = 1.0 / x_sampling_rate as f64;
        let y_lerp_step = 1.0 / y_sampling_rate as f64;
        let z_lerp_step = 1.0 / z_sampling_rate as f64;

        let mut left_x = 0;
        while left_x < x_size {
            let right_x = left_x + x_sampling_rate;

            let mut left_z = 0;
            while left_z < z_size {
                let right_z = left_z + z_sampling_rate;

                let mut left_y = 0;
                while left_y < y_size {
                    let right_y = left_y + y_sampling_rate;

                    // Corner samples
                    let c000 = noise[left_x][left_z][left_y];
                    let c100 = noise[right_x][left_z][left_y];
                    let c001 = noise[left_x][left_z][right_y];
                    let c101 = noise[right_x][left_z][right_y];
                    let c010 = noise[left_x][right_z][left_y];
                    let c110 = noise[right_x][right_z][left_y];
                    let c011 = noise[left_x][right_z][right_y];
                    let c111 = noise[right_x][right_z][right_y];

                    for x_step in 0..x_sampling_rate {
                        let xx = left_x + x_step;
                        let dx2 = x_step as f64 * x_lerp_step;
                        let dx1 = 1.0 - dx2;

                        let x00 = c000 * dx1 + c100 * dx2;
                        let x01 = c001 * dx1 + c101 * dx2;
                        let x10 = c010 * dx1 + c110 * dx2;
                        let x11 = c011 * dx1 + c111 * dx2;

                        for z_step in 0..z_sampling_rate {
                            let zz = left_z + z_step;
                            let dz2 = z_step as f64 * z_lerp_step;
                            let dz1 = 1.0 - dz2;

                            let z0 = x00 * dz1 + x10 * dz2;
                            let z1 = x01 * dz1 + x11 * dz2;

                            // Skip first row if both steps are 0 (already sampled)
                            let y_start = if x_step == 0 && z_step == 0 { 1 } else { 0 };
                            for y_step in y_start..y_sampling_rate {
                                let yy = left_y + y_step;
                                let dy2 = y_step as f64 * y_lerp_step;
                                let dy1 = 1.0 - dy2;

                                noise[xx][zz][yy] = dy1 * z0 + dy2 * z1;
                            }
                        }
                    }

                    left_y += y_sampling_rate;
                }
                left_z += z_sampling_rate;
            }
            left_x += x_sampling_rate;
        }

        noise
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplex_deterministic() {
        let mut rng = Random::new(42);
        let s1 = Simplex::new(&mut rng, 4, 0.25, 1.0 / 32.0);

        let mut rng2 = Random::new(42);
        let s2 = Simplex::new(&mut rng2, 4, 0.25, 1.0 / 32.0);

        for i in 0..10 {
            let x = i as f64 * 0.5;
            assert_eq!(s1.get_noise_3d(x, 0.0, 0.0), s2.get_noise_3d(x, 0.0, 0.0));
        }
    }

    #[test]
    fn test_simplex_range() {
        let mut rng = Random::new(123);
        let s = Simplex::new(&mut rng, 4, 0.25, 1.0 / 32.0);

        for x in 0..50 {
            for z in 0..50 {
                let v = s.get_noise_3d(x as f64, 0.0, z as f64);
                assert!(
                    (-32.0..=32.0).contains(&v),
                    "noise out of range: {v} at ({x}, 0, {z})"
                );
            }
        }
    }

    #[test]
    fn test_fast_noise_3d() {
        let mut rng = Random::new(42);
        let s = Simplex::new(&mut rng, 4, 0.25, 1.0 / 32.0);

        let noise = s.get_fast_noise_3d(16, 8, 16, 4, 8, 4, 0, 0, 0);

        assert_eq!(noise.len(), 17);
        assert_eq!(noise[0].len(), 17);
        assert_eq!(noise[0][0].len(), 9);
    }
}
