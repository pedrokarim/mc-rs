//! Stockage persistent des chests (per-position).
//!
//! Port simplifié de `.reference/PocketMine-MP/src/block/tile/Chest.php` :
//! chaque chest occupe une position et stocke 27 slots `ItemStack`. Pairing
//! double-chest sera géré côté `chest_system` plus tard.
//!
//! L'ouverture/transactions UI passent par `InventoryManager` (window ID
//! dynamique) — ce module est juste le data store.

use std::collections::HashMap;

use mc_rs_proto::packets::player::ItemStack;

pub const CHEST_SLOTS: usize = 27;

#[derive(Debug, Clone)]
pub struct ChestData {
    pub slots: Vec<ItemStack>,
    pub viewers: u32,
}

impl Default for ChestData {
    fn default() -> Self {
        Self {
            slots: vec![ItemStack::AIR; CHEST_SLOTS],
            viewers: 0,
        }
    }
}

#[derive(Default)]
pub struct ChestManager {
    chests: HashMap<(i32, i32, i32), ChestData>,
}

impl ChestManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Récupère ou crée le chest à la position donnée.
    pub fn get_or_create(&mut self, pos: (i32, i32, i32)) -> &mut ChestData {
        self.chests.entry(pos).or_default()
    }

    pub fn get(&self, pos: (i32, i32, i32)) -> Option<&ChestData> {
        self.chests.get(&pos)
    }

    pub fn set_slot(&mut self, pos: (i32, i32, i32), slot: usize, item: ItemStack) {
        let data = self.chests.entry(pos).or_default();
        if slot < data.slots.len() {
            data.slots[slot] = item;
        }
    }

    pub fn add_viewer(&mut self, pos: (i32, i32, i32)) -> u32 {
        let data = self.chests.entry(pos).or_default();
        data.viewers = data.viewers.saturating_add(1);
        data.viewers
    }

    pub fn remove_viewer(&mut self, pos: (i32, i32, i32)) -> u32 {
        let data = self.chests.entry(pos).or_default();
        data.viewers = data.viewers.saturating_sub(1);
        data.viewers
    }

    /// Drop tous les items du chest (à appeler quand le chest est cassé).
    /// Retourne la liste des items (le caller spawn les ItemEntity).
    pub fn drop_all(&mut self, pos: (i32, i32, i32)) -> Vec<ItemStack> {
        let Some(data) = self.chests.remove(&pos) else {
            return Vec::new();
        };
        data.slots.into_iter().filter(|s| !s.is_air()).collect()
    }

    pub fn count(&self) -> usize {
        self.chests.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_get_set() {
        let mut mgr = ChestManager::new();
        assert!(mgr.get((0, 64, 0)).is_none());
        let item = ItemStack::new(3, 5, 0);
        mgr.set_slot((0, 64, 0), 0, item.clone());
        let data = mgr.get((0, 64, 0)).expect("created");
        assert_eq!(data.slots[0].id, 3);
        assert_eq!(data.slots[0].count, 5);
    }

    #[test]
    fn drop_all_returns_non_air() {
        let mut mgr = ChestManager::new();
        mgr.set_slot((0, 64, 0), 0, ItemStack::new(3, 5, 0));
        mgr.set_slot((0, 64, 0), 5, ItemStack::new(7, 1, 0));
        let dropped = mgr.drop_all((0, 64, 0));
        assert_eq!(dropped.len(), 2);
        assert!(mgr.get((0, 64, 0)).is_none());
    }

    #[test]
    fn viewers_track() {
        let mut mgr = ChestManager::new();
        assert_eq!(mgr.add_viewer((0, 64, 0)), 1);
        assert_eq!(mgr.add_viewer((0, 64, 0)), 2);
        assert_eq!(mgr.remove_viewer((0, 64, 0)), 1);
    }
}
