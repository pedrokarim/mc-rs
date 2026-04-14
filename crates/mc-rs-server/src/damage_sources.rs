//! Damage sources / types (PMMP + vanilla).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageCause {
    Entity,       // attack from mob/player
    Projectile,   // arrow/trident
    Suffocation,  // block
    Fall,
    Fire,
    Lava,
    Drowning,
    BlockExplosion,
    EntityExplosion,
    Void,
    Lightning,
    Starvation,
    Magic,        // potion
    Thorns,
    Poison,
    Wither,
    Falling,      // falling block
    Contact,      // cactus/sweet berries
    Suicide,
    Custom,
}

impl DamageCause {
    /// Can be blocked by armor?
    pub fn is_armor_blockable(&self) -> bool {
        !matches!(self, Self::Drowning | Self::Starvation | Self::Void | Self::Magic | Self::Poison | Self::Wither)
    }

    /// Can be reduced by Protection enchant?
    pub fn is_protectable(&self) -> bool {
        !matches!(self, Self::Starvation | Self::Void | Self::Suicide)
    }

    /// Does this cause apply knockback?
    pub fn causes_knockback(&self) -> bool {
        matches!(self,
            Self::Entity | Self::Projectile | Self::BlockExplosion | Self::EntityExplosion | Self::Thorns
        )
    }
}

/// Death message key.
pub fn death_message_key(cause: DamageCause) -> &'static str {
    match cause {
        DamageCause::Entity => "death.attack.player",
        DamageCause::Projectile => "death.attack.arrow",
        DamageCause::Suffocation => "death.attack.inWall",
        DamageCause::Fall => "death.fell.accident.generic",
        DamageCause::Fire => "death.attack.inFire",
        DamageCause::Lava => "death.attack.lava",
        DamageCause::Drowning => "death.attack.drown",
        DamageCause::BlockExplosion => "death.attack.explosion",
        DamageCause::EntityExplosion => "death.attack.explosion.player",
        DamageCause::Void => "death.attack.outOfWorld",
        DamageCause::Lightning => "death.attack.lightningBolt",
        DamageCause::Starvation => "death.attack.starve",
        DamageCause::Magic => "death.attack.magic",
        DamageCause::Thorns => "death.attack.thorns",
        DamageCause::Poison => "death.attack.magic",
        DamageCause::Wither => "death.attack.wither",
        DamageCause::Falling => "death.attack.fallingBlock",
        DamageCause::Contact => "death.attack.cactus",
        DamageCause::Suicide => "death.attack.generic",
        DamageCause::Custom => "death.attack.generic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn void_not_blockable() {
        assert!(!DamageCause::Void.is_armor_blockable());
    }

    #[test]
    fn entity_blockable() {
        assert!(DamageCause::Entity.is_armor_blockable());
    }
}
