//! Armor — port sélectif de `.reference/PocketMine-MP/src/item/Armor.php` +
//! `src/entity/Living.php::applyDamageModifiers` (armor reduction).
//!
//! L'armure réduit les dégâts reçus selon le défense points total + enchants.

use mc_rs_proto::packets::player::ItemStack;

/// Slots d'armure PMMP `ArmorInventory::SLOT_*`.
pub const SLOT_HEAD: usize = 0;
pub const SLOT_CHEST: usize = 1;
pub const SLOT_LEGS: usize = 2;
pub const SLOT_FEET: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorMaterial {
    Leather,
    Gold,
    Chainmail,
    Iron,
    Diamond,
    Netherite,
    Turtle,
}

impl ArmorMaterial {
    /// Points de défense par slot pour ce matériau. PMMP `Armor::defensePoints`.
    pub fn defense_points(&self, slot: usize) -> u32 {
        match self {
            Self::Leather => match slot {
                SLOT_HEAD => 1,
                SLOT_CHEST => 3,
                SLOT_LEGS => 2,
                SLOT_FEET => 1,
                _ => 0,
            },
            Self::Gold => match slot {
                SLOT_HEAD => 2,
                SLOT_CHEST => 5,
                SLOT_LEGS => 3,
                SLOT_FEET => 1,
                _ => 0,
            },
            Self::Chainmail => match slot {
                SLOT_HEAD => 2,
                SLOT_CHEST => 5,
                SLOT_LEGS => 4,
                SLOT_FEET => 1,
                _ => 0,
            },
            Self::Iron => match slot {
                SLOT_HEAD => 2,
                SLOT_CHEST => 6,
                SLOT_LEGS => 5,
                SLOT_FEET => 2,
                _ => 0,
            },
            Self::Diamond | Self::Netherite => match slot {
                SLOT_HEAD => 3,
                SLOT_CHEST => 8,
                SLOT_LEGS => 6,
                SLOT_FEET => 3,
                _ => 0,
            },
            Self::Turtle => match slot {
                SLOT_HEAD => 2,
                _ => 0,
            },
        }
    }

    pub fn toughness(&self) -> f32 {
        match self {
            Self::Diamond => 2.0,
            Self::Netherite => 3.0,
            _ => 0.0,
        }
    }
}

/// Lookup armor info depuis le network id d'un item. PMMP `ArmorMaterials`.
pub fn armor_info(item_network_id: i32) -> Option<(ArmorMaterial, usize)> {
    use crate::item_registry::required_item_id;
    let table: &[(&str, ArmorMaterial, usize)] = &[
        (
            "minecraft:leather_helmet",
            ArmorMaterial::Leather,
            SLOT_HEAD,
        ),
        (
            "minecraft:leather_chestplate",
            ArmorMaterial::Leather,
            SLOT_CHEST,
        ),
        (
            "minecraft:leather_leggings",
            ArmorMaterial::Leather,
            SLOT_LEGS,
        ),
        ("minecraft:leather_boots", ArmorMaterial::Leather, SLOT_FEET),
        (
            "minecraft:chainmail_helmet",
            ArmorMaterial::Chainmail,
            SLOT_HEAD,
        ),
        (
            "minecraft:chainmail_chestplate",
            ArmorMaterial::Chainmail,
            SLOT_CHEST,
        ),
        (
            "minecraft:chainmail_leggings",
            ArmorMaterial::Chainmail,
            SLOT_LEGS,
        ),
        (
            "minecraft:chainmail_boots",
            ArmorMaterial::Chainmail,
            SLOT_FEET,
        ),
        ("minecraft:iron_helmet", ArmorMaterial::Iron, SLOT_HEAD),
        ("minecraft:iron_chestplate", ArmorMaterial::Iron, SLOT_CHEST),
        ("minecraft:iron_leggings", ArmorMaterial::Iron, SLOT_LEGS),
        ("minecraft:iron_boots", ArmorMaterial::Iron, SLOT_FEET),
        ("minecraft:golden_helmet", ArmorMaterial::Gold, SLOT_HEAD),
        (
            "minecraft:golden_chestplate",
            ArmorMaterial::Gold,
            SLOT_CHEST,
        ),
        ("minecraft:golden_leggings", ArmorMaterial::Gold, SLOT_LEGS),
        ("minecraft:golden_boots", ArmorMaterial::Gold, SLOT_FEET),
        (
            "minecraft:diamond_helmet",
            ArmorMaterial::Diamond,
            SLOT_HEAD,
        ),
        (
            "minecraft:diamond_chestplate",
            ArmorMaterial::Diamond,
            SLOT_CHEST,
        ),
        (
            "minecraft:diamond_leggings",
            ArmorMaterial::Diamond,
            SLOT_LEGS,
        ),
        ("minecraft:diamond_boots", ArmorMaterial::Diamond, SLOT_FEET),
        (
            "minecraft:netherite_helmet",
            ArmorMaterial::Netherite,
            SLOT_HEAD,
        ),
        (
            "minecraft:netherite_chestplate",
            ArmorMaterial::Netherite,
            SLOT_CHEST,
        ),
        (
            "minecraft:netherite_leggings",
            ArmorMaterial::Netherite,
            SLOT_LEGS,
        ),
        (
            "minecraft:netherite_boots",
            ArmorMaterial::Netherite,
            SLOT_FEET,
        ),
        ("minecraft:turtle_helmet", ArmorMaterial::Turtle, SLOT_HEAD),
    ];
    for (name, mat, slot) in table {
        if required_item_id(name) == item_network_id {
            return Some((*mat, *slot));
        }
    }
    None
}

/// Calcule les dégâts reçus après réduction d'armure.
/// Formule PMMP simplifiée (armor points + toughness) :
///   reduction = min(20, max(defense / 5, defense - damage / (2 + toughness/4)))
///   final = damage * (1 - reduction/25)
pub fn apply_armor_reduction(damage: f32, armor_slots: &[ItemStack; 4]) -> f32 {
    let mut total_defense = 0u32;
    let mut total_toughness = 0.0f32;
    for (slot, item) in armor_slots.iter().enumerate() {
        if item.is_air() {
            continue;
        }
        if let Some((mat, expected_slot)) = armor_info(item.id) {
            if expected_slot == slot {
                total_defense += mat.defense_points(slot);
                total_toughness += mat.toughness();
            }
        }
    }
    if total_defense == 0 {
        return damage;
    }
    let defense_f = total_defense as f32;
    let reduction_pct =
        ((defense_f / 5.0).max(defense_f - damage / (2.0 + total_toughness / 4.0))).min(20.0);
    damage * (1.0 - reduction_pct / 25.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leather_reduces_damage() {
        // 1 + 3 + 2 + 1 = 7 defense points
        // Pour damage 10 : reduction_pct = max(7/5=1.4, 7 - 10/2=2) = 2. Final = 10*(1-2/25) = 9.2.
        let armor = [
            ItemStack {
                id: crate::item_registry::required_item_id("minecraft:leather_helmet"),
                count: 1,
                meta: 0,
                block_runtime_id: 0,
                extra_data: vec![],
            },
            ItemStack {
                id: crate::item_registry::required_item_id("minecraft:leather_chestplate"),
                count: 1,
                meta: 0,
                block_runtime_id: 0,
                extra_data: vec![],
            },
            ItemStack {
                id: crate::item_registry::required_item_id("minecraft:leather_leggings"),
                count: 1,
                meta: 0,
                block_runtime_id: 0,
                extra_data: vec![],
            },
            ItemStack {
                id: crate::item_registry::required_item_id("minecraft:leather_boots"),
                count: 1,
                meta: 0,
                block_runtime_id: 0,
                extra_data: vec![],
            },
        ];
        let reduced = apply_armor_reduction(10.0, &armor);
        assert!(reduced < 10.0);
        assert!(reduced > 5.0);
    }

    #[test]
    fn no_armor_full_damage() {
        let armor = [
            ItemStack::AIR,
            ItemStack::AIR,
            ItemStack::AIR,
            ItemStack::AIR,
        ];
        assert_eq!(apply_armor_reduction(10.0, &armor), 10.0);
    }
}
