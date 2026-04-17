//! Recipe book — port conceptuel Bedrock. Track quelles recettes le joueur
//! a débloquées.

use std::collections::HashMap;
use std::collections::HashSet;
use std::net::SocketAddr;

#[derive(Debug, Clone, Default)]
pub struct PlayerRecipeBook {
    /// Recipe IDs connues.
    pub unlocked_recipes: HashSet<String>,
    /// Recipes notifiées "newly unlocked".
    pub pending_notifications: HashSet<String>,
}

impl PlayerRecipeBook {
    pub fn unlock(&mut self, recipe_id: impl Into<String>) -> bool {
        let id = recipe_id.into();
        if self.unlocked_recipes.insert(id.clone()) {
            self.pending_notifications.insert(id);
            true
        } else {
            false
        }
    }

    pub fn knows(&self, recipe_id: &str) -> bool {
        self.unlocked_recipes.contains(recipe_id)
    }

    pub fn drain_notifications(&mut self) -> Vec<String> {
        let v: Vec<_> = self.pending_notifications.drain().collect();
        v
    }
}

#[derive(Debug, Default)]
pub struct RecipeBookManager {
    pub per_player: HashMap<SocketAddr, PlayerRecipeBook>,
}

impl RecipeBookManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn unlock_for(&mut self, addr: SocketAddr, recipe_id: &str) -> bool {
        self.per_player.entry(addr).or_default().unlock(recipe_id)
    }

    pub fn drain_notifications(&mut self, addr: &SocketAddr) -> Vec<String> {
        self.per_player
            .get_mut(addr)
            .map(|p| p.drain_notifications())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlock_once() {
        let mut rb = PlayerRecipeBook::default();
        assert!(rb.unlock("crafting/planks_oak"));
        assert!(!rb.unlock("crafting/planks_oak"));
    }

    #[test]
    fn notifications_drained() {
        let mut rb = PlayerRecipeBook::default();
        rb.unlock("r1");
        rb.unlock("r2");
        let n = rb.drain_notifications();
        assert_eq!(n.len(), 2);
        let n2 = rb.drain_notifications();
        assert!(n2.is_empty());
    }
}
