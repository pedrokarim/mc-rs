//! Horse / Donkey / Mule attributes — port PMMP partial.
//! Random gen de health / jump strength / movement speed au spawn.

use rand::Rng;

/// Vanilla formulas PMMP `Horse::generateAttributes()`.
pub fn random_horse_health() -> f32 {
    let mut rng = rand::thread_rng();
    // 15-30 HP (vanilla : 15 + rand*9 + rand*9 + rand*9)
    15.0 + (rng.gen::<f32>() * 9.0 + rng.gen::<f32>() * 9.0 + rng.gen::<f32>() * 9.0) / 3.0
}

pub fn random_horse_jump_strength() -> f32 {
    let mut rng = rand::thread_rng();
    // 0.4 - 1.0 (vanilla 0.4 + rand*0.2 + rand*0.2 + rand*0.2)
    0.4 + (rng.gen::<f32>() * 0.2 + rng.gen::<f32>() * 0.2 + rng.gen::<f32>() * 0.2)
}

pub fn random_horse_movement_speed() -> f32 {
    let mut rng = rand::thread_rng();
    // 0.1125 - 0.3375
    (0.45 + rng.gen::<f32>() * 0.3 + rng.gen::<f32>() * 0.3 + rng.gen::<f32>() * 0.3) * 0.25
}

#[derive(Debug, Clone, Copy)]
pub struct HorseVariant {
    pub base_color: u8, // 0-6
    pub markings: u8,   // 0-4
}

impl HorseVariant {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            base_color: rng.gen_range(0..7),
            markings: rng.gen_range(0..5),
        }
    }

    pub fn as_meta(&self) -> u32 {
        (self.base_color as u32) | ((self.markings as u32) << 8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horse_health_in_range() {
        let h = random_horse_health();
        assert!(h >= 15.0 && h <= 30.0);
    }

    #[test]
    fn horse_jump_in_range() {
        let j = random_horse_jump_strength();
        assert!(j >= 0.4 && j <= 1.0);
    }
}
