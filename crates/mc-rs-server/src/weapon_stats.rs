//! Weapon / tool attack stats — PMMP `src/item/Tool.php` attack points.

use crate::durability::{durable_info, ToolTier, ToolType};

/// Dégâts d'une attaque mêlée du joueur selon l'item tenu (network id). Poing nu
/// ou item non-arme = 1.0 (base vanilla) ; épée/hache/pioche/pelle/houe via les
/// tables PMMP. (Trident/masse : à ajouter si nécessaire.)
pub fn player_attack_damage(held_item_id: i32) -> f32 {
    match durable_info(held_item_id) {
        Some(info) => match info.tool_type {
            ToolType::Sword => sword_damage(info.tier),
            ToolType::Axe => axe_damage(info.tier),
            ToolType::Pickaxe => pickaxe_damage(info.tier),
            ToolType::Shovel => shovel_damage(info.tier),
            ToolType::Hoe => hoe_damage(info.tier),
            ToolType::Shears | ToolType::Armor => 1.0,
        },
        None => 1.0,
    }
}

pub fn sword_damage(tier: ToolTier) -> f32 {
    match tier {
        ToolTier::Wood | ToolTier::Gold => 5.0,
        ToolTier::Stone => 6.0,
        ToolTier::Iron => 7.0,
        ToolTier::Diamond => 8.0,
        ToolTier::Netherite => 9.0,
    }
}

pub fn axe_damage(tier: ToolTier) -> f32 {
    match tier {
        ToolTier::Wood => 7.0,
        ToolTier::Gold => 7.0,
        ToolTier::Stone => 9.0,
        ToolTier::Iron => 9.0,
        ToolTier::Diamond => 9.0,
        ToolTier::Netherite => 10.0,
    }
}

pub fn pickaxe_damage(tier: ToolTier) -> f32 {
    match tier {
        ToolTier::Wood | ToolTier::Gold => 2.0,
        ToolTier::Stone => 3.0,
        ToolTier::Iron => 4.0,
        ToolTier::Diamond => 5.0,
        ToolTier::Netherite => 6.0,
    }
}

pub fn shovel_damage(tier: ToolTier) -> f32 {
    match tier {
        ToolTier::Wood | ToolTier::Gold => 2.5,
        ToolTier::Stone => 3.5,
        ToolTier::Iron => 4.5,
        ToolTier::Diamond => 5.5,
        ToolTier::Netherite => 6.5,
    }
}

pub fn hoe_damage(tier: ToolTier) -> f32 {
    1.0 + tier.base_attack_points() as f32 * 0.25
}

pub fn trident_damage() -> f32 {
    9.0
}

pub fn bow_max_damage() -> f32 {
    // Full-charge arrow = 2 + 2*strength modifier. Base.
    5.0
}

pub fn mace_damage() -> f32 {
    5.0 // base ; multiplié par fall distance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netherite_sword_9() {
        assert_eq!(sword_damage(ToolTier::Netherite), 9.0);
    }

    #[test]
    fn fist_deals_one() {
        // Item inconnu / poing nu = 1.0 (base vanilla), plus le 4.0 placeholder.
        assert_eq!(player_attack_damage(0), 1.0);
    }

    #[test]
    fn held_sword_scales_damage() {
        if let Some(id) = crate::item_registry::network_id("minecraft:netherite_sword") {
            assert_eq!(player_attack_damage(id), 9.0);
        }
        if let Some(id) = crate::item_registry::network_id("minecraft:wooden_sword") {
            assert_eq!(player_attack_damage(id), 5.0);
        }
    }

    #[test]
    fn diamond_axe_more_than_sword() {
        // In vanilla, diamond axe = 9, diamond sword = 8 (in Bedrock).
        assert!(axe_damage(ToolTier::Diamond) >= sword_damage(ToolTier::Diamond));
    }
}
