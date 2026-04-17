//! Combat — port sélectif de `.reference/PocketMine-MP/src/entity/Living.php`
//! (attack, applyDamageModifiers, knockBack) + `Player.php`.
//!
//! Le système combat repose sur :
//! - `AttributeMap` (bloc 2) pour stocker HEALTH / ATTACK_DAMAGE / KNOCKBACK_RESISTANCE
//! - `EventManager` (bloc 1) pour dispatcher `EntityDamageEvent` / `EntityDeathEvent`
//! - `noDamageTicks` (i-frames) pour éviter double-hit
//!
//! Ce module expose des fonctions pures qui prennent l'état nécessaire en
//! paramètre ; l'intégration dans `Connection` se fait à l'appel.

use crate::attribute::{ids, AttributeMap};
use crate::event::entity::{DamageCause, EntityDamageEvent, EntityDeathEvent};
use crate::event::EventManager;

/// Constantes PMMP.
pub const DEFAULT_KNOCKBACK_FORCE: f32 = 0.4;
pub const DEFAULT_KNOCKBACK_VERTICAL_LIMIT: f32 = 0.4;
/// `Living::DEFAULT_NO_DAMAGE_TICKS = 10` (ticks à 20 TPS = 0.5s).
pub const DEFAULT_NO_DAMAGE_TICKS: i32 = 10;
pub const DEFAULT_ATTACK_COOLDOWN: i32 = 10;

/// État combat par-entité — à stocker à côté de `AttributeMap`.
/// Port de champs `Living.php::attackTime`, `noDamageTicks`, etc.
#[derive(Debug, Clone, Default)]
pub struct CombatState {
    /// Ticks restants avant de pouvoir être ré-attaqué (i-frames).
    pub attack_time: i32,
    /// `Living::noDamageTicks` : ticks d'invincibilité après spawn/respawn.
    pub no_damage_ticks: i32,
    /// Base damage du dernier coup reçu (pour ignore si < qu'un coup futur simultané).
    pub last_damage_cause_base: Option<f32>,
}

impl CombatState {
    pub fn new() -> Self {
        Self {
            attack_time: 0,
            no_damage_ticks: 20, // PMMP : 1s d'i-frames au spawn
            last_damage_cause_base: None,
        }
    }

    /// À appeler chaque tick (20 TPS). Décrémente i-frames.
    pub fn tick(&mut self) {
        if self.attack_time > 0 {
            self.attack_time -= 1;
        }
        if self.no_damage_ticks > 0 {
            self.no_damage_ticks -= 1;
        }
    }

    pub fn is_invincible(&self) -> bool {
        self.no_damage_ticks > 0 || self.attack_time > 0
    }
}

/// Résultat d'un attack() — sert à l'appelant pour appliquer les side-effects
/// (envoyer des paquets, spawn drops, etc.).
pub struct AttackOutcome {
    /// Dégâts effectifs appliqués (après événement + modifiers).
    pub applied_damage: f32,
    /// Entité est morte suite à cette attaque.
    pub died: bool,
    /// Knockback à appliquer : (dx, dz) unités à pousser.
    pub knockback: Option<(f32, f32, f32)>, // (x, y, z motion to add
    /// Event originel (consultable pour display_message, drops modifiés, etc.).
    pub event_cancelled: bool,
}

/// Port de `Living::attack()`. Applique un dégât à une entité :
/// - fire `EntityDamageEvent` (cancellable par plugins)
/// - respecte `no_damage_ticks` (i-frames)
/// - calcule knockback si attaquant fourni
/// - décrémente HEALTH, détecte mort (→ `EntityDeathEvent`)
///
/// `attacker_position` : position XZ de l'attaquant pour calculer le knockback.
pub fn attack_entity(
    events: &mut EventManager,
    target_entity_id: u64,
    target_position: [f32; 3],
    attrs: &mut AttributeMap,
    combat: &mut CombatState,
    cause: DamageCause,
    base_damage: f32,
    attacker_entity_id: Option<u64>,
    attacker_position: Option<[f32; 3]>,
    knockback_force: f32,
) -> AttackOutcome {
    // 1) Respect i-frames sauf suicide.
    let mut event = EntityDamageEvent {
        target_entity_id,
        cause,
        base_damage,
        final_damage: base_damage,
        knockback: knockback_force,
        attacker_entity_id,
        cancelled: false,
    };

    if combat.no_damage_ticks > 0 && cause != DamageCause::Suicide {
        event.cancelled = true;
    }

    // 2) `applyDamageModifiers` : si un coup précédent était plus fort et qu'on
    //    est encore dans la fenêtre d'invincibilité, on annule ou on ne prend
    //    que le diff.
    if let Some(prev_base) = combat.last_damage_cause_base {
        if combat.attack_time > 0 {
            if prev_base >= event.base_damage {
                event.cancelled = true;
            } else {
                event.final_damage = event.base_damage - prev_base;
            }
        }
    }

    // 3) Armor + effect modifiers (non implémenté — placeholder).
    // TODO: effectManager + armor

    // 4) Dispatch event (plugins peuvent cancel ou muter final_damage).
    events.call(&mut event);
    if event.cancelled {
        return AttackOutcome {
            applied_damage: 0.0,
            died: false,
            knockback: None,
            event_cancelled: true,
        };
    }

    // 5) Applique le dégât.
    let hp_before = attrs.must_get(ids::HEALTH).current_value;
    let hp_after = (hp_before - event.final_damage).max(0.0);
    attrs.must_get_mut(ids::HEALTH).set_value(hp_after, true);
    combat.attack_time = DEFAULT_ATTACK_COOLDOWN;
    combat.last_damage_cause_base = Some(event.base_damage);

    // 6) Knockback (PMMP `Living::knockBack(x, z, force, verticalLimit)`).
    let knockback = if let Some(attacker_pos) = attacker_position {
        let dx = target_position[0] - attacker_pos[0];
        let dz = target_position[2] - attacker_pos[2];
        let f = (dx * dx + dz * dz).sqrt();
        if f > 0.0 {
            let resistance = attrs
                .get(ids::KNOCKBACK_RESISTANCE)
                .map(|a| a.current_value)
                .unwrap_or(0.0);
            let effective_force = event.knockback * (1.0 - resistance);
            let nx = dx / f;
            let nz = dz / f;
            let ky = DEFAULT_KNOCKBACK_VERTICAL_LIMIT;
            Some((nx * effective_force, ky.min(0.4), nz * effective_force))
        } else {
            None
        }
    } else {
        None
    };

    // 7) Mort ?
    let died = hp_after <= 0.0;
    if died {
        let mut death = EntityDeathEvent {
            entity_id: target_entity_id,
            drops: Vec::new(),
            xp_drop: 0,
        };
        events.call(&mut death);
    }

    AttackOutcome {
        applied_damage: event.final_damage,
        died,
        knockback,
        event_cancelled: false,
    }
}

/// Port simplifié de `Living::heal()`. Applique un heal, fire
/// `EntityRegainHealthEvent` (non implémenté ici mais placeholder).
pub fn heal_entity(attrs: &mut AttributeMap, amount: f32) -> f32 {
    let hp = attrs.must_get(ids::HEALTH).current_value;
    let max = attrs.must_get(ids::HEALTH).max_value;
    let new_hp = (hp + amount).min(max);
    attrs.must_get_mut(ids::HEALTH).set_value(new_hp, true);
    new_hp - hp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_reduces_health() {
        let mut events = EventManager::new();
        let mut attrs = AttributeMap::default_for_player();
        let mut combat = CombatState::new();
        combat.no_damage_ticks = 0; // spawn i-frames off pour ce test
        let outcome = attack_entity(
            &mut events,
            1,
            [0.0, 64.0, 0.0],
            &mut attrs,
            &mut combat,
            DamageCause::EntityAttack,
            5.0,
            Some(2),
            Some([1.0, 64.0, 0.0]),
            DEFAULT_KNOCKBACK_FORCE,
        );
        assert!(!outcome.event_cancelled);
        assert_eq!(outcome.applied_damage, 5.0);
        assert_eq!(attrs.must_get(ids::HEALTH).current_value, 15.0);
        assert!(outcome.knockback.is_some());
    }

    #[test]
    fn invincibility_blocks_damage() {
        let mut events = EventManager::new();
        let mut attrs = AttributeMap::default_for_player();
        let mut combat = CombatState::new();
        // combat.no_damage_ticks = 20 (par défaut dans new())
        let outcome = attack_entity(
            &mut events,
            1,
            [0.0, 64.0, 0.0],
            &mut attrs,
            &mut combat,
            DamageCause::EntityAttack,
            5.0,
            None,
            None,
            0.0,
        );
        assert!(outcome.event_cancelled);
        assert_eq!(attrs.must_get(ids::HEALTH).current_value, 20.0);
    }

    #[test]
    fn fatal_damage_fires_death_event() {
        let mut events = EventManager::new();
        let fired = std::sync::Arc::new(std::sync::Mutex::new(false));
        {
            let f = fired.clone();
            events.on::<EntityDeathEvent, _>(move |_| {
                *f.lock().unwrap() = true;
            });
        }
        let mut attrs = AttributeMap::default_for_player();
        attrs.must_get_mut(ids::HEALTH).set_value(2.0, true);
        let mut combat = CombatState::new();
        combat.no_damage_ticks = 0;
        let outcome = attack_entity(
            &mut events,
            1,
            [0.0, 64.0, 0.0],
            &mut attrs,
            &mut combat,
            DamageCause::EntityAttack,
            10.0,
            None,
            None,
            0.0,
        );
        assert!(outcome.died);
        assert_eq!(attrs.must_get(ids::HEALTH).current_value, 0.0);
        assert!(*fired.lock().unwrap());
    }

    #[test]
    fn heal_respects_max_health() {
        let mut attrs = AttributeMap::default_for_player();
        attrs.must_get_mut(ids::HEALTH).set_value(15.0, true);
        let healed = heal_entity(&mut attrs, 10.0);
        assert_eq!(healed, 5.0);
        assert_eq!(attrs.must_get(ids::HEALTH).current_value, 20.0);
    }
}
