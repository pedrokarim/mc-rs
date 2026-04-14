//! Frog — white/green/temperate. Mange slimes/magma cubes. Ponde frogspawn.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrogVariant {
    Temperate,  // Beige — default + most biomes
    Warm,       // White — warm biomes (jungle, desert, savanna, badlands, swamp, mangrove)
    Cold,       // Green — snowy/cold biomes
}

impl FrogVariant {
    pub fn from_biome_temperature(temperature: f32) -> Self {
        if temperature < 0.0 {
            Self::Cold
        } else if temperature > 1.0 {
            Self::Warm
        } else {
            Self::Temperate
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frog {
    pub variant: FrogVariant,
    pub age: i32,
    pub jumping: bool,
    pub tongue_cooldown: u32,
    pub ready_to_lay: bool,
}

/// Breeding item = slimeball.
pub const BREEDING_ITEM: u16 = 341;
/// Tongue cooldown après une attaque (30 ticks).
pub const TONGUE_COOLDOWN: u32 = 30;
/// Tongue range (3 blocs).
pub const TONGUE_RANGE: f64 = 3.0;

impl Frog {
    pub fn new(variant: FrogVariant) -> Self {
        Self {
            variant,
            age: 0,
            jumping: false,
            tongue_cooldown: 0,
            ready_to_lay: false,
        }
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.tongue_cooldown > 0 {
            self.tongue_cooldown -= 1;
        }
    }

    pub fn can_use_tongue(&self) -> bool {
        self.tongue_cooldown == 0
    }

    pub fn use_tongue(&mut self) {
        self.tongue_cooldown = TONGUE_COOLDOWN;
    }

    /// When frog eats small magma cube, drop pearlescent froglight etc.
    pub fn froglight_from_prey(prey_tag: &str) -> Option<&'static str> {
        match prey_tag {
            "minecraft:small_magma_cube" => match rand::random::<u8>() % 3 {
                0 => Some("minecraft:ochre_froglight"),
                1 => Some("minecraft:pearlescent_froglight"),
                _ => Some("minecraft:verdant_froglight"),
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_variant_for_snow() {
        assert_eq!(FrogVariant::from_biome_temperature(-0.3), FrogVariant::Cold);
    }

    #[test]
    fn warm_variant_for_desert() {
        assert_eq!(FrogVariant::from_biome_temperature(1.5), FrogVariant::Warm);
    }

    #[test]
    fn tongue_cooldown_ticks_down() {
        let mut f = Frog::new(FrogVariant::Temperate);
        f.use_tongue();
        assert!(!f.can_use_tongue());
        for _ in 0..TONGUE_COOLDOWN {
            f.tick();
        }
        assert!(f.can_use_tongue());
    }
}
