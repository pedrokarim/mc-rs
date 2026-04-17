//! Potion container recipes — port PMMP `PotionContainerChangeRecipe.php`.
//! Glass bottle + liquid = filled potion ; potion + ingredient = new potion.

use crate::brewing::PotionType;

/// Input container → Output after brewing with specific ingredient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotionContainer {
    GlassBottle,
    Water,
    Awkward,
    Mundane,
    Thick,
    Generic, // contains a brewed PotionType
    Splash,
    Lingering,
}

/// Convert a normal potion to a splash potion using gunpowder.
pub fn to_splash(p: PotionType) -> PotionType {
    // Dans Bedrock il y a une variant Splash séparée par type. Ici on garde
    // la même PotionType mais le container (splash vs normal) est séparé.
    p
}

/// Convert splash to lingering via dragon's breath.
pub fn to_lingering(p: PotionType) -> PotionType {
    p
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotionForm {
    Drinkable,
    Splash,
    Lingering,
    Arrow,
}

impl PotionForm {
    /// Duration multiplier vs drinkable.
    pub fn duration_multiplier(&self) -> f32 {
        match self {
            Self::Drinkable => 1.0,
            Self::Splash => 0.75,
            Self::Lingering => 0.25,
            Self::Arrow => 0.125,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splash_shorter_than_drinkable() {
        assert!(PotionForm::Splash.duration_multiplier() < 1.0);
    }

    #[test]
    fn lingering_shortest() {
        assert!(
            PotionForm::Lingering.duration_multiplier() < PotionForm::Splash.duration_multiplier()
        );
    }
}
