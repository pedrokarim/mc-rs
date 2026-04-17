//! Fire mechanics — port PMMP `src/block/Fire.php`.

/// Age d'un bloc fire (0-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireAge(pub u8);

impl FireAge {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn increment(&mut self) {
        self.0 = (self.0 + 1).min(15);
    }

    pub fn is_burned_out(&self) -> bool {
        self.0 >= 15
    }

    /// Tick : chance de s'éteindre. Croît avec l'age.
    pub fn should_burn_out(&self) -> bool {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_range(0..16) <= self.0
    }
}

impl Default for FireAge {
    fn default() -> Self {
        Self::new()
    }
}

/// Matériaux flammables et leurs ratings (encourage / flamability).
/// PMMP `Fire::$materialFlameOdds` + `materialBurnOdds`.
pub fn flame_odds(block_name: &str) -> u8 {
    match block_name {
        "minecraft:oak_planks"
        | "minecraft:birch_planks"
        | "minecraft:spruce_planks"
        | "minecraft:jungle_planks"
        | "minecraft:acacia_planks"
        | "minecraft:dark_oak_planks"
        | "minecraft:log"
        | "minecraft:wood"
        | "minecraft:oak_log"
        | "minecraft:birch_log"
        | "minecraft:spruce_log"
        | "minecraft:jungle_log"
        | "minecraft:acacia_log"
        | "minecraft:dark_oak_log" => 5,
        "minecraft:leaves"
        | "minecraft:oak_leaves"
        | "minecraft:birch_leaves"
        | "minecraft:spruce_leaves"
        | "minecraft:jungle_leaves"
        | "minecraft:acacia_leaves"
        | "minecraft:dark_oak_leaves" => 30,
        "minecraft:tall_grass" | "minecraft:tall_plants" => 60,
        "minecraft:bed" => 60,
        "minecraft:wool" => 30,
        "minecraft:bookshelf" => 30,
        "minecraft:hay_block" => 60,
        "minecraft:tnt" => 15,
        _ => 0,
    }
}

pub fn burn_odds(block_name: &str) -> u8 {
    match block_name {
        "minecraft:oak_planks"
        | "minecraft:birch_planks"
        | "minecraft:spruce_planks"
        | "minecraft:jungle_planks"
        | "minecraft:acacia_planks"
        | "minecraft:dark_oak_planks"
        | "minecraft:log"
        | "minecraft:wood"
        | "minecraft:oak_log"
        | "minecraft:birch_log"
        | "minecraft:spruce_log"
        | "minecraft:jungle_log"
        | "minecraft:acacia_log"
        | "minecraft:dark_oak_log" => 20,
        "minecraft:leaves"
        | "minecraft:oak_leaves"
        | "minecraft:birch_leaves"
        | "minecraft:spruce_leaves"
        | "minecraft:jungle_leaves"
        | "minecraft:acacia_leaves"
        | "minecraft:dark_oak_leaves" => 60,
        "minecraft:tall_grass" | "minecraft:tall_plants" => 100,
        "minecraft:bed" => 20,
        "minecraft:wool" => 60,
        "minecraft:bookshelf" => 20,
        "minecraft:hay_block" => 20,
        "minecraft:tnt" => 100,
        _ => 0,
    }
}

pub fn can_catch_fire(block_name: &str) -> bool {
    flame_odds(block_name) > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planks_are_flammable() {
        assert!(can_catch_fire("minecraft:oak_planks"));
    }

    #[test]
    fn stone_is_not_flammable() {
        assert!(!can_catch_fire("minecraft:stone"));
    }

    #[test]
    fn tnt_high_burn_odds() {
        assert_eq!(burn_odds("minecraft:tnt"), 100);
    }

    #[test]
    fn fire_age_caps_at_15() {
        let mut age = FireAge::new();
        for _ in 0..20 {
            age.increment();
        }
        assert_eq!(age.0, 15);
    }
}
