//! Scoreboard — port de PMMP `src/scoreboard/*` (pas de fichier direct mais
//! basé sur les packets `SetScore`, `SetScoreboardIdentity`,
//! `RemoveObjectivePacket`, `SetDisplayObjectivePacket`).

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectiveDisplaySlot {
    /// Affiché à gauche de l'écran.
    Sidebar,
    /// Affiché sous le nom du joueur.
    BelowName,
    /// Liste des joueurs (tab).
    List,
}

impl ObjectiveDisplaySlot {
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Sidebar => "sidebar",
            Self::BelowName => "belowname",
            Self::List => "list",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// Objectif (= une table de scores). Port PMMP `Objective`.
#[derive(Debug, Clone)]
pub struct Objective {
    pub name: String,
    pub display_name: String,
    pub criteria: String,
    /// player_name or entity_id → score.
    pub scores: HashMap<String, i32>,
    pub sort_order: SortOrder,
}

impl Objective {
    pub fn new(name: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            display_name: display_name.into(),
            criteria: "dummy".to_string(),
            scores: HashMap::new(),
            sort_order: SortOrder::Descending,
        }
    }

    pub fn set_score(&mut self, player: impl Into<String>, score: i32) {
        self.scores.insert(player.into(), score);
    }

    pub fn get_score(&self, player: &str) -> Option<i32> {
        self.scores.get(player).copied()
    }

    pub fn remove_score(&mut self, player: &str) {
        self.scores.remove(player);
    }

    pub fn sorted_entries(&self) -> Vec<(&String, &i32)> {
        let mut entries: Vec<_> = self.scores.iter().collect();
        entries.sort_by(|a, b| match self.sort_order {
            SortOrder::Ascending => a.1.cmp(b.1),
            SortOrder::Descending => b.1.cmp(a.1),
        });
        entries
    }
}

/// Manager global des scoreboards. Port PMMP `Scoreboard`.
#[derive(Debug, Default)]
pub struct ScoreboardManager {
    pub objectives: HashMap<String, Objective>,
    pub displays: HashMap<ObjectiveDisplaySlot, String>,
}

impl ScoreboardManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_objective(&mut self, obj: Objective) {
        self.objectives.insert(obj.name.clone(), obj);
    }

    pub fn remove_objective(&mut self, name: &str) {
        self.objectives.remove(name);
        self.displays.retain(|_, v| v != name);
    }

    pub fn set_display(&mut self, slot: ObjectiveDisplaySlot, objective_name: &str) {
        self.displays.insert(slot, objective_name.to_string());
    }

    pub fn get_display(&self, slot: ObjectiveDisplaySlot) -> Option<&Objective> {
        self.displays
            .get(&slot)
            .and_then(|name| self.objectives.get(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objective_tracks_scores() {
        let mut obj = Objective::new("kills", "Kills");
        obj.set_score("Alice", 5);
        obj.set_score("Bob", 3);
        assert_eq!(obj.get_score("Alice"), Some(5));
        assert_eq!(obj.get_score("Bob"), Some(3));
    }

    #[test]
    fn sorted_descending() {
        let mut obj = Objective::new("kills", "Kills");
        obj.set_score("Alice", 5);
        obj.set_score("Bob", 10);
        obj.set_score("Charlie", 2);
        let sorted = obj.sorted_entries();
        assert_eq!(sorted[0].0, "Bob");
        assert_eq!(sorted[1].0, "Alice");
        assert_eq!(sorted[2].0, "Charlie");
    }

    #[test]
    fn manager_display_objective() {
        let mut mgr = ScoreboardManager::new();
        mgr.add_objective(Objective::new("kills", "Kills"));
        mgr.set_display(ObjectiveDisplaySlot::Sidebar, "kills");
        let obj = mgr.get_display(ObjectiveDisplaySlot::Sidebar);
        assert!(obj.is_some());
        assert_eq!(obj.unwrap().name, "kills");
    }
}
