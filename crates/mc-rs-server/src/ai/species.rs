//! Assemblage du [`BehaviorGroup`] par espèce — équivalent des définitions
//! d'entités d'Allay qui listent sensors + behaviors + controllers.

use super::behavior::{AlwaysEvaluator, Behavior, ClosureEvaluator, FnEvaluator};
use super::controller::{FlyController, LookController, WalkController};
use super::executor::{
    BowAttackExecutor, CreakingExecutor, CreeperSwellExecutor, DragonExecutor,
    FlatRandomRoamExecutor, FlyAttackExecutor, FlyRoamExecutor, FlyShootExecutor,
    GuardianLaserExecutor, MeleeAttackExecutor, PanicFleeExecutor, TemptExecutor,
};
use super::sensor::NearestPlayerSensor;
use super::{BehaviorGroup, Controller, Sensor};
use crate::mob_entities::MobKind;

/// Priorités (plus grand = plus prioritaire). NB : bedrock-samples utilise la
/// convention INVERSE (petit numéro = prioritaire), on garde celle d'Allay
/// (grand = prioritaire) mais l'ORDRE sémantique est identique au vanilla :
/// panic > breed > tempt > follow_parent > random_stroll.
const PRIO_COMBAT: i32 = 4;
const PRIO_PANIC: i32 = 4;
const PRIO_TEMPT: i32 = 3;
const PRIO_ROAM: i32 = 1;
/// Portée à laquelle un mob passif remarque la nourriture tenue (blocs).
const TEMPT_RANGE: f64 = 10.0;
/// Ticks serveur (100 TPS) par game-tick vanilla (20 TPS). Le tick des mobs
/// tourne au tick serveur → tous les cooldowns/durées ci-dessous sont exprimés
/// en game-ticks vanilla × ce facteur.
const SERVER_TICKS_PER_GAME_TICK: u32 = 5;
/// Cooldown d'attaque mêlée (~1 s : 20 game-ticks → ticks serveur, comme vanilla).
const MELEE_COOLDOWN: u32 = 20 * SERVER_TICKS_PER_GAME_TICK;
/// Rayon d'errance (blocs) — vanilla `random_stroll.xz_dist` défaut 10.
const ROAM_RANGE: i32 = 10;

// --- Vitesses : multiplicateurs par comportement, recalés sur la spec vanilla
// (bedrock-samples `behavior_pack/entities/*.json`, champ `speed_multiplier`).
// La vitesse effective = attribut `minecraft:movement` du mob (kind.movement_speed)
// × SPEED_SCALE × multiplicateur du comportement. SPEED_SCALE est un bouton de
// calibrage global de notre moteur (l'attribut vanilla n'est pas une vitesse/tick
// exacte) : 1.0 = attribut tel quel ; à baisser en jeu si tout est trop rapide.
/// Calibrage global attribut→vitesse/tick.
const SPEED_SCALE: f32 = 1.0;
/// `minecraft:behavior.random_stroll` (défaut 1.0 ; certains animaux 0.8).
const STROLL_MULT: f32 = 1.0;
/// `minecraft:behavior.panic` (1.25 ; poule 1.5).
const PANIC_MULT: f32 = 1.25;
/// `minecraft:behavior.tempt` (1.25 ; poule 1.0).
const TEMPT_MULT: f32 = 1.25;
/// `minecraft:behavior.melee_box_attack` / `melee_attack` (1.0 ; creeper 1.25).
const MELEE_MULT: f32 = 1.0;

/// Vitesse effective d'un comportement = attribut × calibrage × multiplicateur.
fn beh_speed(base: f32, multiplier: f32) -> f32 {
    base * SPEED_SCALE * multiplier
}
/// Rayon d'errance des volants (blocs).
const FLY_ROAM_RANGE: i32 = 10;
/// Boules de feu : vitesse (blocs/tick serveur = 0.8/game-tick × 0.2) et cooldown (game-ticks → serveur).
const FIREBALL_SPEED: f32 = 0.8 * 0.2;
const FIREBALL_COOLDOWN: u32 = 40 * SERVER_TICKS_PER_GAME_TICK;
/// Guardian : distance de tir et durée de charge du rayon (game-ticks → serveur).
const GUARDIAN_HOLD_RANGE: f64 = 10.0;
const GUARDIAN_CHARGE_TICKS: u32 = 60 * SERVER_TICKS_PER_GAME_TICK;
/// Creeper : portée d'amorçage et durée de fuse (~1.5 s : 30 game-ticks → serveur, vanilla).
const CREEPER_IGNITE_RANGE: f64 = 3.0;
const CREEPER_FUSE_TICKS: u32 = 30 * SERVER_TICKS_PER_GAME_TICK;
/// Skeleton (arc) : on recule sous `MIN`, on s'approche au-delà de `SHOOT`,
/// on tire entre les deux. Vitesse de flèche et dégâts/cooldown.
const BOW_MIN_RANGE: f64 = 4.0;
const BOW_SHOOT_RANGE: f64 = 12.0;
/// Vitesse de la flèche : 1.4/game-tick × 0.2 = blocs/tick serveur (100 TPS).
const BOW_ARROW_SPEED: f32 = 1.4 * 0.2;
const BOW_ARROW_DAMAGE: f32 = 3.0;
const BOW_COOLDOWN: u32 = 30 * SERVER_TICKS_PER_GAME_TICK;

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
    let sensors: Vec<Box<dyn Sensor>> = vec![Box::new(NearestPlayerSensor::with_range(
        kind.sight_range(),
    ))];
    let roam = || -> Behavior {
        Behavior::new(
            Box::new(AlwaysEvaluator),
            Box::new(FlatRandomRoamExecutor::new(
                beh_speed(speed, STROLL_MULT),
                ROAM_RANGE,
            )),
            PRIO_ROAM,
            1,
        )
    };

    match kind.ai_profile() {
        AiProfile::Melee => {
            let combat = combat_behavior(Box::new(MeleeAttackExecutor::new(
                beh_speed(speed, MELEE_MULT),
                kind.sight_range(),
                kind.attack_range(),
                MELEE_COOLDOWN,
            )));
            BehaviorGroup::new(
                vec![],
                vec![combat, roam()],
                sensors,
                standard_controllers(),
                true,
            )
        }
        AiProfile::Bow => {
            // Certains lancent un projectile (potion, wind charge, bullet) au lieu
            // d'une flèche ; ceux-là frappent un peu plus fort, moins vite.
            let combat = match kind.thrown_projectile() {
                Some(proj) => {
                    // La wind charge du breeze repousse fort mais blesse peu.
                    let dmg = if matches!(kind, MobKind::Breeze) {
                        1.0
                    } else {
                        6.0
                    };
                    combat_behavior(Box::new(BowAttackExecutor::with_projectile(
                        speed,
                        kind.sight_range(),
                        BOW_MIN_RANGE,
                        BOW_SHOOT_RANGE,
                        BOW_ARROW_SPEED,
                        dmg,
                        BOW_COOLDOWN + 20 * SERVER_TICKS_PER_GAME_TICK,
                        Some(proj),
                    )))
                }
                None => combat_behavior(Box::new(BowAttackExecutor::new(
                    speed,
                    kind.sight_range(),
                    BOW_MIN_RANGE,
                    BOW_SHOOT_RANGE,
                    BOW_ARROW_SPEED,
                    BOW_ARROW_DAMAGE,
                    BOW_COOLDOWN,
                ))),
            };
            BehaviorGroup::new(
                vec![],
                vec![combat, roam()],
                sensors,
                standard_controllers(),
                true,
            )
        }
        AiProfile::CreeperSwell => {
            let combat = combat_behavior(Box::new(CreeperSwellExecutor::new(
                speed,
                kind.sight_range(),
                CREEPER_IGNITE_RANGE,
                CREEPER_FUSE_TICKS,
            )));
            BehaviorGroup::new(
                vec![],
                vec![combat, roam()],
                sensors,
                standard_controllers(),
                true,
            )
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
            let (fireball, dmg, range) = match kind {
                MobKind::Ghast => ("minecraft:fireball", 6.0, 32.0),
                MobKind::Wither => ("minecraft:wither_skull_dangerous", 8.0, 24.0),
                _ => ("minecraft:small_fireball", 5.0, 14.0),
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
                Box::new(PanicFleeExecutor::new(beh_speed(speed, PANIC_MULT), 8.0)),
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
                    Box::new(TemptExecutor::new(
                        beh_speed(speed, TEMPT_MULT),
                        food_ids,
                        TEMPT_RANGE,
                    )),
                    PRIO_TEMPT,
                    1,
                ));
            }
            normal.push(roam());
            BehaviorGroup::new(vec![], normal, sensors, standard_controllers(), true)
        }
        AiProfile::FlyingMelee => {
            // Vol + fonce sur le joueur (frappe au contact), sinon erre en vol.
            let combat = combat_behavior(Box::new(FlyAttackExecutor::new(
                speed,
                kind.sight_range(),
                kind.attack_range(),
                kind.attack_damage(),
                MELEE_COOLDOWN,
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
        AiProfile::Creaking => {
            // Figé quand regardé, sinon fonce + frappe.
            let combat = combat_behavior(Box::new(CreakingExecutor::new(
                speed,
                kind.sight_range(),
                kind.attack_range(),
                kind.attack_damage(),
                MELEE_COOLDOWN,
            )));
            BehaviorGroup::new(
                vec![],
                vec![combat, roam()],
                sensors,
                standard_controllers(),
                true,
            )
        }
        AiProfile::Dragon => {
            // Phases cercle ↔ strafe (l'executor pilote tout) + vol.
            let combat = Behavior::new(
                Box::new(AlwaysEvaluator),
                Box::new(DragonExecutor::new(speed)),
                PRIO_COMBAT,
                1,
            );
            let controllers: Vec<Box<dyn Controller>> = vec![Box::new(FlyController::new())];
            BehaviorGroup::new(vec![], vec![combat], sensors, controllers, false)
        }
        AiProfile::GuardianLaser => {
            // Aquatique : se tient à distance, charge un rayon, frappe. Nage 3D.
            let combat = combat_behavior(Box::new(GuardianLaserExecutor::new(
                speed,
                kind.sight_range(),
                GUARDIAN_HOLD_RANGE,
                kind.attack_damage(),
                GUARDIAN_CHARGE_TICKS,
            )));
            let controllers: Vec<Box<dyn Controller>> = vec![Box::new(FlyController::new())];
            BehaviorGroup::new(vec![], vec![combat], sensors, controllers, false)
        }
    }
}
