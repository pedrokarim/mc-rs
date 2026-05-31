//! Assemblage du [`BehaviorGroup`] par espèce — équivalent des définitions
//! d'entités d'Allay qui listent sensors + behaviors + controllers.

use super::behavior::{AlwaysEvaluator, Behavior, ClosureEvaluator, FnEvaluator};
use super::controller::{FlyController, LookController, WalkController};
use super::executor::{
    BowAttackExecutor, CreeperSwellExecutor, FlatRandomRoamExecutor, FlyRoamExecutor,
    FlyShootExecutor, MeleeAttackExecutor, PanicFleeExecutor, TemptExecutor,
};
use super::sensor::NearestPlayerSensor;
use super::{BehaviorGroup, Controller, Sensor};
use crate::mob_entities::MobKind;

/// Priorités (plus grand = plus prioritaire).
const PRIO_COMBAT: i32 = 4;
const PRIO_PANIC: i32 = 4;
const PRIO_TEMPT: i32 = 3;
const PRIO_ROAM: i32 = 1;
/// Portée à laquelle un mob passif remarque la nourriture tenue (blocs).
const TEMPT_RANGE: f64 = 10.0;
/// Cooldown d'attaque mêlée (ticks à 20 TPS → ~1 s, comme vanilla).
const MELEE_COOLDOWN: u32 = 20;
/// Rayon d'errance (blocs).
const ROAM_RANGE: i32 = 8;
/// Rayon d'errance des volants (blocs).
const FLY_ROAM_RANGE: i32 = 10;
/// Boules de feu : vitesse (blocs/tick) et cooldown de tir (ticks).
const FIREBALL_SPEED: f32 = 0.8;
const FIREBALL_COOLDOWN: u32 = 40;
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

/// Contrôleurs standard (marche + regard cible/route) communs à tous les profils.
fn standard_controllers() -> Vec<Box<dyn Controller>> {
    vec![
        Box::new(WalkController::new()),
        Box::new(LookController::new(true, true)),
    ]
}

/// Behavior de combat (lançable si une cible est en mémoire).
fn combat_behavior(executor: Box<dyn super::behavior::Executor>) -> Behavior {
    Behavior::new(
        Box::new(FnEvaluator(|m, _, _| m.nearest_player.is_some())),
        executor,
        PRIO_COMBAT,
        1,
    )
}

/// Construit le groupe de behaviors adapté à l'espèce, dispatché par profil d'IA.
pub fn build_behavior_group(kind: MobKind) -> BehaviorGroup {
    use crate::mob_entities::AiProfile;
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

    match kind.ai_profile() {
        AiProfile::Melee => {
            let combat = combat_behavior(Box::new(MeleeAttackExecutor::new(
                speed,
                kind.sight_range(),
                kind.attack_range(),
                MELEE_COOLDOWN,
            )));
            BehaviorGroup::new(vec![], vec![combat, roam()], sensors, standard_controllers(), true)
        }
        AiProfile::Bow => {
            let combat = combat_behavior(Box::new(BowAttackExecutor::new(
                speed,
                kind.sight_range(),
                BOW_MIN_RANGE,
                BOW_SHOOT_RANGE,
                BOW_ARROW_SPEED,
                BOW_ARROW_DAMAGE,
                BOW_COOLDOWN,
            )));
            BehaviorGroup::new(vec![], vec![combat, roam()], sensors, standard_controllers(), true)
        }
        AiProfile::CreeperSwell => {
            let combat = combat_behavior(Box::new(CreeperSwellExecutor::new(
                speed,
                kind.sight_range(),
                CREEPER_IGNITE_RANGE,
                CREEPER_FUSE_TICKS,
            )));
            BehaviorGroup::new(vec![], vec![combat, roam()], sensors, standard_controllers(), true)
        }
        AiProfile::Neutral => {
            // Erre uniquement, regarde le joueur proche (n'attaque pas en v1).
            BehaviorGroup::new(vec![], vec![roam()], sensors, standard_controllers(), true)
        }
        AiProfile::Flying => {
            // Vol 3D : errance volante + FlyController (pas de route au sol).
            let normal = vec![Behavior::new(
                Box::new(AlwaysEvaluator),
                Box::new(FlyRoamExecutor::new(speed, FLY_ROAM_RANGE)),
                PRIO_ROAM,
                1,
            )];
            let controllers: Vec<Box<dyn Controller>> = vec![Box::new(FlyController::new())];
            BehaviorGroup::new(vec![], normal, sensors, controllers, false)
        }
        AiProfile::FlyingShooter => {
            // Vol + tir de boules de feu (ghast = grosse portée, blaze = courte).
            let (fireball, dmg, range) = if matches!(kind, MobKind::Ghast) {
                ("minecraft:fireball", 6.0, 32.0)
            } else {
                ("minecraft:small_fireball", 5.0, 14.0)
            };
            let combat = combat_behavior(Box::new(FlyShootExecutor::new(
                speed,
                kind.sight_range().max(range + 4.0),
                range,
                FIREBALL_SPEED,
                dmg,
                FIREBALL_COOLDOWN,
                fireball,
            )));
            let normal = vec![
                combat,
                Behavior::new(
                    Box::new(AlwaysEvaluator),
                    Box::new(FlyRoamExecutor::new(speed, FLY_ROAM_RANGE)),
                    PRIO_ROAM,
                    1,
                ),
            ];
            let controllers: Vec<Box<dyn Controller>> = vec![Box::new(FlyController::new())];
            BehaviorGroup::new(vec![], normal, sensors, controllers, false)
        }
        AiProfile::Passive => {
            // Fuit si blessé (prio haute), suit la nourriture (tempt), sinon erre.
            let mut normal = vec![Behavior::new(
                Box::new(FnEvaluator(PanicFleeExecutor::should_flee)),
                Box::new(PanicFleeExecutor::new(speed * 1.25, 8.0)),
                PRIO_PANIC,
                1,
            )];
            let food_ids: Vec<i32> = kind
                .tempting_items()
                .iter()
                .filter_map(|name| crate::item_registry::network_id(name))
                .collect();
            if !food_ids.is_empty() {
                let range_sq = TEMPT_RANGE * TEMPT_RANGE;
                let food_eval = food_ids.clone();
                normal.push(Behavior::new(
                    Box::new(ClosureEvaluator(Box::new(move |_m, base, players| {
                        players.iter().any(|p| {
                            if !p.alive || !food_eval.contains(&p.held_item) {
                                return false;
                            }
                            let dx = (p.position[0] - base.position[0]) as f64;
                            let dy = (p.position[1] - base.position[1]) as f64;
                            let dz = (p.position[2] - base.position[2]) as f64;
                            dx * dx + dy * dy + dz * dz <= range_sq
                        })
                    }))),
                    Box::new(TemptExecutor::new(speed, food_ids, TEMPT_RANGE)),
                    PRIO_TEMPT,
                    1,
                ));
            }
            normal.push(roam());
            BehaviorGroup::new(vec![], normal, sensors, standard_controllers(), true)
        }
    }
}
