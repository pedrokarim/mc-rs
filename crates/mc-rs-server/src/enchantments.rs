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
    /// Parse "minecraft:sharpness" / "sharpness" / id numérique.
    pub fn from_name_or_id(token: &str) -> Option<Self> {
        if let Ok(id) = token.parse::<u8>() {
            return Self::from_id(id);
        }
        let short = token
            .strip_prefix("minecraft:")
            .unwrap_or(token)
            .to_ascii_lowercase();
        match short.as_str() {
            "protection" => Some(Self::Protection),
            "fire_protection" => Some(Self::FireProtection),
            "feather_falling" => Some(Self::FeatherFalling),
            "blast_protection" => Some(Self::BlastProtection),
            "projectile_protection" => Some(Self::ProjectileProtection),
            "thorns" => Some(Self::Thorns),
            "respiration" => Some(Self::Respiration),
            "depth_strider" => Some(Self::DepthStrider),
            "aqua_affinity" => Some(Self::AquaAffinity),
            "sharpness" => Some(Self::Sharpness),
            "smite" => Some(Self::Smite),
            "bane_of_arthropods" => Some(Self::BaneOfArthropods),
            "knockback" => Some(Self::Knockback),
            "fire_aspect" => Some(Self::FireAspect),
            "looting" => Some(Self::Looting),
            "efficiency" => Some(Self::Efficiency),
            "silk_touch" => Some(Self::SilkTouch),
            "unbreaking" => Some(Self::Unbreaking),
            "fortune" => Some(Self::Fortune),
            "power" | "bow_power" => Some(Self::Power),
            "punch" | "bow_punch" => Some(Self::Punch),
            "flame" | "bow_flame" => Some(Self::Flame),
            "infinity" | "bow_infinity" => Some(Self::Infinity),
            "luck_of_the_sea" => Some(Self::LuckOfTheSea),
            "lure" => Some(Self::Lure),
            "frost_walker" => Some(Self::FrostWalker),
            "mending" => Some(Self::Mending),
            "binding" | "binding_curse" | "curse_of_binding" => Some(Self::BindingCurse),
            "vanishing" | "vanishing_curse" | "curse_of_vanishing" => Some(Self::VanishingCurse),
            "impaling" => Some(Self::Impaling),
            "riptide" => Some(Self::Riptide),
            "loyalty" => Some(Self::Loyalty),
            "channeling" => Some(Self::Channeling),
            "multishot" => Some(Self::Multishot),
            "piercing" => Some(Self::Piercing),
            "quick_charge" => Some(Self::QuickCharge),
            "soul_speed" => Some(Self::SoulSpeed),
            "swift_sneak" => Some(Self::SwiftSneak),
            _ => None,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Protection),
            1 => Some(Self::FireProtection),
            2 => Some(Self::FeatherFalling),
            3 => Some(Self::BlastProtection),
            4 => Some(Self::ProjectileProtection),
            5 => Some(Self::Thorns),
            6 => Some(Self::Respiration),
            7 => Some(Self::DepthStrider),
            8 => Some(Self::AquaAffinity),
            9 => Some(Self::Sharpness),
            10 => Some(Self::Smite),
            11 => Some(Self::BaneOfArthropods),
            12 => Some(Self::Knockback),
            13 => Some(Self::FireAspect),
            14 => Some(Self::Looting),
            15 => Some(Self::Efficiency),
            16 => Some(Self::SilkTouch),
            17 => Some(Self::Unbreaking),
            18 => Some(Self::Fortune),
            19 => Some(Self::Power),
            20 => Some(Self::Punch),
            21 => Some(Self::Flame),
            22 => Some(Self::Infinity),
            23 => Some(Self::LuckOfTheSea),
            24 => Some(Self::Lure),
            25 => Some(Self::FrostWalker),
            26 => Some(Self::Mending),
            27 => Some(Self::BindingCurse),
            28 => Some(Self::VanishingCurse),
            29 => Some(Self::Impaling),
            30 => Some(Self::Riptide),
            31 => Some(Self::Loyalty),
            32 => Some(Self::Channeling),
            33 => Some(Self::Multishot),
            34 => Some(Self::Piercing),
            35 => Some(Self::QuickCharge),
            36 => Some(Self::SoulSpeed),
            37 => Some(Self::SwiftSneak),
            _ => None,
        }
    }

    pub fn id(&self) -> u8 {
        *self as u8
    }

    /// Liste des noms vanilla d'enchantements — utilisée pour populer
    /// la SoftEnum d'autocomplétion `/enchant`.
    pub fn all_names() -> &'static [&'static str] {
        &[
            "protection",
            "fire_protection",
            "feather_falling",
            "blast_protection",
            "projectile_protection",
            "thorns",
            "respiration",
            "depth_strider",
            "aqua_affinity",
            "sharpness",
            "smite",
            "bane_of_arthropods",
            "knockback",
            "fire_aspect",
            "looting",
            "efficiency",
            "silk_touch",
            "unbreaking",
            "fortune",
            "power",
            "punch",
            "flame",
            "infinity",
            "luck_of_the_sea",
            "lure",
            "frost_walker",
            "mending",
            "binding_curse",
            "vanishing_curse",
            "impaling",
            "riptide",
            "loyalty",
            "channeling",
            "multishot",
            "piercing",
            "quick_charge",
            "soul_speed",
            "swift_sneak",
        ]
    }

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

/// Construit un `extra_data` Bedrock contenant une liste d'enchants.
/// Format PMMP `ItemStackExtraData::write` :
///   i16 LE 0xFFFF (NBT marker) + u8 1 (version) + raw NBT LE bytes
///   + u32 LE canPlaceOn=0 + u32 LE canDestroy=0
pub fn build_extra_data_with_enchant(enchant_id: u8, level: u8) -> Vec<u8> {
    use bytes::BytesMut;
    use mc_rs_nbt::{tag::NbtCompound, NbtRoot, NbtTag};

    // Compound { "ench": List< Compound { id, lvl } > }
    let mut entry = NbtCompound::new();
    entry.insert("id".to_string(), NbtTag::Short(enchant_id as i16));
    entry.insert("lvl".to_string(), NbtTag::Short(level as i16));
    let mut root_compound = NbtCompound::new();
    root_compound.insert(
        "ench".to_string(),
        NbtTag::List(vec![NbtTag::Compound(entry)]),
    );
    let root = NbtRoot::new("", root_compound);

    let mut nbt_buf = BytesMut::new();
    mc_rs_nbt::write_nbt_le(&mut nbt_buf, &root);

    let mut out = Vec::with_capacity(11 + nbt_buf.len());
    out.extend_from_slice(&(-1i16).to_le_bytes()); // marker 0xFFFF
    out.push(1u8); // version
    out.extend_from_slice(&nbt_buf);
    out.extend_from_slice(&0u32.to_le_bytes()); // canPlaceOn
    out.extend_from_slice(&0u32.to_le_bytes()); // canDestroy
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_extra_data_starts_with_nbt_marker() {
        let bytes = build_extra_data_with_enchant(9, 5); // sharpness V
                                                         // Marker FF FF + version 01
        assert_eq!(&bytes[0..3], &[0xFF, 0xFF, 0x01]);
    }

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
