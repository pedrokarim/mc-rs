//! Mooshroom — cow champignon, shear + bowl + stew.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MooshroomVariant {
    Red,
    Brown, // From lightning strike
}

#[derive(Debug, Clone)]
pub struct Mooshroom {
    pub variant: MooshroomVariant,
    pub age: i32,
    pub stew_effects: Vec<&'static str>, // Suspicious stew effects from flowers
    pub shorn: bool,
}

/// Stew giving when milked with bowl.
pub const BOWL_STEW_PER_USE: u32 = 1;
/// Flower for suspicious stew (brown mooshroom).
pub fn small_flowers_for_suspicious_stew() -> &'static [(&'static str, &'static str)] {
    &[
        ("minecraft:dandelion", "saturation"),
        ("minecraft:poppy", "night_vision"),
        ("minecraft:blue_orchid", "saturation"),
        ("minecraft:allium", "fire_resistance"),
        ("minecraft:azure_bluet", "blindness"),
        ("minecraft:red_tulip", "weakness"),
        ("minecraft:orange_tulip", "weakness"),
        ("minecraft:white_tulip", "weakness"),
        ("minecraft:pink_tulip", "weakness"),
        ("minecraft:oxeye_daisy", "regeneration"),
        ("minecraft:cornflower", "jump_boost"),
        ("minecraft:lily_of_the_valley", "poison"),
    ]
}

impl Mooshroom {
    pub fn new(variant: MooshroomVariant) -> Self {
        Self {
            variant,
            age: 0,
            stew_effects: Vec::new(),
            shorn: false,
        }
    }

    /// Brown mooshrooms eat flowers and yield suspicious stew with effect.
    pub fn consume_flower(&mut self, flower_name: &str) {
        if self.variant != MooshroomVariant::Brown {
            return;
        }
        if let Some((_, effect)) = small_flowers_for_suspicious_stew()
            .iter()
            .find(|(f, _)| *f == flower_name)
        {
            self.stew_effects.push(effect);
        }
    }

    /// Struck by lightning turns red mooshroom into brown.
    pub fn struck_by_lightning(&mut self) {
        if self.variant == MooshroomVariant::Red {
            self.variant = MooshroomVariant::Brown;
        } else {
            self.variant = MooshroomVariant::Red;
        }
    }

    /// Shear drops 5 mushrooms and makes it a regular cow.
    pub fn shear(&mut self) -> Option<&'static str> {
        if self.shorn {
            return None;
        }
        self.shorn = true;
        Some(match self.variant {
            MooshroomVariant::Red => "minecraft:red_mushroom",
            MooshroomVariant::Brown => "minecraft:brown_mushroom",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lightning_swaps_variant() {
        let mut m = Mooshroom::new(MooshroomVariant::Red);
        m.struck_by_lightning();
        assert_eq!(m.variant, MooshroomVariant::Brown);
    }

    #[test]
    fn shear_once_only() {
        let mut m = Mooshroom::new(MooshroomVariant::Red);
        assert!(m.shear().is_some());
        assert!(m.shear().is_none());
    }
}
