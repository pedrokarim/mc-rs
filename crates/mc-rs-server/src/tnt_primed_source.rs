//! TNT primed sources — ce qui peut déclencher la TNT.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TntIgnition {
    /// Joueur flint & steel.
    FlintAndSteel,
    /// Projectile burning (arrow_flame).
    BurningProjectile,
    /// Redstone signal.
    Redstone,
    /// Fire block adjacent.
    FireSpread,
    /// Explosion d'une autre TNT.
    Explosion,
    /// Lava flowing.
    Lava,
    /// Creeper explosion.
    Creeper,
}

impl TntIgnition {
    /// Fuse ticks override pour cette source.
    pub fn fuse_ticks(&self) -> u32 {
        match self {
            Self::Explosion | Self::Lava => 10, // immediate
            _ => 80,                            // standard
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lava_immediate_fuse() {
        assert!(TntIgnition::Lava.fuse_ticks() < TntIgnition::FlintAndSteel.fuse_ticks());
    }
}
