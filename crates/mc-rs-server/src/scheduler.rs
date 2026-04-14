//! Scheduler — port conceptuel de `.reference/PocketMine-MP/src/scheduler/*`.
//!
//! Permet de programmer des tâches qui s'exécutent après un délai, une seule
//! fois (`after`) ou répétées (`repeat`). Les tâches sont identifiées par un
//! `TaskId` qui peut être utilisé pour les annuler.
//!
//! L'exécution a lieu à chaque game-tick (20 TPS) via `Scheduler::tick()`.

use std::collections::BinaryHeap;

pub type TaskId = u64;

/// Une tâche Rust-native. Le callback est un `Box<dyn FnMut()>`.
/// Pour les callbacks Lua, voir `plugin.rs` (bindings séparés).
pub struct ScheduledTask {
    pub id: TaskId,
    /// Tick d'échéance absolu (server tick counter).
    pub fire_at_tick: u64,
    /// Si `Some(interval)`, la tâche est re-programmée après exécution.
    pub repeat_interval: Option<u64>,
    pub callback: Box<dyn FnMut() + Send>,
}

impl std::fmt::Debug for ScheduledTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScheduledTask")
            .field("id", &self.id)
            .field("fire_at_tick", &self.fire_at_tick)
            .field("repeat_interval", &self.repeat_interval)
            .finish()
    }
}

/// Comparateur pour BinaryHeap : min-heap par fire_at_tick.
impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.fire_at_tick == other.fire_at_tick
    }
}
impl Eq for ScheduledTask {}
impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap est max-heap, inverser pour min-heap.
        other.fire_at_tick.cmp(&self.fire_at_tick)
    }
}
impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Scheduler simple basé sur un min-heap de tâches.
pub struct Scheduler {
    pub current_tick: u64,
    tasks: BinaryHeap<ScheduledTask>,
    cancelled: std::collections::HashSet<TaskId>,
    next_id: TaskId,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            current_tick: 0,
            tasks: BinaryHeap::new(),
            cancelled: std::collections::HashSet::new(),
            next_id: 1,
        }
    }

    fn new_id(&mut self) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Programme une tâche pour s'exécuter dans `delay_ticks` ticks.
    /// PMMP `Scheduler::scheduleDelayedTask(task, delayTicks)`.
    pub fn after<F>(&mut self, delay_ticks: u64, callback: F) -> TaskId
    where
        F: FnMut() + Send + 'static,
    {
        let id = self.new_id();
        self.tasks.push(ScheduledTask {
            id,
            fire_at_tick: self.current_tick + delay_ticks,
            repeat_interval: None,
            callback: Box::new(callback),
        });
        id
    }

    /// Programme une tâche répétée chaque `interval_ticks` ticks.
    /// PMMP `Scheduler::scheduleRepeatingTask(task, intervalTicks)`.
    pub fn repeat<F>(&mut self, interval_ticks: u64, callback: F) -> TaskId
    where
        F: FnMut() + Send + 'static,
    {
        assert!(interval_ticks > 0, "Scheduler::repeat interval must be > 0");
        let id = self.new_id();
        self.tasks.push(ScheduledTask {
            id,
            fire_at_tick: self.current_tick + interval_ticks,
            repeat_interval: Some(interval_ticks),
            callback: Box::new(callback),
        });
        id
    }

    /// Programme une tâche à exécuter dans `delay_ticks` ticks, puis répétée
    /// chaque `interval_ticks` ticks.
    pub fn delayed_repeat<F>(
        &mut self,
        delay_ticks: u64,
        interval_ticks: u64,
        callback: F,
    ) -> TaskId
    where
        F: FnMut() + Send + 'static,
    {
        assert!(interval_ticks > 0);
        let id = self.new_id();
        self.tasks.push(ScheduledTask {
            id,
            fire_at_tick: self.current_tick + delay_ticks,
            repeat_interval: Some(interval_ticks),
            callback: Box::new(callback),
        });
        id
    }

    /// Annule une tâche (pas enlevée du heap immédiatement — skippée à tick).
    pub fn cancel(&mut self, id: TaskId) {
        self.cancelled.insert(id);
    }

    /// Avance d'un tick. Exécute toutes les tâches dont l'échéance est
    /// passée. Les tâches répétées sont re-pushées avec la prochaine échéance.
    pub fn tick(&mut self) {
        self.current_tick = self.current_tick.wrapping_add(1);
        let mut to_requeue: Vec<ScheduledTask> = Vec::new();
        while let Some(peek) = self.tasks.peek() {
            if peek.fire_at_tick > self.current_tick {
                break;
            }
            let mut task = self.tasks.pop().unwrap();
            if self.cancelled.contains(&task.id) {
                self.cancelled.remove(&task.id);
                continue;
            }
            (task.callback)();
            if let Some(interval) = task.repeat_interval {
                task.fire_at_tick = self.current_tick + interval;
                to_requeue.push(task);
            }
        }
        for t in to_requeue {
            self.tasks.push(t);
        }
    }

    /// Nombre de tâches en attente.
    pub fn pending(&self) -> usize {
        self.tasks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn after_fires_once() {
        let mut sched = Scheduler::new();
        let count = Arc::new(Mutex::new(0));
        {
            let c = count.clone();
            sched.after(5, move || *c.lock().unwrap() += 1);
        }
        for _ in 0..10 {
            sched.tick();
        }
        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[test]
    fn repeat_fires_periodically() {
        let mut sched = Scheduler::new();
        let count = Arc::new(Mutex::new(0));
        {
            let c = count.clone();
            sched.repeat(3, move || *c.lock().unwrap() += 1);
        }
        // Après 10 ticks, devrait avoir tiré aux ticks 3, 6, 9 → 3 fois.
        for _ in 0..10 {
            sched.tick();
        }
        assert_eq!(*count.lock().unwrap(), 3);
    }

    #[test]
    fn cancel_prevents_execution() {
        let mut sched = Scheduler::new();
        let count = Arc::new(Mutex::new(0));
        let id = {
            let c = count.clone();
            sched.after(5, move || *c.lock().unwrap() += 1)
        };
        sched.cancel(id);
        for _ in 0..10 {
            sched.tick();
        }
        assert_eq!(*count.lock().unwrap(), 0);
    }

    #[test]
    fn delayed_repeat() {
        let mut sched = Scheduler::new();
        let count = Arc::new(Mutex::new(0));
        {
            let c = count.clone();
            sched.delayed_repeat(2, 3, move || *c.lock().unwrap() += 1);
        }
        // Tick 2 : 1 fois. Tick 5 : 2. Tick 8 : 3. Tick 10 : 3.
        for _ in 0..10 {
            sched.tick();
        }
        assert_eq!(*count.lock().unwrap(), 3);
    }
}
