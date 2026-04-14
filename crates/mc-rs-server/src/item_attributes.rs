//! Item attribute modifiers (generic attribute system).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeKind {
    MaxHealth,
    MovementSpeed,
    AttackDamage,
    AttackSpeed,
    AttackKnockback,
    Armor,
    ArmorToughness,
    KnockbackResistance,
    Luck,
    FlyingSpeed,
    FollowRange,
    SpawnReinforcements,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeOp {
    Add,
    MultiplyBase,
    Multiply,
}

#[derive(Debug, Clone)]
pub struct AttributeModifier {
    pub name: String,
    pub attribute: AttributeKind,
    pub operation: AttributeOp,
    pub amount: f64,
    pub slot: Option<u8>, // restrict to slot (main hand, off hand, armor)
}

/// Tool damage modifiers per tool type.
pub fn tool_attack_damage(tier: &str) -> f64 {
    match tier {
        "wooden" => 4.0,
        "stone" => 5.0,
        "iron" => 6.0,
        "gold" => 4.0,
        "diamond" => 7.0,
        "netherite" => 8.0,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netherite_highest_damage() {
        assert!(tool_attack_damage("netherite") > tool_attack_damage("iron"));
    }
}
