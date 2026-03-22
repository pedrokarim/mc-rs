/// XorShift128 Random Number Generator.
/// Port of PocketMine-MP's `pocketmine\utils\Random` class.
/// Produces identical sequences for the same seed.
pub struct Random {
    seed: i64,
    x: i64,
    y: i64,
    z: i64,
    w: i64,
}

const X: i64 = 123456789;
const Y: i64 = 362436069;
const Z: i64 = 521288629;
const W: i64 = 88675123;

impl Random {
    pub fn new(seed: i64) -> Self {
        let mut r = Self {
            seed: 0,
            x: 0,
            y: 0,
            z: 0,
            w: 0,
        };
        r.set_seed(seed);
        r
    }

    pub fn set_seed(&mut self, seed: i64) {
        self.seed = seed;
        self.x = X ^ seed;
        // PHP precedence: & > ^ > |
        // (Y ^ (seed << 17)) | (((seed >> 15) & 0x7fffffff) & 0xffffffff)
        self.y = (Y ^ (seed << 17)) | (((seed >> 15) & 0x7fff_ffff) & 0xffff_ffff);
        self.z = (Z ^ (seed << 31)) | (((seed >> 1) & 0x7fff_ffff) & 0xffff_ffff);
        self.w = (W ^ (seed << 18)) | (((seed >> 14) & 0x7fff_ffff) & 0xffff_ffff);
    }

    #[allow(dead_code)]
    pub fn get_seed(&self) -> i64 {
        self.seed
    }

    /// Returns a signed 32-bit integer.
    pub fn next_signed_int(&mut self) -> i32 {
        let t = (self.x ^ (self.x << 11)) & 0xffff_ffff;

        self.x = self.y;
        self.y = self.z;
        self.z = self.w;
        self.w = (self.w ^ ((self.w >> 19) & 0x7fff_ffff) ^ (t ^ ((t >> 8) & 0x7fff_ffff)))
            & 0xffff_ffff;

        self.w as i32
    }

    /// Returns an unsigned 31-bit integer (0 to 0x7fffffff).
    pub fn next_int(&mut self) -> i32 {
        self.next_signed_int() & 0x7fff_ffff
    }

    /// Returns a float between 0.0 and 1.0 (inclusive).
    pub fn next_float(&mut self) -> f64 {
        self.next_int() as f64 / 0x7fff_ffff_i64 as f64
    }

    /// Returns an integer in [0, bound).
    pub fn next_bounded_int(&mut self, bound: i32) -> i32 {
        self.next_int() % bound
    }

    /// Returns an integer in [start, end] (inclusive).
    #[allow(dead_code)]
    pub fn next_range(&mut self, start: i32, end: i32) -> i32 {
        start + (self.next_int() % (end + 1 - start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        let mut r1 = Random::new(42);
        let mut r2 = Random::new(42);
        for _ in 0..100 {
            assert_eq!(r1.next_int(), r2.next_int());
        }
    }

    #[test]
    fn test_next_float_range() {
        let mut r = Random::new(12345);
        for _ in 0..1000 {
            let f = r.next_float();
            assert!((0.0..=1.0).contains(&f));
        }
    }

    #[test]
    fn test_next_bounded_int_range() {
        let mut r = Random::new(99);
        for _ in 0..1000 {
            let v = r.next_bounded_int(256);
            assert!((0..256).contains(&v));
        }
    }

    #[test]
    fn test_different_seeds_differ() {
        let mut r1 = Random::new(1);
        let mut r2 = Random::new(2);
        let mut same = true;
        for _ in 0..10 {
            if r1.next_int() != r2.next_int() {
                same = false;
                break;
            }
        }
        assert!(!same);
    }
}
