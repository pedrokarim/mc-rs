//! Magma Cube — Nether slime variant.

#[derive(Debug, Clone)]
pub struct MagmaCube {
    pub size: u8, // 1, 2, 4
    pub on_fire: bool,
}

pub fn attack_damage(size: u8) -> f32 {
    match size {
        1 => 3.0,
        2 => 4.0,
        _ => 6.0,
    }
}

pub fn hp(size: u8) -> f32 {
    (size as f32).powi(2)
}

/// Fire immune.
pub fn immune_to_fire() -> bool { true }
/// Doesn't take fall damage.
pub fn immune_to_fall() -> bool { true }

impl MagmaCube {
    pub fn new(size: u8) -> Self {
        Self { size, on_fire: false }
    }

    pub fn split_on_death(&self) -> Vec<MagmaCube> {
        if self.size == 1 {
            return vec![];
        }
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let count = rng.gen_range(2..=4);
        let new_size = self.size / 2;
        (0..count).map(|_| MagmaCube::new(new_size)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_splits_to_medium() {
        let m = MagmaCube::new(4);
        let splits = m.split_on_death();
        assert!(splits.iter().all(|s| s.size == 2));
    }
}
