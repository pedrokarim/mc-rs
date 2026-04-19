//! Durabilité outils — port de `.reference/PocketMine-MP/src/item/Durable.php`.
//!
//! Les outils (pioche, hache, épée, etc.) et armures ont une durabilité max
//! et prennent des dégâts à chaque usage. Quand `damage == max_durability`,
//! l'outil casse et disparaît de l'inventaire.
//!
//! La durabilité est stockée dans `ItemStack.meta` (convention PMMP :
//! `meta = damage` pour les items Durable).

use mc_rs_proto::packets::player::ItemStack;

/// Tiers d'outils vanilla — port PMMP `Tool*` + `TieredTool.php`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolTier {
    Wood,
    Stone,
    Gold,
    Iron,
    Diamond,
    Netherite,
}

impl ToolTier {
    /// Durabilité max par tier (PMMP `TieredTool::getMaxDurability()`).
    pub fn max_durability(self) -> u32 {
        match self {
            Self::Wood => 59,
            Self::Stone => 131,
            Self::Gold => 32,
            Self::Iron => 250,
            Self::Diamond => 1561,
            Self::Netherite => 2031,
        }
    }

    /// Rang de minage (PMMP `TieredTool::getMiningTier`). Utilisé pour
    /// décider si un outil peut drop un ore (comparé à `min_tool_tier_for_drop`).
    /// Gold == Wood (Gold pickaxe n'améliore pas le mining tier).
    pub fn mining_tier(self) -> u8 {
        match self {
            Self::Wood | Self::Gold => 0,
            Self::Stone => 1,
            Self::Iron => 2,
            Self::Diamond => 3,
            Self::Netherite => 4,
        }
    }

    /// Attack damage en demi-cœurs (PMMP `TieredTool::getBaseAttackPoints()`).
    pub fn base_attack_points(self) -> u32 {
        match self {
            Self::Wood | Self::Gold => 2,
            Self::Stone => 3,
            Self::Iron => 4,
            Self::Diamond => 5,
            Self::Netherite => 6,
        }
    }

    /// Multiplicateur de vitesse de cassage (PMMP).
    pub fn mining_speed(self) -> f32 {
        match self {
            Self::Wood => 2.0,
            Self::Stone => 4.0,
            Self::Iron => 6.0,
            Self::Gold => 12.0,
            Self::Diamond => 8.0,
            Self::Netherite => 9.0,
        }
    }
}

/// Catégorie d'item pour déterminer les types de blocs cassables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    Pickaxe,
    Axe,
    Shovel,
    Hoe,
    Sword,
    Shears,
    Armor,
}

/// Metadata d'un item durable. Vérifiable en consultant le network ID.
#[derive(Debug, Clone, Copy)]
pub struct DurableInfo {
    pub tier: ToolTier,
    pub tool_type: ToolType,
}

/// Retourne `Some(DurableInfo)` si `item_network_id` correspond à un outil
/// durable connu. On maintient une table simple pour les outils vanilla
/// courants. Étendre selon besoin.
pub fn durable_info(item_network_id: i32) -> Option<DurableInfo> {
    use crate::item_registry::required_item_id;
    let map = &[
        (
            "minecraft:wooden_pickaxe",
            ToolTier::Wood,
            ToolType::Pickaxe,
        ),
        (
            "minecraft:stone_pickaxe",
            ToolTier::Stone,
            ToolType::Pickaxe,
        ),
        ("minecraft:iron_pickaxe", ToolTier::Iron, ToolType::Pickaxe),
        (
            "minecraft:golden_pickaxe",
            ToolTier::Gold,
            ToolType::Pickaxe,
        ),
        (
            "minecraft:diamond_pickaxe",
            ToolTier::Diamond,
            ToolType::Pickaxe,
        ),
        (
            "minecraft:netherite_pickaxe",
            ToolTier::Netherite,
            ToolType::Pickaxe,
        ),
        ("minecraft:wooden_axe", ToolTier::Wood, ToolType::Axe),
        ("minecraft:stone_axe", ToolTier::Stone, ToolType::Axe),
        ("minecraft:iron_axe", ToolTier::Iron, ToolType::Axe),
        ("minecraft:golden_axe", ToolTier::Gold, ToolType::Axe),
        ("minecraft:diamond_axe", ToolTier::Diamond, ToolType::Axe),
        (
            "minecraft:netherite_axe",
            ToolTier::Netherite,
            ToolType::Axe,
        ),
        ("minecraft:wooden_shovel", ToolTier::Wood, ToolType::Shovel),
        ("minecraft:stone_shovel", ToolTier::Stone, ToolType::Shovel),
        ("minecraft:iron_shovel", ToolTier::Iron, ToolType::Shovel),
        ("minecraft:golden_shovel", ToolTier::Gold, ToolType::Shovel),
        (
            "minecraft:diamond_shovel",
            ToolTier::Diamond,
            ToolType::Shovel,
        ),
        (
            "minecraft:netherite_shovel",
            ToolTier::Netherite,
            ToolType::Shovel,
        ),
        ("minecraft:wooden_sword", ToolTier::Wood, ToolType::Sword),
        ("minecraft:stone_sword", ToolTier::Stone, ToolType::Sword),
        ("minecraft:iron_sword", ToolTier::Iron, ToolType::Sword),
        ("minecraft:golden_sword", ToolTier::Gold, ToolType::Sword),
        (
            "minecraft:diamond_sword",
            ToolTier::Diamond,
            ToolType::Sword,
        ),
        (
            "minecraft:netherite_sword",
            ToolTier::Netherite,
            ToolType::Sword,
        ),
        ("minecraft:wooden_hoe", ToolTier::Wood, ToolType::Hoe),
        ("minecraft:stone_hoe", ToolTier::Stone, ToolType::Hoe),
        ("minecraft:iron_hoe", ToolTier::Iron, ToolType::Hoe),
        ("minecraft:golden_hoe", ToolTier::Gold, ToolType::Hoe),
        ("minecraft:diamond_hoe", ToolTier::Diamond, ToolType::Hoe),
        (
            "minecraft:netherite_hoe",
            ToolTier::Netherite,
            ToolType::Hoe,
        ),
        ("minecraft:shears", ToolTier::Iron, ToolType::Shears),
    ];
    for (name, tier, ty) in map {
        if required_item_id(name) == item_network_id {
            return Some(DurableInfo {
                tier: *tier,
                tool_type: *ty,
            });
        }
    }
    None
}

/// Applique un dégât à un ItemStack Durable. PMMP `Durable::applyDamage()`.
/// `amount` est typiquement 1 par usage (break bloc, attack mob).
/// Retourne `true` si l'item est maintenant cassé (count doit passer à 0).
pub fn apply_damage(stack: &mut ItemStack, amount: u32) -> bool {
    let Some(info) = durable_info(stack.id) else {
        return false;
    };
    let max = info.tier.max_durability();
    let new_damage = (stack.meta + amount).min(max);
    stack.meta = new_damage;
    new_damage >= max
}

pub fn is_broken(stack: &ItemStack) -> bool {
    let Some(info) = durable_info(stack.id) else {
        return false;
    };
    stack.meta >= info.tier.max_durability()
}

/// `Durable::getMaxDurability()` PMMP.
pub fn max_durability(stack: &ItemStack) -> Option<u32> {
    durable_info(stack.id).map(|i| i.tier.max_durability())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_max_durability() {
        assert_eq!(ToolTier::Wood.max_durability(), 59);
        assert_eq!(ToolTier::Netherite.max_durability(), 2031);
    }
}
