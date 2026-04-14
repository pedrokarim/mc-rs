//! Drop reasons — qui a droit à quels drops.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropContext {
    /// Bloc cassé par un joueur.
    PlayerBreak { has_silk_touch: bool, fortune: u8 },
    /// Bloc détruit par explosion.
    Explosion,
    /// Bloc cassé par une autre entité (creeper, wither, etc.).
    EntityBreak,
    /// Bloc retiré par piston.
    PistonMove,
    /// Bloc melted/decayed (leaves, ice).
    NaturalDecay,
    /// Mob tué.
    MobKill { by_player: bool, looting: u8 },
    /// Trivial drop (e.g. harvest crop).
    Harvest,
}

impl DropContext {
    pub fn should_drop_items(&self) -> bool {
        match self {
            Self::PlayerBreak { .. } | Self::EntityBreak | Self::Harvest => true,
            Self::Explosion => true, // Bedrock : yes (some drops with reduced rate)
            Self::NaturalDecay => true,
            Self::PistonMove => false, // vanilla : piston pushes blocks rather than drop
            Self::MobKill { .. } => true,
        }
    }

    pub fn drop_rate_multiplier(&self) -> f32 {
        match self {
            Self::PlayerBreak { has_silk_touch: true, .. } => 1.0,
            Self::Explosion => 0.35, // vanilla : réduit
            Self::NaturalDecay => 0.2, // leaves only 20% drop sapling
            _ => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piston_doesnt_drop() {
        assert!(!DropContext::PistonMove.should_drop_items());
    }

    #[test]
    fn explosion_reduced_drop_rate() {
        assert!(DropContext::Explosion.drop_rate_multiplier() < 1.0);
    }
}
