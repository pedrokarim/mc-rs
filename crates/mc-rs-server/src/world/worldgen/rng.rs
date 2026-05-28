//! RNG vanilla Minecraft 1.18+ : `XoroshiroRandomSource` (Xoroshiro128++)
//! et la factory positionnelle `fromHashOf` (md5).
//!
//! Porté fidèlement depuis `net.minecraft.world.level.levelgen.XoroshiroRandomSource`
//! et `RandomSupport`. Les valeurs sont en `u64` : les opérations bit-à-bit et
//! l'arithmétique en complément à deux sont identiques au `long` signé de Java.

use md5::{Digest, Md5};

const GOLDEN_RATIO_64: u64 = 0x9E37_79B9_7F4A_7C15;
const SILVER_RATIO_64: u64 = 0x6A09_E667_F3BC_C909;

/// 2^-53, facteur de `nextDouble` (identique à Java).
const DOUBLE_UNIT: f64 = 1.110_223_024_625_156_5e-16;

#[inline]
fn mix_stafford13(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Décompose une seed 64-bit en état 128-bit mixé (RandomSupport).
fn upgrade_seed_to_128bit(seed: u64) -> (u64, u64) {
    let lo = seed ^ SILVER_RATIO_64;
    let hi = lo.wrapping_add(GOLDEN_RATIO_64);
    (mix_stafford13(lo), mix_stafford13(hi))
}

/// Source aléatoire Xoroshiro128++ vanilla.
#[derive(Clone)]
pub struct XoroshiroRandom {
    lo: u64,
    hi: u64,
}

impl XoroshiroRandom {
    /// Crée une source depuis une seed 64-bit (mixée en 128-bit).
    pub fn from_seed(seed: u64) -> Self {
        let (lo, hi) = upgrade_seed_to_128bit(seed);
        Self::from_parts(lo, hi)
    }

    /// Crée une source depuis l'état 128-bit brut.
    /// Si l'état est entièrement nul, vanilla le remplace par (GOLDEN, SILVER).
    pub fn from_parts(lo: u64, hi: u64) -> Self {
        if (lo | hi) == 0 {
            Self {
                lo: GOLDEN_RATIO_64,
                hi: SILVER_RATIO_64,
            }
        } else {
            Self { lo, hi }
        }
    }

    /// Cœur Xoroshiro128++ : produit 64 bits et avance l'état.
    #[inline]
    pub fn next_long(&mut self) -> u64 {
        let l = self.lo;
        let m = self.hi;
        let n = l.wrapping_add(m).rotate_left(17).wrapping_add(l);
        let m = m ^ l;
        self.lo = l.rotate_left(49) ^ m ^ (m << 21);
        self.hi = m.rotate_left(28);
        n
    }

    /// Les `bits` de poids fort du prochain long (Java `next(int bits)`).
    #[inline]
    fn next_bits(&mut self, bits: u32) -> u64 {
        self.next_long() >> (64 - bits)
    }

    #[inline]
    pub fn next_int(&mut self) -> i32 {
        self.next_bits(32) as u32 as i32
    }

    /// `nextInt(bound)` vanilla (Lemire avec rejet), bound > 0.
    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        debug_assert!(bound > 0);
        let i = bound as u32 as u64;
        let mut l = self.next_int() as u32 as u64;
        let mut m = l.wrapping_mul(i);
        let mut n = m & 0xFFFF_FFFF;
        if n < i {
            // Integer.remainderUnsigned(~bound + 1, bound) = (2^32 - bound) % bound
            let j = (0u32.wrapping_sub(bound as u32) % bound as u32) as u64;
            while n < j {
                l = self.next_int() as u32 as u64;
                m = l.wrapping_mul(i);
                n = m & 0xFFFF_FFFF;
            }
        }
        (m >> 32) as i32
    }

    #[inline]
    pub fn next_double(&mut self) -> f64 {
        (self.next_long() >> 11) as f64 * DOUBLE_UNIT
    }

    #[inline]
    pub fn next_float(&mut self) -> f32 {
        (self.next_bits(24) as f32) * 5.9604645e-8 // 2^-24
    }

    pub fn next_bool(&mut self) -> bool {
        (self.next_long() & 1) != 0
    }

    /// Fabrique positionnelle pour dériver des sources nommées.
    ///
    /// Vanilla : `new XoroshiroPositionalRandomFactory(nextLong(), nextLong())`
    /// — consomme DEUX longs de la source (n'utilise pas l'état courant). Cet
    /// ordre est critique pour la parité de seeding du bruit.
    pub fn fork_positional(&mut self) -> PositionalRandomFactory {
        let lo = self.next_long();
        let hi = self.next_long();
        PositionalRandomFactory { lo, hi }
    }
}

/// `XoroshiroPositionalRandomFactory` : dérive des sources déterministes
/// à partir d'un nom (hash md5) ou d'une position.
#[derive(Clone)]
pub struct PositionalRandomFactory {
    lo: u64,
    hi: u64,
}

impl PositionalRandomFactory {
    /// Dérive une source depuis le hash md5 d'un nom (ex: `"minecraft:temperature"`,
    /// `"octave_-7"`). Le md5 (16 octets) est lu en deux longs big-endian, puis
    /// XORé avec l'état de la factory.
    // Nom calqué sur l'API vanilla `fromHashOf` ; prend `&self` à dessein.
    #[allow(clippy::wrong_self_convention)]
    pub fn from_hash_of(&self, name: &str) -> XoroshiroRandom {
        let digest = Md5::digest(name.as_bytes());
        let l = u64::from_be_bytes(digest[0..8].try_into().unwrap());
        let m = u64::from_be_bytes(digest[8..16].try_into().unwrap());
        XoroshiroRandom::from_parts(l ^ self.lo, m ^ self.hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_same_seed() {
        let mut a = XoroshiroRandom::from_seed(12345);
        let mut b = XoroshiroRandom::from_seed(12345);
        for _ in 0..1000 {
            assert_eq!(a.next_long(), b.next_long());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = XoroshiroRandom::from_seed(1);
        let mut b = XoroshiroRandom::from_seed(2);
        assert_ne!(a.next_long(), b.next_long());
    }

    #[test]
    fn next_int_bound_in_range() {
        let mut r = XoroshiroRandom::from_seed(42);
        for _ in 0..10000 {
            let v = r.next_int_bound(256);
            assert!((0..256).contains(&v));
        }
    }

    #[test]
    fn next_double_unit_interval() {
        let mut r = XoroshiroRandom::from_seed(7);
        for _ in 0..10000 {
            let d = r.next_double();
            assert!((0.0..1.0).contains(&d));
        }
    }

    #[test]
    fn from_hash_of_deterministic_and_distinct() {
        let mut base = XoroshiroRandom::from_seed(999);
        let f = base.fork_positional();
        let mut t1 = f.from_hash_of("minecraft:temperature");
        let mut t2 = f.from_hash_of("minecraft:temperature");
        let mut h = f.from_hash_of("minecraft:vegetation");
        assert_eq!(t1.next_long(), t2.next_long());
        // Noms différents → flux différents.
        let mut t1b = f.from_hash_of("minecraft:temperature");
        assert_ne!(t1b.next_long(), h.next_long());
    }

    #[test]
    fn all_zero_state_is_replaced() {
        let r = XoroshiroRandom::from_parts(0, 0);
        assert_eq!(r.lo, GOLDEN_RATIO_64);
        assert_eq!(r.hi, SILVER_RATIO_64);
    }
}
