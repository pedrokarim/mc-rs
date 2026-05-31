//! Framework de behaviors — port du modèle `Behavior`/`BehaviorEvaluator`/
//! `BehaviorExecutor` d'Allay (`api/.../entity/ai/behavior/`).
//!
//! Un [`Behavior`] couple un [`Evaluator`] (« peut-il démarrer ? », lecture
//! seule) et un [`Executor`] (« exécute-le », état interne mutable autorisé),
//! avec une **priorité** (plus grand = plus prioritaire, comme Allay) et une
//! **période d'évaluation** en ticks.

use super::memory::Memory;
use super::{ExecCtx, PlayerSnapshot};
use crate::entity::EntityBase;

/// État courant d'un behavior. Port de `BehaviorState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorState {
    Active,
    Stop,
}

/// Décide si un behavior est lançable. Lecture seule sur la mémoire / les
/// joueurs (port `BehaviorEvaluator`).
pub trait Evaluator: Send {
    fn evaluate(&self, memory: &Memory, base: &EntityBase, players: &[PlayerSnapshot]) -> bool;
}

/// Exécute la logique d'un behavior (port `BehaviorExecutor`). `execute`
/// renvoie `false` lorsque le behavior est terminé (déclenche alors `on_stop`).
pub trait Executor: Send {
    /// Appelé une fois quand le behavior devient actif.
    fn on_start(&mut self, _memory: &mut Memory, _base: &mut EntityBase) {}
    /// Appelé chaque tick tant que le behavior tourne. `false` → terminé.
    fn execute(&mut self, ctx: &mut ExecCtx) -> bool;
    /// Appelé quand le behavior s'arrête de lui-même (`execute` a renvoyé `false`).
    fn on_stop(&mut self, _memory: &mut Memory, _base: &mut EntityBase) {}
    /// Appelé quand un behavior plus prioritaire interrompt celui-ci.
    fn on_interrupt(&mut self, memory: &mut Memory, base: &mut EntityBase) {
        self.on_stop(memory, base);
    }
}

/// Un behavior = evaluator + executor + priorité + période d'évaluation.
pub struct Behavior {
    pub evaluator: Box<dyn Evaluator>,
    pub executor: Box<dyn Executor>,
    /// Priorité : plus grand = plus prioritaire (convention Allay).
    pub priority: i32,
    /// Période d'évaluation en ticks (>= 1) : ré-évalué tous les `period` ticks.
    pub period: u32,
}

impl Behavior {
    pub fn new(
        evaluator: Box<dyn Evaluator>,
        executor: Box<dyn Executor>,
        priority: i32,
        period: u32,
    ) -> Self {
        Self {
            evaluator,
            executor,
            priority,
            period: period.max(1),
        }
    }
}

// ---------------------------------------------------------------------------
// Évaluateurs réutilisables
// ---------------------------------------------------------------------------

/// Toujours lançable.
pub struct AlwaysEvaluator;
impl Evaluator for AlwaysEvaluator {
    fn evaluate(&self, _m: &Memory, _b: &EntityBase, _p: &[PlayerSnapshot]) -> bool {
        true
    }
}

/// Lançable si un prédicat sur la mémoire est vrai (port
/// `MemoryCheckNotEmptyEvaluator` généralisé). Ex : cible d'attaque présente.
pub struct MemoryEvaluator {
    pub check: fn(&Memory) -> bool,
}
impl Evaluator for MemoryEvaluator {
    fn evaluate(&self, m: &Memory, _b: &EntityBase, _p: &[PlayerSnapshot]) -> bool {
        (self.check)(m)
    }
}

/// Évaluateur générique par pointeur de fonction sur le contexte complet
/// `(mémoire, entité, joueurs)` — pour les conditions qui dépendent de l'état
/// de l'entité (ex : « blessé ET joueur proche »).
pub struct FnEvaluator(pub fn(&Memory, &EntityBase, &[PlayerSnapshot]) -> bool);
impl Evaluator for FnEvaluator {
    fn evaluate(&self, m: &Memory, b: &EntityBase, p: &[PlayerSnapshot]) -> bool {
        (self.0)(m, b, p)
    }
}
