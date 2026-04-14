//! Enchantements — port sélectif de `.reference/PocketMine-MP/src/item/enchantment/*`
//! et `.reference/PocketMine-MP/src/data/bedrock/EnchantmentIdMap.php`.
//!
//! Liste des enchantements vanilla avec leurs IDs réseau Bedrock, leurs
//! niveaux max et leurs applicabilités. Pas de logique d'application (=
//! bonus au damage, réduction, etc.) dans ce module — juste la data model.

/// Port PMMP `VanillaEnchantments.php`. Les IDs matchent EnchantmentIdMap.php.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EnchantmentKind {
    Protection = 0,
    FireProtection = 1,
    FeatherFalling = 2,
    BlastProtection = 3,
    ProjectileProtection = 4,
    Thorns = 5,
    Respiration = 6,
    DepthStrider = 7,
    AquaAffinity = 8,
    Sharpness = 9,
    Smite = 10,
    BaneOfArthropods = 11,
    Knockback = 12,
    FireAspect = 13,
    Looting = 14,
    Efficiency = 15,
    SilkTouch = 16,
    Unbreaking = 17,
    Fortune = 18,
    Power = 19,
    Punch = 20,
    Flame = 21,
    Infinity = 22,
    LuckOfTheSea = 23,
    Lure = 24,
    FrostWalker = 25,
    Mending = 26,
    BindingCurse = 27,
    VanishingCurse = 28,
    Impaling = 29,
    Riptide = 30,
    Loyalty = 31,
    Channeling = 32,
    Multishot = 33,
    Piercing = 34,
    QuickCharge = 35,
    SoulSpeed = 36,
    SwiftSneak = 37,
}

impl EnchantmentKind {
    /// Niveau max (1..N) pour cet enchantement.
    pub fn max_level(&self) -> u8 {
        match self {
            Self::Protection
            | Self::FireProtection
            | Self::FeatherFalling
            | Self::BlastProtection
            | Self::ProjectileProtection
            | Self::Sharpness
            | Self::Smite
            | Self::BaneOfArthropods
            | Self::Efficiency
            | Self::Power
            | Self::Impaling
            | Self::Piercing
            | Self::SoulSpeed
            | Self::SwiftSneak => 5,
            Self::Thorns
            | Self::Respiration
            | Self::DepthStrider
            | Self::Looting
            | Self::Fortune
            | Self::LuckOfTheSea
            | Self::Lure
            | Self::FrostWalker
            | Self::Riptide
            | Self::Loyalty
            | Self::QuickCharge => 3,
            Self::Knockback | Self::FireAspect | Self::Unbreaking | Self::Punch => 4,
            Self::AquaAffinity
            | Self::SilkTouch
            | Self::Infinity
            | Self::Mending
            | Self::BindingCurse
            | Self::VanishingCurse
            | Self::Multishot
            | Self::Flame
            | Self::Channeling => 1,
        }
    }

    /// PMMP `Enchantment::getRarity()` — pour le calcul des chances d'enchant.
    /// 1=common, 2=uncommon, 3=rare, 4=very_rare.
    pub fn rarity(&self) -> u8 {
        match self {
            Self::Protection
            | Self::Efficiency
            | Self::Sharpness
            | Self::Power
            | Self::Piercing
            | Self::FeatherFalling => 1,
            Self::FireProtection
            | Self::ProjectileProtection
            | Self::Smite
            | Self::BaneOfArthropods
            | Self::Knockback
            | Self::Looting
            | Self::FireAspect
            | Self::Unbreaking
            | Self::Lure
            | Self::QuickCharge
            | Self::Multishot
            | Self::Respiration
            | Self::DepthStrider => 2,
            Self::BlastProtection
            | Self::Thorns
            | Self::AquaAffinity
            | Self::Fortune
            | Self::LuckOfTheSea
            | Self::Punch
            | Self::Flame
            | Self::Impaling
            | Self::Loyalty
            | Self::Riptide
            | Self::FrostWalker
            | Self::SoulSpeed => 3,
            Self::SilkTouch
            | Self::Infinity
            | Self::Mending
            | Self::BindingCurse
            | Self::VanishingCurse
            | Self::Channeling
            | Self::SwiftSneak => 4,
        }
    }

    /// Liste des enchantements incompatibles (exclusifs mutuellement).
    /// PMMP `EnchantmentProtectionTypes` + manual list.
    pub fn incompatible_with(&self, other: Self) -> bool {
        use EnchantmentKind::*;
        matches!(
            (*self, other),
            // Les types de protection ne se cumulent pas entre eux.
            (Protection, FireProtection | BlastProtection | ProjectileProtection)
            | (FireProtection, Protection | BlastProtection | ProjectileProtection)
            | (BlastProtection, Protection | FireProtection | ProjectileProtection)
            | (ProjectileProtection, Protection | FireProtection | BlastProtection)
            // Sharpness/Smite/BaneOfArthropods exclusifs.
            | (Sharpness, Smite | BaneOfArthropods)
            | (Smite, Sharpness | BaneOfArthropods)
            | (BaneOfArthropods, Sharpness | Smite)
            // SilkTouch/Fortune exclusifs.
            | (SilkTouch, Fortune)
            | (Fortune, SilkTouch)
            // Infinity/Mending exclusifs.
            | (Infinity, Mending)
            | (Mending, Infinity)
            // Piercing/Multishot exclusifs.
            | (Piercing, Multishot)
            | (Multishot, Piercing)
            // Loyalty/Riptide/Channeling exclusifs avec Riptide.
            | (Riptide, Loyalty | Channeling)
            | (Loyalty, Riptide)
            | (Channeling, Riptide)
        )
    }
}

/// Instance d'un enchantement appliqué sur un item.
#[derive(Debug, Clone, Copy)]
pub struct EnchantmentInstance {
    pub kind: EnchantmentKind,
    pub level: u8,
}

impl EnchantmentInstance {
    pub fn new(kind: EnchantmentKind, level: u8) -> Self {
        Self {
            kind,
            level: level.clamp(1, kind.max_level()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharpness_smite_incompatible() {
        assert!(EnchantmentKind::Sharpness.incompatible_with(EnchantmentKind::Smite));
    }

    #[test]
    fn silk_touch_fortune_incompatible() {
        assert!(EnchantmentKind::SilkTouch.incompatible_with(EnchantmentKind::Fortune));
    }

    #[test]
    fn sharpness_max_5() {
        assert_eq!(EnchantmentKind::Sharpness.max_level(), 5);
    }

    #[test]
    fn instance_clamps_level() {
        let e = EnchantmentInstance::new(EnchantmentKind::SilkTouch, 10);
        assert_eq!(e.level, 1); // silk touch max_level = 1
    }
}
