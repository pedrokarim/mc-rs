//! Ender chest — port PMMP `src/inventory/PlayerEnderInventory.php`.
//! 27 slots d'inventaire partagés entre toutes les ender chests d'un joueur.

use mc_rs_proto::packets::player::ItemStack;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct EnderInventory {
    pub items: Vec<ItemStack>, // 27 slots
}

impl EnderInventory {
    pub fn new() -> Self {
        Self {
            items: vec![ItemStack::AIR; 27],
        }
    }

    pub fn size(&self) -> usize {
        self.items.len()
    }

    pub fn set_item(&mut self, slot: usize, item: ItemStack) -> bool {
        if slot < self.items.len() {
            self.items[slot] = item;
            true
        } else {
            false
        }
    }

    pub fn get_item(&self, slot: usize) -> Option<&ItemStack> {
        self.items.get(slot)
    }

    pub fn is_empty(&self) -> bool {
        self.items.iter().all(|s| s.is_air())
    }
}

impl Default for EnderInventory {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry global des ender inventories par joueur (persistant).
#[derive(Debug, Default)]
pub struct EnderInventoryRegistry {
    pub per_xuid: HashMap<String, EnderInventory>,
}

impl EnderInventoryRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_create(&mut self, xuid: &str) -> &mut EnderInventory {
        self.per_xuid.entry(xuid.to_string()).or_default()
    }

    pub fn get(&self, xuid: &str) -> Option<&EnderInventory> {
        self.per_xuid.get(xuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ender_inventory_27_slots() {
        let e = EnderInventory::new();
        assert_eq!(e.size(), 27);
    }

    #[test]
    fn registry_creates_on_demand() {
        let mut reg = EnderInventoryRegistry::new();
        let e = reg.get_or_create("xuid_1");
        e.set_item(0, ItemStack::new(1, 10, 0));
        assert!(reg.get("xuid_1").is_some());
    }
}
