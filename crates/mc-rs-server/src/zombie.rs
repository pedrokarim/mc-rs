//! Zombie — attraction to villagers, conversion, baby jockey.

use rand::Rng;

#[derive(Debug, Clone)]
pub struct Zombie {
    pub age: i32,
    pub is_villager: bool,
    pub village_id: Option<u64>,
    pub converting_to_villager: bool,
    pub conversion_ticks: u32,
    pub reinforcement_chance: f32,
}

/// Conversion duration after curing (3000-6000 ticks).
pub const CONVERSION_MIN: u32 = 3000;
pub const CONVERSION_MAX: u32 = 6000;
/// Villager conversion chance (hard diff = 100%, normal = 50%, easy = 0%).
pub fn villager_conversion_chance(difficulty: u8) -> f32 {
    match difficulty {
        0 => 0.0,
        1 => 0.0,
        2 => 0.5,
        _ => 1.0,
    }
}

impl Zombie {
    pub fn new_adult() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            age: 0,
            is_villager: false,
            village_id: None,
            converting_to_villager: false,
            conversion_ticks: 0,
            reinforcement_chance: rng.gen::<f32>(),
        }
    }

    pub fn new_baby() -> Self {
        Self {
            age: -24000,
            is_villager: false,
            village_id: None,
            converting_to_villager: false,
            conversion_ticks: 0,
            reinforcement_chance: 0.0,
        }
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.converting_to_villager && self.conversion_ticks > 0 {
            self.conversion_ticks -= 1;
        }
    }

    pub fn is_baby(&self) -> bool {
        self.age < 0
    }

    /// Start curing into villager (golden apple + weakness).
    pub fn start_curing(&mut self) {
        if !self.is_villager {
            return;
        }
        let mut rng = rand::thread_rng();
        self.converting_to_villager = true;
        self.conversion_ticks = rng.gen_range(CONVERSION_MIN..=CONVERSION_MAX);
    }

    pub fn is_cured(&self) -> bool {
        self.converting_to_villager && self.conversion_ticks == 0
    }

    /// Sunlight burns adult zombies (but not babies in some versions).
    pub fn burns_in_sunlight(&self) -> bool {
        !self.is_baby()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baby_grows() {
        let mut z = Zombie::new_baby();
        assert!(z.is_baby());
        for _ in 0..24000 {
            z.tick();
        }
        assert!(!z.is_baby());
    }

    #[test]
    fn hard_always_converts() {
        assert_eq!(villager_conversion_chance(3), 1.0);
    }

    #[test]
    fn easy_never_converts() {
        assert_eq!(villager_conversion_chance(1), 0.0);
    }
}
