//! BehaviorList ECS component — holds a mob's AI behaviors.

use bevy_ecs::prelude::*;

use super::behavior::{Behavior, BehaviorType};

/// Holds the list of behaviors and tracks which are currently active.
#[derive(Component)]
pub struct BehaviorList {
    pub behaviors: Vec<Box<dyn Behavior>>,
    /// Index of the currently active movement behavior, or None.
    pub active_movement: Option<usize>,
    /// Index of the currently active target selector, or None.
    pub active_target_selector: Option<usize>,
    /// Indices of currently active passive behaviors.
    pub active_passives: Vec<usize>,
}

impl BehaviorList {
    /// Create a new BehaviorList from a list of behaviors.
    pub fn new(behaviors: Vec<Box<dyn Behavior>>) -> Self {
        Self {
            behaviors,
            active_movement: None,
            active_target_selector: None,
            active_passives: Vec::new(),
        }
    }

    /// Count behaviors by type.
    pub fn count_by_type(&self, bt: BehaviorType) -> usize {
        self.behaviors
            .iter()
            .filter(|b| b.behavior_type() == bt)
            .count()
    }
}

impl std::fmt::Debug for BehaviorList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BehaviorList")
            .field("behavior_count", &self.behaviors.len())
            .field("active_movement", &self.active_movement)
            .field("active_target_selector", &self.active_target_selector)
            .field("active_passives", &self.active_passives)
            .finish()
    }
}
