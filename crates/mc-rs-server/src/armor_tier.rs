//! Armor material tier + protection values.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorTier {
    Leather,
    Chainmail,
    Gold,
    Iron,
    Diamond,
    Netherite,
    Turtle,
}

impl ArmorTier {
    /// Defense points per piece.
    pub fn defense(&self, slot: ArmorSlot) -> u8 {
        use ArmorSlot::*;
        match (self, slot) {
            (Self::Leather, Helmet) => 1,
            (Self::Leather, Chestplate) => 3,
            (Self::Leather, Leggings) => 2,
            (Self::Leather, Boots) => 1,
            (Self::Chainmail, Helmet) => 2,
            (Self::Chainmail, Chestplate) => 5,
            (Self::Chainmail, Leggings) => 4,
            (Self::Chainmail, Boots) => 1,
            (Self::Iron, Helmet) => 2,
            (Self::Iron, Chestplate) => 6,
            (Self::Iron, Leggings) => 5,
            (Self::Iron, Boots) => 2,
            (Self::Gold, Helmet) => 2,
            (Self::Gold, Chestplate) => 5,
            (Self::Gold, Leggings) => 3,
            (Self::Gold, Boots) => 1,
            (Self::Diamond, Helmet) => 3,
            (Self::Diamond, Chestplate) => 8,
            (Self::Diamond, Leggings) => 6,
            (Self::Diamond, Boots) => 3,
            (Self::Netherite, Helmet) => 3,
            (Self::Netherite, Chestplate) => 8,
            (Self::Netherite, Leggings) => 6,
            (Self::Netherite, Boots) => 3,
            (Self::Turtle, Helmet) => 2,
            _ => 0,
        }
    }

    /// Toughness (netherite/diamond only).
    pub fn toughness(&self) -> f32 {
        match self {
            Self::Diamond => 2.0,
            Self::Netherite => 3.0,
            _ => 0.0,
        }
    }

    /// Knockback resistance (netherite).
    pub fn knockback_resistance(&self) -> f32 {
        match self {
            Self::Netherite => 0.1, // per piece
            _ => 0.0,
        }
    }

    /// Durability per piece.
    pub fn durability(&self, slot: ArmorSlot) -> u16 {
        let base = match self {
            Self::Leather => 55,
            Self::Chainmail | Self::Iron => 165,
            Self::Gold => 77,
            Self::Diamond => 363,
            Self::Netherite => 407,
            Self::Turtle => 275,
        };
        match slot {
            ArmorSlot::Helmet => base * 11 / 15,
            ArmorSlot::Chestplate => base * 16 / 15,
            ArmorSlot::Leggings => base,
            ArmorSlot::Boots => base * 13 / 15,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorSlot {
    Helmet,
    Chestplate,
    Leggings,
    Boots,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diamond_more_defense_than_iron() {
        assert!(
            ArmorTier::Diamond.defense(ArmorSlot::Chestplate)
                > ArmorTier::Iron.defense(ArmorSlot::Chestplate)
        );
    }
}
