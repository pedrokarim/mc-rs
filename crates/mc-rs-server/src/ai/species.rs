//! Assemblage du [`BehaviorGroup`] par espèce — équivalent des définitions
//! d'entités d'Allay qui listent sensors + behaviors + controllers.

use super::behavior::{AlwaysEvaluator, Behavior, FnEvaluator};
use super::controller::{LookController, WalkController};
use super::executor::{
    BowAttackExecutor, CreeperSwellExecutor, FlatRandomRoamExecutor, MeleeAttackExecutor,
    PanicFleeExecutor,
};
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
/// Creeper : portée d'amorçage et durée de fuse (ticks 20 TPS → 1.5 s, vanilla).
const CREEPER_IGNITE_RANGE: f64 = 3.0;
const CREEPER_FUSE_TICKS: u32 = 30;
/// Skeleton (arc) : on recule sous `MIN`, on s'approche au-delà de `SHOOT`,
/// on tire entre les deux. Vitesse de flèche et dégâts/cooldown.
const BOW_MIN_RANGE: f64 = 4.0;
const BOW_SHOOT_RANGE: f64 = 12.0;
const BOW_ARROW_SPEED: f32 = 1.4;
const BOW_ARROW_DAMAGE: f32 = 3.0;
const BOW_COOLDOWN: u32 = 30;

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
        // Comportement de combat selon l'espèce : le creeper s'amorce et explose,
        // les autres hostiles frappent en mêlée. (Skeleton-arc = follow-up.)
        let combat = if matches!(kind, MobKind::Creeper) {
            Behavior::new(
                Box::new(FnEvaluator(|m, _, _| m.nearest_player.is_some())),
                Box::new(CreeperSwellExecutor::new(
                    speed,
                    kind.sight_range(),
                    CREEPER_IGNITE_RANGE,
                    CREEPER_FUSE_TICKS,
                )),
                PRIO_COMBAT,
                1,
            )
        } else if matches!(kind, MobKind::Skeleton) {
            Behavior::new(
                Box::new(FnEvaluator(|m, _, _| m.nearest_player.is_some())),
                Box::new(BowAttackExecutor::new(
                    speed,
                    kind.sight_range(),
                    BOW_MIN_RANGE,
                    BOW_SHOOT_RANGE,
                    BOW_ARROW_SPEED,
                    BOW_ARROW_DAMAGE,
                    BOW_COOLDOWN,
                )),
                PRIO_COMBAT,
                1,
            )
        } else {
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
            )
        };
        // Hostile : combat (prio haute), sinon erre. Regarde sa cible.
        let normal = vec![combat, roam()];
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
