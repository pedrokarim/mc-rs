//! Damage modifiers — port PMMP `src/entity/Living.php::applyDamageModifiers`.
//! Calculs de défense finals : armor + enchants + effects.

use crate::effects::{EffectKind, EffectManager};

/// Apply Protection-type enchant damage reduction.
/// PMMP `EPF (Enchantment Protection Factor)`.
pub fn protection_reduction(
    protection_levels: u32,
    fire_protection_levels: u32,
    blast_protection_levels: u32,
    projectile_protection_levels: u32,
    damage_kind: DamageKind,
) -> f32 {
    let epf = match damage_kind {
        DamageKind::All => protection_levels,
        DamageKind::Fire => protection_levels + 2 * fire_protection_levels,
        DamageKind::Blast => protection_levels + 2 * blast_protection_levels,
        DamageKind::Projectile => protection_levels + 2 * projectile_protection_levels,
    };
    let cap = 20.min(epf) as f32;
    cap / 25.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageKind {
    All,
    Fire,
    Blast,
    Projectile,
}

/// Apply Resistance effect damage reduction.
/// PMMP : resistance N → -20%*N damage.
pub fn resistance_reduction(effects: &EffectManager) -> f32 {
    if let Some(e) = effects.get(EffectKind::Resistance) {
        let lvl = e.effect_level() as f32;
        (lvl * 0.2).min(1.0)
    } else {
        0.0
    }
}

/// Apply full damage pipeline :
///   base → armor → protection enchant → resistance effect.
pub fn compute_final_damage(
    base: f32,
    armor_reduction: f32,
    protection_pct: f32,
    resistance_pct: f32,
) -> f32 {
    let after_armor = base - armor_reduction;
    let after_protection = after_armor * (1.0 - protection_pct);
    let after_resistance = after_protection * (1.0 - resistance_pct);
    after_resistance.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protection_cap_at_20() {
        let r = protection_reduction(30, 0, 0, 0, DamageKind::All);
        assert!(r <= 0.8); // 20/25 = 0.8
    }

    #[test]
    fn fire_protection_bonus_for_fire() {
        let fire = protection_reduction(0, 4, 0, 0, DamageKind::Fire);
        let other = protection_reduction(0, 4, 0, 0, DamageKind::All);
        assert!(fire > other);
    }

    #[test]
    fn final_damage_after_full_pipeline() {
        let d = compute_final_damage(10.0, 3.0, 0.2, 0.2);
        // after armor : 7, after prot : 5.6, after res : 4.48
        assert!((d - 4.48).abs() < 0.01);
    }
}
