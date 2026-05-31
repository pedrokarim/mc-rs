//! Assemblage du [`BehaviorGroup`] par espèce — équivalent des définitions
//! d'entités d'Allay qui listent sensors + behaviors + controllers.

use super::behavior::{AlwaysEvaluator, Behavior, FnEvaluator};
use super::controller::{LookController, WalkController};
use super::executor::{FlatRandomRoamExecutor, MeleeAttackExecutor, PanicFleeExecutor};
use super::sensor::NearestPlayerSensor;
use super::{BehaviorGroup, Controller, Sensor};
use crate::mob_entities::MobKind;

/// Priorités (plus grand = plus prioritaire).
const PRIO_COMBAT: i32 = 4;
const PRIO_PANIC: i32 = 4;
const PRIO_ROAM: i32 = 1;
/// Cooldown d'attaque mêlée (ticks à 20 TPS → ~1 s, comme vanilla).
const MELEE_COOLDOWN: u32 = 20;
/// Rayon d'errance (blocs).
const ROAM_RANGE: i32 = 8;

/// Construit le groupe de behaviors adapté à l'espèce.
pub fn build_behavior_group(kind: MobKind) -> BehaviorGroup {
    let speed = kind.movement_speed();
    let sensors: Vec<Box<dyn Sensor>> =
        vec![Box::new(NearestPlayerSensor::with_range(kind.sight_range()))];

    let roam = || -> Behavior {
        Behavior::new(
            Box::new(AlwaysEvaluator),
            Box::new(FlatRandomRoamExecutor::new(speed, ROAM_RANGE)),
            PRIO_ROAM,
            1,
        )
    };

    if kind.is_hostile() {
        // Hostile : traque + frappe (prio haute), sinon erre. Regarde sa cible.
        let normal = vec![
            Behavior::new(
                Box::new(FnEvaluator(|m, _, _| m.nearest_player.is_some())),
                Box::new(MeleeAttackExecutor::new(
                    speed,
                    kind.sight_range(),
                    kind.attack_range(),
                    MELEE_COOLDOWN,
                )),
                PRIO_COMBAT,
                1,
            ),
            roam(),
        ];
        let controllers: Vec<Box<dyn Controller>> = vec![
            Box::new(WalkController::new()),
            Box::new(LookController::new(true, true)),
        ];
        BehaviorGroup::new(vec![], normal, sensors, controllers, true)
    } else {
        // Passif : fuit s'il est blessé près d'un joueur (prio haute), sinon erre.
        let normal = vec![
            Behavior::new(
                Box::new(FnEvaluator(PanicFleeExecutor::should_flee)),
                Box::new(PanicFleeExecutor::new(speed * 1.25, 8.0)),
                PRIO_PANIC,
                1,
            ),
            roam(),
        ];
        let controllers: Vec<Box<dyn Controller>> = vec![
            Box::new(WalkController::new()),
            Box::new(LookController::new(false, true)),
        ];
        BehaviorGroup::new(vec![], normal, sensors, controllers, true)
    }
}
