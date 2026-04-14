//! Advancement tree — progress tracking.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AdvancementDef {
    pub id: String,
    pub parent: Option<String>,
    pub title: String,
    pub description: String,
    pub icon: String,
    pub frame: AdvancementFrame,
    pub criteria: Vec<String>,
    pub requirements: Vec<Vec<String>>,
    pub announce_to_chat: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvancementFrame {
    Task,
    Goal,
    Challenge,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerAdvancementProgress {
    pub completed: Vec<String>,
    pub in_progress: HashMap<String, Vec<String>>, // advancement_id → completed criteria
}

impl PlayerAdvancementProgress {
    pub fn has_completed(&self, id: &str) -> bool {
        self.completed.iter().any(|a| a == id)
    }

    pub fn grant_criterion(&mut self, adv_id: &str, criterion: &str) {
        self.in_progress
            .entry(adv_id.to_string())
            .or_default()
            .push(criterion.to_string());
    }

    pub fn complete(&mut self, id: String) {
        if !self.has_completed(&id) {
            self.completed.push(id.clone());
            self.in_progress.remove(&id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_once() {
        let mut p = PlayerAdvancementProgress::default();
        p.complete("test".into());
        p.complete("test".into());
        assert_eq!(p.completed.len(), 1);
    }
}
