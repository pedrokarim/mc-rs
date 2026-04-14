//! Weapon / tool attack stats — PMMP `src/item/Tool.php` attack points.

use crate::durability::ToolTier;

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
    fn diamond_axe_more_than_sword() {
        // In vanilla, diamond axe = 9, diamond sword = 8 (in Bedrock).
        assert!(axe_damage(ToolTier::Diamond) >= sword_damage(ToolTier::Diamond));
    }
}
