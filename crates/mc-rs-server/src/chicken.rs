//! Chicken — mob qui pond des oeufs + Jockey combo.

use rand::Rng;

#[derive(Debug, Clone)]
pub struct Chicken {
    pub age: i32,
    pub egg_layer_cooldown: u32,
    pub is_chicken_jockey: bool,
}

/// Egg laying cooldown (6000-12000 ticks vanilla).
pub const EGG_COOLDOWN_MIN: u32 = 6000;
pub const EGG_COOLDOWN_MAX: u32 = 12000;
/// Breeding item = seeds.
pub fn breeding_items() -> &'static [&'static str] {
    &[
        "minecraft:wheat_seeds",
        "minecraft:melon_seeds",
        "minecraft:pumpkin_seeds",
        "minecraft:beetroot_seeds",
        "minecraft:pitcher_pod",
        "minecraft:torchflower_seeds",
    ]
}

impl Chicken {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            age: 0,
            egg_layer_cooldown: rng.gen_range(EGG_COOLDOWN_MIN..=EGG_COOLDOWN_MAX),
            is_chicken_jockey: false,
        }
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.egg_layer_cooldown > 0 {
            self.egg_layer_cooldown -= 1;
        }
    }

    pub fn try_lay_egg(&mut self) -> bool {
        if self.egg_layer_cooldown > 0 || self.age < 0 {
            return false;
        }
        let mut rng = rand::thread_rng();
        self.egg_layer_cooldown = rng.gen_range(EGG_COOLDOWN_MIN..=EGG_COOLDOWN_MAX);
        true
    }

    /// Baby chicken from egg thrown (1/8 chance).
    pub fn egg_hatch_chance() -> f32 {
        1.0 / 8.0
    }
    /// Egg hatch 4 babies chance (1/32).
    pub fn egg_hatch_quad_chance() -> f32 {
        1.0 / 32.0
    }
}

impl Default for Chicken {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_ticks_down() {
        let mut c = Chicken::new();
        let init = c.egg_layer_cooldown;
        c.tick();
        assert_eq!(c.egg_layer_cooldown, init - 1);
    }
}
