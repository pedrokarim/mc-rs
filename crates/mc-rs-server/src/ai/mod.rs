//! IA des mobs — framework générique inspiré d'**Allay**
//! (`.reference/Allay/.../entity/ai/`), seule référence valable ici car
//! PocketMine-MP n'implémente pas d'IA de mobs.
//!
//! Pipeline par tick (port de `BehaviorGroupImpl.tick()`) :
//! `sensors → eval core → eval normaux (priorité) → exec → update_route →
//! controllers`.
//!
//! Adaptations Rust :
//! - Mémoire typée par champs (voir [`memory::Memory`]) au lieu de
//!   `MemoryType<T>`.
//! - Pas d'accès direct entité→entité (borrow checker) : le tick reçoit un
//!   snapshot des joueurs ([`PlayerSnapshot`]) et le `ChunkCache` en paramètre.
//! - Les actions touchant d'autres entités (dégâts à un joueur) ne sont pas
//!   appliquées dans le tick : elles sont émises comme [`AiEffect`] et drainées
//!   par `main.rs` (équivalent des `syncedActions` d'Allay).

pub mod behavior;
pub mod memory;

use crate::entity::EntityBase;
use crate::mob_entities::MobKind;
use crate::world::chunk_cache::ChunkCache;
use behavior::Behavior;
use memory::Memory;

/// Cycle de recalcul périodique de route, en ticks (port `ROUTE_UPDATE_CYCLE`).
const ROUTE_UPDATE_CYCLE: u32 = 20;

/// Snapshot léger d'un joueur connecté, construit chaque tick par `main.rs`
/// depuis `connections`. Évite que l'IA touche directement les `Connection`.
#[derive(Debug, Clone, Copy)]
pub struct PlayerSnapshot {
    pub runtime_id: u64,
    pub position: [f32; 3],
    /// 0=survival, 1=creative, 2=adventure, 3=spectator (`Connection::gamemode`).
    pub gamemode: i32,
    pub alive: bool,
}

impl PlayerSnapshot {
    /// Cible valide pour un mob hostile : vivant + survival/adventure
    /// (port de `MeleeAttackExecutor.isTargetValid`).
    pub fn is_attackable(&self) -> bool {
        self.alive && (self.gamemode == 0 || self.gamemode == 2)
    }
}

/// Effet produit par l'IA qui doit être appliqué **hors** du tick mob (par
/// `main.rs`), car il touche d'autres entités — équivalent des `syncedActions`
/// d'Allay.
#[derive(Debug, Clone)]
pub enum AiEffect {
    /// Le mob `attacker_runtime_id` (en `attacker_position`) inflige `damage`
    /// au joueur `target_runtime_id`.
    Attack {
        attacker_runtime_id: u64,
        attacker_position: [f32; 3],
        target_runtime_id: u64,
        damage: f32,
    },
}

/// Contexte passé aux executors : état mutable du mob, monde en lecture seule
/// (joueurs), et file d'effets à émettre.
pub struct ExecCtx<'a> {
    pub base: &'a mut EntityBase,
    pub kind: MobKind,
    pub memory: &'a mut Memory,
    pub players: &'a [PlayerSnapshot],
    pub effects: &'a mut Vec<AiEffect>,
}

/// Contexte passé aux controllers : état mutable du mob + `ChunkCache` (pour la
/// collision / le step-up).
pub struct ControlCtx<'a> {
    pub base: &'a mut EntityBase,
    pub kind: MobKind,
    pub memory: &'a mut Memory,
    pub chunk_cache: &'a mut ChunkCache,
}

/// Capte l'environnement et écrit dans la mémoire (port `Sensor`).
pub trait Sensor: Send {
    fn sense(&mut self, memory: &mut Memory, base: &EntityBase, players: &[PlayerSnapshot]);
    /// Période d'échantillonnage en ticks (>= 1).
    fn period(&self) -> u32 {
        1
    }
}

/// Traduit la mémoire en mouvement réel du mob (port `Controller`).
pub trait Controller: Send {
    fn control(&mut self, ctx: &mut ControlCtx);
}

// ---------------------------------------------------------------------------
// Orchestrateur
// ---------------------------------------------------------------------------

struct SensorSlot {
    sensor: Box<dyn Sensor>,
    counter: u32,
}

struct BehaviorSlot {
    behavior: Behavior,
    counter: u32,
    running: bool,
}

impl BehaviorSlot {
    fn new(behavior: Behavior) -> Self {
        Self {
            behavior,
            counter: 0,
            running: false,
        }
    }
}

/// Groupe de behaviors d'un mob (port `BehaviorGroup`). Orchestre sensors,
/// behaviors core (sans priorité, cumulables) et normaux (sélection par
/// priorité), controllers et route.
pub struct BehaviorGroup {
    core: Vec<BehaviorSlot>,
    normal: Vec<BehaviorSlot>,
    sensors: Vec<SensorSlot>,
    controllers: Vec<Box<dyn Controller>>,
    /// Active la recherche de chemin (mobs au sol). Désactivé → pas de route.
    route_enabled: bool,
}

impl BehaviorGroup {
    /// Construit un groupe. Voir `species::build_behavior_group` pour
    /// l'assemblage par espèce.
    pub fn new(
        core: Vec<Behavior>,
        normal: Vec<Behavior>,
        sensors: Vec<Box<dyn Sensor>>,
        controllers: Vec<Box<dyn Controller>>,
        route_enabled: bool,
    ) -> Self {
        Self {
            core: core.into_iter().map(BehaviorSlot::new).collect(),
            normal: normal.into_iter().map(BehaviorSlot::new).collect(),
            sensors: sensors
                .into_iter()
                .map(|sensor| SensorSlot { sensor, counter: 0 })
                .collect(),
            controllers,
            route_enabled,
        }
    }

    /// Pipeline complet d'un tick (port `BehaviorGroupImpl.tick`).
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        memory: &mut Memory,
        base: &mut EntityBase,
        kind: MobKind,
        players: &[PlayerSnapshot],
        chunk_cache: &mut ChunkCache,
        effects: &mut Vec<AiEffect>,
    ) {
        self.collect_sensors(memory, base, players);
        self.evaluate_core(memory, base, players);
        self.evaluate_normal(memory, base, players);
        self.execute_running(base, kind, memory, players, effects);
        self.update_route(memory, base, chunk_cache);
        self.apply_controllers(base, kind, memory, chunk_cache);
    }

    fn collect_sensors(&mut self, memory: &mut Memory, base: &EntityBase, players: &[PlayerSnapshot]) {
        for slot in self.sensors.iter_mut() {
            slot.counter += 1;
            if slot.counter >= slot.sensor.period() {
                slot.counter = 0;
                slot.sensor.sense(memory, base, players);
            }
        }
    }

    /// Behaviors core : indépendants de la priorité, tout ceux qui évaluent vrai
    /// démarrent en parallèle (port `evaluateCoreBehaviors`).
    fn evaluate_core(&mut self, memory: &mut Memory, base: &mut EntityBase, players: &[PlayerSnapshot]) {
        for slot in self.core.iter_mut() {
            if slot.running {
                continue;
            }
            slot.counter += 1;
            if slot.counter < slot.behavior.period {
                continue;
            }
            slot.counter = 0;
            if slot.behavior.evaluator.evaluate(memory, base, players) {
                slot.behavior.executor.on_start(memory, base);
                slot.running = true;
            }
        }
    }

    /// Behaviors normaux : sélection par priorité (port `evaluateBehaviors`).
    /// Seuls les behaviors de plus haute priorité évalués vrai sont retenus ;
    /// un behavior actif n'est interrompu que par une priorité strictement
    /// supérieure.
    fn evaluate_normal(&mut self, memory: &mut Memory, base: &mut EntityBase, players: &[PlayerSnapshot]) {
        // Passe 1 : collecte les candidats échus & non-running de plus haute priorité.
        let mut eval_succeed: Vec<usize> = Vec::new();
        let mut highest = i32::MIN;
        for (i, slot) in self.normal.iter_mut().enumerate() {
            if slot.running {
                continue;
            }
            slot.counter += 1;
            if slot.counter < slot.behavior.period {
                continue;
            }
            slot.counter = 0;
            if slot.behavior.evaluator.evaluate(memory, base, players) {
                let p = slot.behavior.priority;
                if p > highest {
                    eval_succeed.clear();
                    highest = p;
                    eval_succeed.push(i);
                } else if p == highest {
                    eval_succeed.push(i);
                }
            }
        }
        if eval_succeed.is_empty() {
            return;
        }

        // Priorité des behaviors actuellement en cours.
        let running_priority = self
            .normal
            .iter()
            .find(|s| s.running)
            .map(|s| s.behavior.priority)
            .unwrap_or(i32::MIN);

        if highest < running_priority {
            // Candidats moins prioritaires que le courant → on ne fait rien.
            return;
        }
        if highest > running_priority {
            // Plus prioritaire → on interrompt les behaviors en cours.
            for slot in self.normal.iter_mut().filter(|s| s.running) {
                slot.behavior.executor.on_interrupt(memory, base);
                slot.running = false;
            }
        }
        // Priorité égale (ou supérieure après interruption) → on démarre.
        for &i in &eval_succeed {
            let slot = &mut self.normal[i];
            slot.behavior.executor.on_start(memory, base);
            slot.running = true;
        }
    }

    fn execute_running(
        &mut self,
        base: &mut EntityBase,
        kind: MobKind,
        memory: &mut Memory,
        players: &[PlayerSnapshot],
        effects: &mut Vec<AiEffect>,
    ) {
        for slot in self.core.iter_mut().chain(self.normal.iter_mut()) {
            if !slot.running {
                continue;
            }
            let finished = {
                let mut ctx = ExecCtx {
                    base,
                    kind,
                    memory,
                    players,
                    effects,
                };
                !slot.behavior.executor.execute(&mut ctx)
            };
            if finished {
                slot.behavior.executor.on_stop(memory, base);
                slot.running = false;
            }
        }
    }

    /// Met à jour la route et le segment de direction courant (port
    /// `updateRoute`). Le recalcul A\* effectif est branché à l'étape 3 ; ici on
    /// gère le suivi de waypoints et l'avance au point suivant.
    fn update_route(&mut self, memory: &mut Memory, base: &mut EntityBase, _chunk_cache: &mut ChunkCache) {
        if !self.route_enabled {
            return;
        }
        let Some(_move_target) = memory.move_target else {
            memory.clear_move_direction();
            return;
        };

        // Recalcul périodique forcé (route périmée par des changements de blocs).
        if memory.has_next_node() {
            memory.route_update_tick += 1;
            if memory.route_update_tick >= ROUTE_UPDATE_CYCLE {
                memory.route_update_tick = 0;
                memory.route_update_required = true;
            }
        }

        // Étape 3 : appel du RouteFinder quand `route_update_required` ou route
        // épuisée. (Le finder remplira `memory.route` + remettra `node_index`.)

        // Avance au waypoint suivant quand l'entité est assez proche (port du
        // "followThePath" : maxDist basé sur la largeur de hitbox).
        if !memory.should_update_move_dir && memory.has_move_direction() {
            if let Some(end) = memory.move_dir_end {
                let dx = end[0] - base.position[0] as f64;
                let dz = end[2] - base.position[2] as f64;
                // Distance d'avance au waypoint (port "followThePath" ; ~largeur
                // de hitbox, ramenée à une constante simple ici).
                let max_dist = 0.75_f64;
                if dx * dx + dz * dz < max_dist * max_dist {
                    memory.should_update_move_dir = true;
                }
            }
        }

        // Consomme le prochain nœud de direction si nécessaire.
        if memory.should_update_move_dir || !memory.has_move_direction() {
            if memory.has_next_node() {
                let next = memory.route[memory.node_index];
                memory.node_index += 1;
                let start = memory
                    .move_dir_end
                    .unwrap_or([base.position[0] as f64, base.position[1] as f64, base.position[2] as f64]);
                memory.move_dir_start = Some(start);
                memory.move_dir_end = Some(next);
                memory.should_update_move_dir = false;
            } else if memory.should_update_move_dir {
                memory.clear_move_direction();
                memory.should_update_move_dir = false;
            }
        }
    }

    fn apply_controllers(
        &mut self,
        base: &mut EntityBase,
        kind: MobKind,
        memory: &mut Memory,
        chunk_cache: &mut ChunkCache,
    ) {
        for controller in self.controllers.iter_mut() {
            let mut ctx = ControlCtx {
                base,
                kind,
                memory,
                chunk_cache,
            };
            controller.control(&mut ctx);
        }
    }

    /// (Tests) priorités des behaviors normaux actuellement actifs.
    #[cfg(test)]
    fn running_normal_priorities(&self) -> Vec<i32> {
        self.normal
            .iter()
            .filter(|s| s.running)
            .map(|s| s.behavior.priority)
            .collect()
    }
}

/// Composant IA attaché à un mob : le groupe de behaviors + sa mémoire.
pub struct AiComponent {
    pub group: BehaviorGroup,
    pub memory: Memory,
}

impl AiComponent {
    pub fn new(group: BehaviorGroup, default_speed: f32) -> Self {
        Self {
            group,
            memory: Memory::new(default_speed),
        }
    }

    /// Tick IA : délègue au groupe avec la mémoire du composant. `self.group` et
    /// `self.memory` sont des champs disjoints → emprunts mut simultanés OK.
    pub fn tick(
        &mut self,
        base: &mut EntityBase,
        kind: MobKind,
        players: &[PlayerSnapshot],
        chunk_cache: &mut ChunkCache,
        effects: &mut Vec<AiEffect>,
    ) {
        self.group
            .tick(&mut self.memory, base, kind, players, chunk_cache, effects);
    }
}

#[cfg(test)]
mod tests {
    use super::behavior::{AlwaysEvaluator, Behavior, Executor, MemoryEvaluator};
    use super::memory::Memory;
    use super::*;

    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// Executor de test : compte ses exécutions, ne se termine jamais.
    struct CountingExecutor {
        runs: Arc<AtomicU32>,
    }
    impl Executor for CountingExecutor {
        fn execute(&mut self, _ctx: &mut ExecCtx) -> bool {
            self.runs.fetch_add(1, Ordering::Relaxed);
            true
        }
    }

    fn dummy_base() -> EntityBase {
        EntityBase::new("minecraft:zombie", "zombie", "Zombie", [0.0, 64.0, 0.0], vec![], vec![])
    }

    fn make_group() -> (BehaviorGroup, Arc<AtomicU32>, Arc<AtomicU32>) {
        let melee_runs = Arc::new(AtomicU32::new(0));
        let roam_runs = Arc::new(AtomicU32::new(0));
        // Melee : priorité 10, lançable seulement si une cible est en mémoire.
        let melee = Behavior::new(
            Box::new(MemoryEvaluator {
                check: |m| m.nearest_player.is_some(),
            }),
            Box::new(CountingExecutor {
                runs: melee_runs.clone(),
            }),
            10,
            1,
        );
        // Roam : priorité 1, toujours lançable.
        let roam = Behavior::new(
            Box::new(AlwaysEvaluator),
            Box::new(CountingExecutor {
                runs: roam_runs.clone(),
            }),
            1,
            1,
        );
        let group = BehaviorGroup::new(vec![], vec![melee, roam], vec![], vec![], false);
        (group, melee_runs, roam_runs)
    }

    #[test]
    fn high_priority_behavior_preempts_low() {
        let (mut group, melee_runs, roam_runs) = make_group();
        let mut base = dummy_base();
        let mut memory = Memory::new(0.1);
        let mut effects = Vec::new();
        // Pas encore de cible → seul roam (priorité basse) doit tourner.
        let test_dir = std::env::temp_dir().join(format!("mc-rs-ai-prio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&test_dir);
        let mut cache = ChunkCache::new(&test_dir, 1, "normal");
        group.tick(&mut memory, &mut base, MobKind::Zombie, &[], &mut cache, &mut effects);
        assert_eq!(group.running_normal_priorities(), vec![1]);
        assert_eq!(roam_runs.load(Ordering::Relaxed), 1);
        assert_eq!(melee_runs.load(Ordering::Relaxed), 0);

        // Cible apparaît → melee (priorité 10) interrompt roam.
        memory.nearest_player = Some(42);
        group.tick(&mut memory, &mut base, MobKind::Zombie, &[], &mut cache, &mut effects);
        assert_eq!(group.running_normal_priorities(), vec![10]);
        assert_eq!(melee_runs.load(Ordering::Relaxed), 1);
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn target_validity_respects_gamemode() {
        let survival = PlayerSnapshot {
            runtime_id: 1,
            position: [0.0; 3],
            gamemode: 0,
            alive: true,
        };
        let creative = PlayerSnapshot {
            gamemode: 1,
            ..survival
        };
        let dead = PlayerSnapshot {
            alive: false,
            ..survival
        };
        assert!(survival.is_attackable());
        assert!(!creative.is_attackable());
        assert!(!dead.is_attackable());
    }
}
