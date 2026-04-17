//! Beacon — port PMMP `src/block/Beacon.php` + `BeaconInventory`.
//!
//! Beacon = base de blocs précieux (fer/or/diamant/émeraude/netherite) en
//! pyramide qui donne des effets de statut aux joueurs alentours. Chaque
//! niveau (1-4) étend la range ET débloque de nouveaux effets.

use crate::effects::EffectKind;

/// Primaires disponibles par level.
pub fn primary_effects(level: u8) -> Vec<EffectKind> {
    match level {
        1 => vec![EffectKind::Speed, EffectKind::Haste],
        2 => vec![
            EffectKind::Speed,
            EffectKind::Haste,
            EffectKind::Resistance,
            EffectKind::JumpBoost,
        ],
        3 => vec![
            EffectKind::Speed,
            EffectKind::Haste,
            EffectKind::Resistance,
            EffectKind::JumpBoost,
            EffectKind::Strength,
        ],
        _ => vec![
            EffectKind::Speed,
            EffectKind::Haste,
            EffectKind::Resistance,
            EffectKind::JumpBoost,
            EffectKind::Strength,
        ],
    }
}

/// Secondaires disponibles : Regeneration seulement, au level 4.
pub fn secondary_effects(level: u8) -> Vec<EffectKind> {
    if level >= 4 {
        vec![EffectKind::Regeneration]
    } else {
        vec![]
    }
}

/// Range (en blocs) autour du beacon.
pub fn effect_range(level: u8) -> u32 {
    10 + level as u32 * 10
}

/// Durée de l'effet appliqué (ticks). Réappliqué régulièrement.
pub fn effect_duration() -> i32 {
    9 * 20 // 9 seconds, refresh every 4s
}

#[derive(Debug, Clone)]
pub struct BeaconState {
    pub level: u8,
    pub primary: Option<EffectKind>,
    pub secondary: Option<EffectKind>,
    /// Materials used as payment (item network ID).
    pub payment_item_id: Option<i32>,
}

impl BeaconState {
    pub fn new() -> Self {
        Self {
            level: 0,
            primary: None,
            secondary: None,
            payment_item_id: None,
        }
    }

    pub fn can_activate_primary(&self, effect: EffectKind) -> bool {
        primary_effects(self.level).contains(&effect)
    }

    pub fn can_activate_secondary(&self, effect: EffectKind) -> bool {
        secondary_effects(self.level).contains(&effect)
    }
}

impl Default for BeaconState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_1_only_speed_haste() {
        let effects = primary_effects(1);
        assert_eq!(effects, vec![EffectKind::Speed, EffectKind::Haste]);
    }

    #[test]
    fn level_4_has_strength_and_regen() {
        assert!(primary_effects(4).contains(&EffectKind::Strength));
        assert!(secondary_effects(4).contains(&EffectKind::Regeneration));
    }

    #[test]
    fn range_scales_with_level() {
        assert_eq!(effect_range(1), 20);
        assert_eq!(effect_range(4), 50);
    }
}
