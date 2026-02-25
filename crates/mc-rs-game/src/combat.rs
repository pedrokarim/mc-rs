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
    pub const SHARPNESS: i16 = 9;
    pub const KNOCKBACK: i16 = 12;
    pub const FIRE_ASPECT: i16 = 13;
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
    parse_enchantments(nbt_data)
        .iter()
        .filter(|e| e.id == enchantment_id::FIRE_ASPECT)
        .map(|e| e.level)
        .sum()
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
}
