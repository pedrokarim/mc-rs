//! Rabbit — variants + killer rabbit rare.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RabbitVariant {
    Brown,
    White,
    Black,
    WhiteAndBlack,
    Gold,
    SaltAndPepper,
    KillerRabbit,
}

impl RabbitVariant {
    pub fn from_biome(biome_id: u8) -> Self {
        match biome_id {
            12 | 30 | 31 | 140 | 158 => {
                if rand::random::<bool>() {
                    Self::White
                } else {
                    Self::WhiteAndBlack
                }
            }
            2 | 17 | 130 => {
                // Desert
                Self::Gold
            }
            _ => {
                match rand::random::<u8>() % 5 {
                    0 => Self::Brown,
                    1 => Self::White,
                    2 => Self::Black,
                    3 => Self::WhiteAndBlack,
                    _ => Self::SaltAndPepper,
                }
            }
        }
    }

    pub fn is_hostile(&self) -> bool {
        matches!(self, Self::KillerRabbit)
    }
}

#[derive(Debug, Clone)]
pub struct Rabbit {
    pub variant: RabbitVariant,
    pub age: i32,
    pub last_jump_tick: u64,
}

/// Breeding item = carrot / golden carrot / dandelion.
pub fn breeding_items() -> &'static [&'static str] {
    &[
        "minecraft:carrot",
        "minecraft:golden_carrot",
        "minecraft:dandelion",
    ]
}

impl Rabbit {
    pub fn new(variant: RabbitVariant) -> Self {
        Self {
            variant,
            age: 0,
            last_jump_tick: 0,
        }
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
    }

    pub fn damage(&self) -> f32 {
        if self.variant == RabbitVariant::KillerRabbit {
            8.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn killer_is_hostile() {
        assert!(RabbitVariant::KillerRabbit.is_hostile());
    }

    #[test]
    fn regular_not_hostile() {
        assert!(!RabbitVariant::Brown.is_hostile());
    }
}
