//! Combat calculations: armor reduction, enchantments, critical hits, damage pipeline.

use mc_rs_nbt::tag::{NbtCompound, NbtRoot, NbtTag};
use mc_rs_proto::item_stack::ItemStack;
use mc_rs_world::item_registry::ItemRegistry;

// ---------------------------------------------------------------------------
// Enchantment types
// ---------------------------------------------------------------------------

/// A single enchantment on an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Enchantment {
    /// Bedrock enchantment ID.
    pub id: i16,
    /// Enchantment level (1-5).
    pub level: i16,
}

/// Well-known Bedrock enchantment IDs.
pub mod enchantment_id {
    pub const PROTECTION: i16 = 0;
    pub const FIRE_PROTECTION: i16 = 1;
    pub const FEATHER_FALLING: i16 = 2;
    pub const BLAST_PROTECTION: i16 = 3;
    pub const PROJECTILE_PROTECTION: i16 = 4;
    pub const THORNS: i16 = 5;
    pub const RESPIRATION: i16 = 6;
    pub const DEPTH_STRIDER: i16 = 7;
    pub const AQUA_AFFINITY: i16 = 8;
    pub const SHARPNESS: i16 = 9;
    pub const SMITE: i16 = 10;
    pub const BANE_OF_ARTHROPODS: i16 = 11;
    pub const KNOCKBACK: i16 = 12;
    pub const FIRE_ASPECT: i16 = 13;
    pub const LOOTING: i16 = 14;
    pub const EFFICIENCY: i16 = 15;
    pub const SILK_TOUCH: i16 = 16;
    pub const UNBREAKING: i16 = 17;
    pub const FORTUNE: i16 = 18;
    pub const POWER: i16 = 19;
    pub const PUNCH: i16 = 20;
    pub const FLAME: i16 = 21;
    pub const INFINITY: i16 = 22;
    pub const LUCK_OF_THE_SEA: i16 = 23;
    pub const LURE: i16 = 24;
    pub const FROST_WALKER: i16 = 25;
    pub const MENDING: i16 = 26;
    pub const BINDING_CURSE: i16 = 27;
    pub const VANISHING_CURSE: i16 = 28;
    pub const IMPALING: i16 = 29;
    pub const RIPTIDE: i16 = 30;
    pub const LOYALTY: i16 = 31;
    pub const CHANNELING: i16 = 32;
    pub const MULTISHOT: i16 = 33;
    pub const PIERCING: i16 = 34;
    pub const QUICK_CHARGE: i16 = 35;
    pub const SOUL_SPEED: i16 = 36;
}

// ---------------------------------------------------------------------------
// Enchantment info registry
// ---------------------------------------------------------------------------

/// Static information about an enchantment type.
pub struct EnchantmentInfo {
    pub id: i16,
    pub name: &'static str,
    pub max_level: i16,
}

/// All known Bedrock enchantments with their max levels.
pub const ENCHANTMENT_LIST: &[EnchantmentInfo] = &[
    EnchantmentInfo {
        id: 0,
        name: "protection",
        max_level: 4,
    },
    EnchantmentInfo {
        id: 1,
        name: "fire_protection",
        max_level: 4,
    },
    EnchantmentInfo {
        id: 2,
        name: "feather_falling",
        max_level: 4,
    },
    EnchantmentInfo {
        id: 3,
        name: "blast_protection",
        max_level: 4,
    },
    EnchantmentInfo {
        id: 4,
        name: "projectile_protection",
        max_level: 4,
    },
    EnchantmentInfo {
        id: 5,
        name: "thorns",
        max_level: 3,
    },
    EnchantmentInfo {
        id: 6,
        name: "respiration",
        max_level: 3,
    },
    EnchantmentInfo {
        id: 7,
        name: "depth_strider",
        max_level: 3,
    },
    EnchantmentInfo {
        id: 8,
        name: "aqua_affinity",
        max_level: 1,
    },
    EnchantmentInfo {
        id: 9,
        name: "sharpness",
        max_level: 5,
    },
    EnchantmentInfo {
        id: 10,
        name: "smite",
        max_level: 5,
    },
    EnchantmentInfo {
        id: 11,
        name: "bane_of_arthropods",
        max_level: 5,
    },
    EnchantmentInfo {
        id: 12,
        name: "knockback",
        max_level: 2,
    },
    EnchantmentInfo {
        id: 13,
        name: "fire_aspect",
        max_level: 2,
    },
    EnchantmentInfo {
        id: 14,
        name: "looting",
        max_level: 3,
    },
    EnchantmentInfo {
        id: 15,
        name: "efficiency",
        max_level: 5,
    },
    EnchantmentInfo {
        id: 16,
        name: "silk_touch",
        max_level: 1,
    },
    EnchantmentInfo {
        id: 17,
        name: "unbreaking",
        max_level: 3,
    },
    EnchantmentInfo {
        id: 18,
        name: "fortune",
        max_level: 3,
    },
    EnchantmentInfo {
        id: 19,
        name: "power",
        max_level: 5,
    },
    EnchantmentInfo {
        id: 20,
        name: "punch",
        max_level: 2,
    },
    EnchantmentInfo {
        id: 21,
        name: "flame",
        max_level: 1,
    },
    EnchantmentInfo {
        id: 22,
        name: "infinity",
        max_level: 1,
    },
    EnchantmentInfo {
        id: 23,
        name: "luck_of_the_sea",
        max_level: 3,
    },
    EnchantmentInfo {
        id: 24,
        name: "lure",
        max_level: 3,
    },
    EnchantmentInfo {
        id: 25,
        name: "frost_walker",
        max_level: 2,
    },
    EnchantmentInfo {
        id: 26,
        name: "mending",
        max_level: 1,
    },
    EnchantmentInfo {
        id: 27,
        name: "binding_curse",
        max_level: 1,
    },
    EnchantmentInfo {
        id: 28,
        name: "vanishing_curse",
        max_level: 1,
    },
    EnchantmentInfo {
        id: 29,
        name: "impaling",
        max_level: 5,
    },
    EnchantmentInfo {
        id: 30,
        name: "riptide",
        max_level: 3,
    },
    EnchantmentInfo {
        id: 31,
        name: "loyalty",
        max_level: 3,
    },
    EnchantmentInfo {
        id: 32,
        name: "channeling",
        max_level: 1,
    },
    EnchantmentInfo {
        id: 33,
        name: "multishot",
        max_level: 1,
    },
    EnchantmentInfo {
        id: 34,
        name: "piercing",
        max_level: 4,
    },
    EnchantmentInfo {
        id: 35,
        name: "quick_charge",
        max_level: 3,
    },
    EnchantmentInfo {
        id: 36,
        name: "soul_speed",
        max_level: 3,
    },
];

/// Look up an enchantment by name (for `/enchant` command).
pub fn enchantment_by_name(name: &str) -> Option<&'static EnchantmentInfo> {
    ENCHANTMENT_LIST.iter().find(|e| e.name == name)
}

/// Look up an enchantment by ID.
pub fn enchantment_by_id(id: i16) -> Option<&'static EnchantmentInfo> {
    ENCHANTMENT_LIST.iter().find(|e| e.id == id)
}

// ---------------------------------------------------------------------------
// Enchantment parsing
// ---------------------------------------------------------------------------

/// Parse enchantments from raw NBT data (network format).
///
/// Bedrock stores enchantments as: `{ench: [{id: Short, lvl: Short}, ...]}`
pub fn parse_enchantments(nbt_data: &[u8]) -> Vec<Enchantment> {
    if nbt_data.is_empty() {
        return Vec::new();
    }

    let root = match mc_rs_nbt::read_nbt_network(&mut &nbt_data[..]) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let ench_list = match root.compound.get("ench") {
        Some(NbtTag::List(list)) => list,
        _ => return Vec::new(),
    };

    let mut result = Vec::new();
    for tag in ench_list {
        if let NbtTag::Compound(compound) = tag {
            let id = compound.get("id").and_then(|t| t.as_short()).unwrap_or(0);
            let lvl = compound.get("lvl").and_then(|t| t.as_short()).unwrap_or(0);
            result.push(Enchantment { id, level: lvl });
        }
    }
    result
}

/// Build NBT data for enchantments (network format).
pub fn build_enchantment_nbt(enchantments: &[Enchantment]) -> Vec<u8> {
    let mut ench_list = Vec::new();
    for e in enchantments {
        let mut entry = NbtCompound::new();
        entry.insert("id".into(), NbtTag::Short(e.id));
        entry.insert("lvl".into(), NbtTag::Short(e.level));
        ench_list.push(NbtTag::Compound(entry));
    }

    let mut root_compound = NbtCompound::new();
    root_compound.insert("ench".into(), NbtTag::List(ench_list));
    let root = NbtRoot::new("", root_compound);

    let mut buf = Vec::new();
    mc_rs_nbt::write_nbt_network(&mut buf, &root);
    buf
}

// ---------------------------------------------------------------------------
// Armor defense
// ---------------------------------------------------------------------------

/// Defense points for a single armor piece by item name.
pub fn armor_piece_defense(item_name: &str) -> f32 {
    match item_name {
        // Leather (total 7)
        "minecraft:leather_helmet" => 1.0,
        "minecraft:leather_chestplate" => 3.0,
        "minecraft:leather_leggings" => 2.0,
        "minecraft:leather_boots" => 1.0,
        // Gold (total 11)
        "minecraft:golden_helmet" => 2.0,
        "minecraft:golden_chestplate" => 5.0,
        "minecraft:golden_leggings" => 3.0,
        "minecraft:golden_boots" => 1.0,
        // Chainmail (total 12)
        "minecraft:chainmail_helmet" => 2.0,
        "minecraft:chainmail_chestplate" => 5.0,
        "minecraft:chainmail_leggings" => 4.0,
        "minecraft:chainmail_boots" => 1.0,
        // Iron (total 15)
        "minecraft:iron_helmet" => 2.0,
        "minecraft:iron_chestplate" => 6.0,
        "minecraft:iron_leggings" => 5.0,
        "minecraft:iron_boots" => 2.0,
        // Diamond (total 20)
        "minecraft:diamond_helmet" => 3.0,
        "minecraft:diamond_chestplate" => 8.0,
        "minecraft:diamond_leggings" => 6.0,
        "minecraft:diamond_boots" => 3.0,
        // Netherite (total 20)
        "minecraft:netherite_helmet" => 3.0,
        "minecraft:netherite_chestplate" => 8.0,
        "minecraft:netherite_leggings" => 6.0,
        "minecraft:netherite_boots" => 3.0,
        // Turtle shell (helmet only)
        "minecraft:turtle_helmet" => 2.0,
        _ => 0.0,
    }
}

/// Total armor defense points from armor slots.
pub fn total_armor_defense(registry: &ItemRegistry, armor_slots: &[ItemStack]) -> f32 {
    armor_slots
        .iter()
        .map(|item| {
            if item.is_empty() {
                return 0.0;
            }
            match registry.get_by_id(item.runtime_id as i16) {
                Some(info) => armor_piece_defense(&info.name),
                None => 0.0,
            }
        })
        .sum()
}

/// Apply Bedrock armor reduction formula.
/// `damage_after = damage * (1.0 - min(20, defense) / 25.0)`
pub fn apply_armor_reduction(damage: f32, defense: f32) -> f32 {
    let effective = defense.min(20.0);
    damage * (1.0 - effective / 25.0)
}

// ---------------------------------------------------------------------------
// Enchantment combat bonuses
// ---------------------------------------------------------------------------

/// Extra damage from Sharpness enchantment. +1.25 per level.
pub fn sharpness_bonus(nbt_data: &[u8]) -> f32 {
    parse_enchantments(nbt_data)
        .iter()
        .filter(|e| e.id == enchantment_id::SHARPNESS)
        .map(|e| e.level as f32 * 1.25)
        .sum()
}

/// Protection enchantment damage reduction from all armor pieces.
/// Each level reduces damage by 4% (additive). Cap at 20 levels (80%).
pub fn protection_reduction(armor_nbt_slots: &[&[u8]]) -> f32 {
    let total_levels: i16 = armor_nbt_slots
        .iter()
        .flat_map(|nbt| parse_enchantments(nbt))
        .filter(|e| e.id == enchantment_id::PROTECTION)
        .map(|e| e.level)
        .sum();
    let capped = total_levels.min(20) as f32;
    capped * 0.04
}

/// Knockback enchantment bonus levels.
pub fn knockback_bonus(nbt_data: &[u8]) -> i16 {
    parse_enchantments(nbt_data)
        .iter()
        .filter(|e| e.id == enchantment_id::KNOCKBACK)
        .map(|e| e.level)
        .sum()
}

/// Fire Aspect enchantment level.
pub fn fire_aspect_level(nbt_data: &[u8]) -> i16 {
    enchantment_level_on_item(nbt_data, enchantment_id::FIRE_ASPECT)
}

// ---------------------------------------------------------------------------
// Enchantment effect helpers
// ---------------------------------------------------------------------------

/// Get the level of a specific enchantment on an item. Returns 0 if not present.
fn enchantment_level_on_item(nbt_data: &[u8], enchant_id: i16) -> i16 {
    parse_enchantments(nbt_data)
        .iter()
        .filter(|e| e.id == enchant_id)
        .map(|e| e.level)
        .sum()
}

/// Sum enchantment levels across multiple armor pieces for a given enchantment ID.
fn enchantment_level_on_armor(armor_nbt_slots: &[&[u8]], enchant_id: i16) -> i16 {
    armor_nbt_slots
        .iter()
        .map(|nbt| enchantment_level_on_item(nbt, enchant_id))
        .sum()
}

/// Feather Falling damage reduction from boots. 12% per level, cap at 48% (4 levels).
pub fn feather_falling_reduction(boots_nbt: &[u8]) -> f32 {
    let level = enchantment_level_on_item(boots_nbt, enchantment_id::FEATHER_FALLING);
    (level.min(4) as f32 * 0.12).min(0.48)
}

/// Fire Protection damage reduction from all armor. 8% per level total, cap at 80%.
pub fn fire_protection_reduction(armor_nbt_slots: &[&[u8]]) -> f32 {
    let total = enchantment_level_on_armor(armor_nbt_slots, enchantment_id::FIRE_PROTECTION);
    (total.min(10) as f32 * 0.08).min(0.80)
}

/// Respiration level from helmet. 0 if not present.
pub fn respiration_level(helmet_nbt: &[u8]) -> i16 {
    enchantment_level_on_item(helmet_nbt, enchantment_id::RESPIRATION)
}

/// Efficiency level from tool. 0 if not present.
pub fn efficiency_level(tool_nbt: &[u8]) -> i16 {
    enchantment_level_on_item(tool_nbt, enchantment_id::EFFICIENCY)
}

/// Total Thorns level from all armor pieces.
pub fn thorns_level(armor_nbt_slots: &[&[u8]]) -> i16 {
    enchantment_level_on_armor(armor_nbt_slots, enchantment_id::THORNS)
}

/// Looting level from weapon. 0 if not present.
pub fn looting_level(weapon_nbt: &[u8]) -> i16 {
    enchantment_level_on_item(weapon_nbt, enchantment_id::LOOTING)
}

/// Depth Strider level from boots. 0 if not present.
pub fn depth_strider_level(boots_nbt: &[u8]) -> i16 {
    enchantment_level_on_item(boots_nbt, enchantment_id::DEPTH_STRIDER)
}

/// Whether the helmet has Aqua Affinity (level >= 1).
pub fn has_aqua_affinity(helmet_nbt: &[u8]) -> bool {
    enchantment_level_on_item(helmet_nbt, enchantment_id::AQUA_AFFINITY) > 0
}

// ---------------------------------------------------------------------------
// Critical hits
// ---------------------------------------------------------------------------

/// Check if an attack is a critical hit.
/// Conditions: player is falling (delta_y < 0) and not on ground.
pub fn is_critical_hit(on_ground: bool, delta_y: f32) -> bool {
    !on_ground && delta_y < 0.0
}

/// Critical hit damage multiplier.
pub const CRITICAL_MULTIPLIER: f32 = 1.5;

// ---------------------------------------------------------------------------
// Full damage pipeline
// ---------------------------------------------------------------------------

/// All inputs needed for the damage pipeline.
pub struct DamageInput<'a> {
    pub base_damage: f32,
    pub weapon_nbt: &'a [u8],
    pub armor_defense: f32,
    pub armor_nbt_slots: &'a [&'a [u8]],
    pub is_critical: bool,
    pub strength_bonus: f32,
    pub weakness_penalty: f32,
    pub resistance_factor: f32,
}

/// Calculate final damage after all modifiers.
///
/// Pipeline: base + sharpness + strength - weakness → ×critical → armor → protection → resistance
pub fn calculate_damage(input: &DamageInput) -> f32 {
    let DamageInput {
        base_damage,
        weapon_nbt,
        armor_defense,
        armor_nbt_slots,
        is_critical,
        strength_bonus,
        weakness_penalty,
        resistance_factor,
    } = input;
    let mut damage = *base_damage;

    // Sharpness enchantment
    damage += sharpness_bonus(weapon_nbt);

    // Potion effects
    damage += *strength_bonus;
    damage -= *weakness_penalty;
    damage = damage.max(0.0);

    // Critical hit
    if *is_critical {
        damage *= CRITICAL_MULTIPLIER;
    }

    // Armor reduction
    damage = apply_armor_reduction(damage, *armor_defense);

    // Protection enchantment
    let prot = protection_reduction(armor_nbt_slots);
    damage *= 1.0 - prot;

    // Resistance effect
    damage *= 1.0 - *resistance_factor;

    damage.max(0.0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armor_piece_defense_diamond_full() {
        let total = armor_piece_defense("minecraft:diamond_helmet")
            + armor_piece_defense("minecraft:diamond_chestplate")
            + armor_piece_defense("minecraft:diamond_leggings")
            + armor_piece_defense("minecraft:diamond_boots");
        assert!((total - 20.0).abs() < 0.01);
    }

    #[test]
    fn armor_piece_defense_partial() {
        let total = armor_piece_defense("minecraft:iron_helmet")
            + armor_piece_defense("minecraft:iron_boots");
        assert!((total - 4.0).abs() < 0.01);
    }

    #[test]
    fn armor_piece_defense_unknown() {
        assert!((armor_piece_defense("minecraft:stone")).abs() < 0.01);
    }

    #[test]
    fn armor_reduction_formula() {
        // 10 damage, 20 defense → 10 * (1 - 20/25) = 10 * 0.2 = 2.0
        let result = apply_armor_reduction(10.0, 20.0);
        assert!((result - 2.0).abs() < 0.01);
    }

    #[test]
    fn armor_reduction_cap_at_20() {
        // Defense > 20 is capped at 20
        let with_30 = apply_armor_reduction(10.0, 30.0);
        let with_20 = apply_armor_reduction(10.0, 20.0);
        assert!((with_30 - with_20).abs() < 0.01);
    }

    #[test]
    fn parse_enchantments_empty() {
        assert!(parse_enchantments(&[]).is_empty());
    }

    #[test]
    fn parse_enchantments_roundtrip() {
        let enchants = vec![Enchantment {
            id: enchantment_id::SHARPNESS,
            level: 5,
        }];
        let nbt = build_enchantment_nbt(&enchants);
        let parsed = parse_enchantments(&nbt);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, enchantment_id::SHARPNESS);
        assert_eq!(parsed[0].level, 5);
    }

    #[test]
    fn parse_enchantments_multiple() {
        let enchants = vec![
            Enchantment {
                id: enchantment_id::SHARPNESS,
                level: 3,
            },
            Enchantment {
                id: enchantment_id::KNOCKBACK,
                level: 2,
            },
        ];
        let nbt = build_enchantment_nbt(&enchants);
        let parsed = parse_enchantments(&nbt);
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn sharpness_bonus_level_5() {
        let nbt = build_enchantment_nbt(&[Enchantment {
            id: enchantment_id::SHARPNESS,
            level: 5,
        }]);
        let bonus = sharpness_bonus(&nbt);
        assert!((bonus - 6.25).abs() < 0.01);
    }

    #[test]
    fn protection_reduction_full() {
        // Protection IV on all 4 armor pieces → 4*4 = 16 levels → 64%
        let prot4_nbt = build_enchantment_nbt(&[Enchantment {
            id: enchantment_id::PROTECTION,
            level: 4,
        }]);
        let slots: Vec<&[u8]> = vec![&prot4_nbt, &prot4_nbt, &prot4_nbt, &prot4_nbt];
        let reduction = protection_reduction(&slots);
        assert!((reduction - 0.64).abs() < 0.01);
    }

    #[test]
    fn is_critical_falling() {
        assert!(is_critical_hit(false, -0.5));
    }

    #[test]
    fn is_critical_on_ground() {
        assert!(!is_critical_hit(true, -0.5));
        assert!(!is_critical_hit(false, 0.1));
    }

    // ── Enchantment registry tests ──────────────────────────────────────

    #[test]
    fn enchantment_by_name_found() {
        let info = enchantment_by_name("sharpness").unwrap();
        assert_eq!(info.id, 9);
        assert_eq!(info.max_level, 5);
    }

    #[test]
    fn enchantment_by_name_not_found() {
        assert!(enchantment_by_name("nonexistent").is_none());
    }

    #[test]
    fn enchantment_by_id_found() {
        let info = enchantment_by_id(2).unwrap();
        assert_eq!(info.name, "feather_falling");
        assert_eq!(info.max_level, 4);
    }

    #[test]
    fn enchantment_by_id_not_found() {
        assert!(enchantment_by_id(999).is_none());
    }

    #[test]
    fn enchantment_list_complete() {
        // Should have 37 entries (IDs 0..36)
        assert_eq!(ENCHANTMENT_LIST.len(), 37);
        // IDs should be sequential
        for (i, info) in ENCHANTMENT_LIST.iter().enumerate() {
            assert_eq!(info.id, i as i16);
        }
    }

    // ── Effect helpers ──────────────────────────────────────────────────

    #[test]
    fn feather_falling_reduction_values() {
        // No enchantment
        assert!((feather_falling_reduction(&[]) - 0.0).abs() < 0.001);

        // Level 4 → 48%
        let nbt = build_enchantment_nbt(&[Enchantment {
            id: enchantment_id::FEATHER_FALLING,
            level: 4,
        }]);
        assert!((feather_falling_reduction(&nbt) - 0.48).abs() < 0.001);

        // Level 2 → 24%
        let nbt2 = build_enchantment_nbt(&[Enchantment {
            id: enchantment_id::FEATHER_FALLING,
            level: 2,
        }]);
        assert!((feather_falling_reduction(&nbt2) - 0.24).abs() < 0.001);
    }

    #[test]
    fn fire_protection_reduction_values() {
        // Fire Protection IV on 2 pieces → 8 levels → 64%
        let fp4 = build_enchantment_nbt(&[Enchantment {
            id: enchantment_id::FIRE_PROTECTION,
            level: 4,
        }]);
        let slots: Vec<&[u8]> = vec![&fp4, &fp4, &[], &[]];
        assert!((fire_protection_reduction(&slots) - 0.64).abs() < 0.001);
    }

    #[test]
    fn thorns_level_multi_piece() {
        let t2 = build_enchantment_nbt(&[Enchantment {
            id: enchantment_id::THORNS,
            level: 2,
        }]);
        let t1 = build_enchantment_nbt(&[Enchantment {
            id: enchantment_id::THORNS,
            level: 1,
        }]);
        let slots: Vec<&[u8]> = vec![&t2, &t1, &[], &[]];
        assert_eq!(thorns_level(&slots), 3);
    }

    #[test]
    fn efficiency_level_value() {
        let nbt = build_enchantment_nbt(&[Enchantment {
            id: enchantment_id::EFFICIENCY,
            level: 5,
        }]);
        assert_eq!(efficiency_level(&nbt), 5);
        assert_eq!(efficiency_level(&[]), 0);
    }

    #[test]
    fn respiration_and_looting_levels() {
        let resp3 = build_enchantment_nbt(&[Enchantment {
            id: enchantment_id::RESPIRATION,
            level: 3,
        }]);
        assert_eq!(respiration_level(&resp3), 3);

        let loot2 = build_enchantment_nbt(&[Enchantment {
            id: enchantment_id::LOOTING,
            level: 2,
        }]);
        assert_eq!(looting_level(&loot2), 2);
    }

    #[test]
    fn aqua_affinity_check() {
        assert!(!has_aqua_affinity(&[]));
        let aa = build_enchantment_nbt(&[Enchantment {
            id: enchantment_id::AQUA_AFFINITY,
            level: 1,
        }]);
        assert!(has_aqua_affinity(&aa));
    }
}
