//! Slime — split on death, various sizes.
//! Magma cube partage la mécanique mais différente.

#[derive(Debug, Clone)]
pub struct Slime {
    pub size: u8, // 1=small, 2=medium, 4=big
}

/// HP par size (1=1, 2=4, 4=16).
pub fn hp_for_size(size: u8) -> f32 {
    (size as f32).powi(2)
}

/// Damage par size (1=0, 2=2, 4=4).
pub fn damage_for_size(size: u8) -> f32 {
    match size {
        1 => 0.0,
        2 => 2.0,
        _ => 4.0,
    }
}

impl Slime {
    pub fn new(size: u8) -> Self {
        Self { size }
    }

    /// When killed, splits into 2-4 smaller slimes (if size > 1).
    pub fn split_on_death(&self) -> Vec<Slime> {
        if self.size == 1 {
            return vec![];
        }
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let new_size = self.size / 2;
        let count = rng.gen_range(2..=4);
        (0..count).map(|_| Slime::new(new_size)).collect()
    }

    pub fn is_big(&self) -> bool {
        self.size == 4
    }
    pub fn is_small(&self) -> bool {
        self.size == 1
    }

    /// Slime chunk spawn chance (~10% of eligible chunks).
    pub fn chunk_is_slime_chunk(chunk_x: i32, chunk_z: i32, world_seed: u64) -> bool {
        // Vanilla java PRNG formula.
        let seed = world_seed
            .wrapping_add((chunk_x as i64 as u64).wrapping_mul(0x4c1906))
            .wrapping_add(
                (chunk_x as i64 as u64)
                    .wrapping_mul(chunk_x as i64 as u64)
                    .wrapping_mul(0x5ac0db),
            )
            .wrapping_add((chunk_z as i64 as u64).wrapping_mul(0x5f24f) & 0xffffffff)
            .wrapping_add(
                (chunk_z as i64 as u64)
                    .wrapping_mul(chunk_z as i64 as u64)
                    .wrapping_mul(0x4307a7),
            )
            ^ 0x3ad8025f;
        seed.wrapping_mul(seed.wrapping_mul(6364136223846793005))
            .wrapping_add(1442695040888963407)
            .is_multiple_of(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_splits() {
        let s = Slime::new(4);
        let splits = s.split_on_death();
        assert!(!splits.is_empty());
        assert!(splits.iter().all(|ss| ss.size == 2));
    }

    #[test]
    fn small_no_split() {
        let s = Slime::new(1);
        assert!(s.split_on_death().is_empty());
    }

    #[test]
    fn hp_scales_with_size() {
        assert_eq!(hp_for_size(1), 1.0);
        assert_eq!(hp_for_size(2), 4.0);
        assert_eq!(hp_for_size(4), 16.0);
    }
}
