//! Arrow physics + types.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowType {
    Normal,
    Tipped(PotionEffect),
    Spectral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotionEffect {
    Strength, Weakness, Swiftness, Slowness,
    FireResistance, NightVision, Invisibility,
    Healing, Harming, Poison, Regeneration,
    Leaping, WaterBreathing, Luck, Unluck,
    SlowFalling, TurtleMaster, Blindness,
}

#[derive(Debug, Clone)]
pub struct Arrow {
    pub arrow_type: ArrowType,
    pub power_level: u8,       // from Power enchant
    pub punch_level: u8,       // from Punch enchant
    pub flame: bool,           // from Flame enchant
    pub infinity: bool,        // Infinity enchant (on bow, not arrow)
    pub piercing_level: u8,    // From Piercing (crossbow)
    pub critical: bool,        // Fully drawn bow
    pub pickup_mode: PickupMode,
    pub age: u32,
    pub shot_from_crossbow: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickupMode {
    Disallowed,
    AllowedByAny,
    CreativeOnly,
}

/// Despawn after 1200 ticks (60s).
pub const DESPAWN_TICKS: u32 = 1200;

/// Base damage (2.0).
pub const BASE_DAMAGE: f32 = 2.0;

impl Arrow {
    pub fn new(arrow_type: ArrowType) -> Self {
        Self {
            arrow_type,
            power_level: 0,
            punch_level: 0,
            flame: false,
            infinity: false,
            piercing_level: 0,
            critical: false,
            pickup_mode: PickupMode::AllowedByAny,
            age: 0,
            shot_from_crossbow: false,
        }
    }

    /// Damage scales: base × (power * 0.25 + 1) × velocity.
    pub fn damage_at(&self, velocity_factor: f32) -> f32 {
        let base = BASE_DAMAGE * velocity_factor;
        let power_bonus = self.power_level as f32 * 0.25 + if self.power_level > 0 { 0.25 } else { 0.0 };
        let mut dmg = base + power_bonus * base;
        if self.critical {
            dmg += rand::random::<f32>() * (dmg / 2.0 + 2.0);
        }
        dmg
    }

    /// Knockback in units of 0.6.
    pub fn knockback(&self) -> f32 {
        self.punch_level as f32 * 0.6
    }

    pub fn is_expired(&self) -> bool {
        self.age >= DESPAWN_TICKS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_increases_damage() {
        let mut a = Arrow::new(ArrowType::Normal);
        let base = a.damage_at(1.0);
        a.power_level = 5;
        assert!(a.damage_at(1.0) > base);
    }
}
