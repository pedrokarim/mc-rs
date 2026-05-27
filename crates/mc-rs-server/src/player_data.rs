use crate::inventory::PlayerInventory;
use crate::item_registry;
use mc_rs_proto::packets::player::{ItemStack, ItemStackWrapper};
use serde::{Deserialize, Serialize};
use std::fs;

use tracing::{info, warn};

const PLAYERS_DIR: &str = "players";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSaveData {
    pub position: [f64; 3],
    pub rotation: [f32; 2], // yaw, pitch
    pub gamemode: i32,
    pub health: f32,
    pub hunger: f32,
    pub spawn_position: Option<[f64; 3]>,
    #[serde(default)]
    pub inventory: SavedPlayerInventory,
}

impl Default for PlayerSaveData {
    fn default() -> Self {
        Self {
            position: [0.5, -58.379, 0.5],
            rotation: [0.0, 0.0],
            gamemode: 0,
            health: 20.0,
            hunger: 20.0,
            spawn_position: None,
            inventory: SavedPlayerInventory::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SavedItemStackData {
    pub id: i32,
    pub count: u16,
    pub meta: u32,
    pub block_runtime_id: i32,
    #[serde(default)]
    pub extra_data: Vec<u8>,
}

impl From<&ItemStack> for SavedItemStackData {
    fn from(item: &ItemStack) -> Self {
        Self {
            id: item.id,
            count: item.count,
            meta: item.meta,
            block_runtime_id: item.block_runtime_id,
            extra_data: item.extra_data.clone(),
        }
    }
}

impl From<SavedItemStackData> for ItemStack {
    fn from(item: SavedItemStackData) -> Self {
        let id = item_registry::migrate_legacy_item_id(item.id);
        let block_runtime_id = if item.block_runtime_id != 0 {
            item.block_runtime_id
        } else {
            crate::inventory::item_to_block(id)
                .map(|runtime_id| runtime_id as i32)
                .unwrap_or(0)
        };

        Self {
            id,
            count: item.count,
            meta: item.meta,
            block_runtime_id,
            extra_data: item.extra_data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SavedItemStackWrapperData {
    #[serde(default)]
    pub stack_id: i32,
    #[serde(default)]
    pub item: SavedItemStackData,
}

impl From<&ItemStackWrapper> for SavedItemStackWrapperData {
    fn from(slot: &ItemStackWrapper) -> Self {
        Self {
            stack_id: slot.stack_id,
            item: SavedItemStackData::from(&slot.item),
        }
    }
}

impl From<SavedItemStackWrapperData> for ItemStackWrapper {
    fn from(slot: SavedItemStackWrapperData) -> Self {
        Self {
            stack_id: slot.stack_id,
            item: slot.item.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPlayerInventory {
    #[serde(default = "default_slots")]
    pub slots: Vec<SavedItemStackWrapperData>,
    #[serde(default = "default_armor")]
    pub armor: Vec<SavedItemStackWrapperData>,
    #[serde(default)]
    pub offhand: SavedItemStackWrapperData,
    #[serde(default)]
    pub held_slot: u8,
}

impl Default for SavedPlayerInventory {
    fn default() -> Self {
        Self {
            slots: default_slots(),
            armor: default_armor(),
            offhand: SavedItemStackWrapperData::default(),
            held_slot: 0,
        }
    }
}

fn default_slots() -> Vec<SavedItemStackWrapperData> {
    vec![SavedItemStackWrapperData::default(); 36]
}

fn default_armor() -> Vec<SavedItemStackWrapperData> {
    vec![SavedItemStackWrapperData::default(); 4]
}

impl SavedPlayerInventory {
    pub fn from_runtime(inventory: &PlayerInventory) -> Self {
        Self {
            slots: inventory
                .slots
                .iter()
                .map(SavedItemStackWrapperData::from)
                .collect(),
            armor: inventory
                .armor
                .iter()
                .map(SavedItemStackWrapperData::from)
                .collect(),
            offhand: SavedItemStackWrapperData::from(&inventory.offhand),
            held_slot: inventory.held_slot,
        }
    }

    pub fn into_runtime(self) -> PlayerInventory {
        PlayerInventory::from_parts(
            self.slots.into_iter().map(ItemStackWrapper::from).collect(),
            self.armor.into_iter().map(ItemStackWrapper::from).collect(),
            self.offhand.into(),
            self.held_slot,
        )
    }
}

impl PlayerSaveData {
    pub fn from_runtime(
        position: [f32; 3],
        rotation: [f32; 2],
        gamemode: i32,
        health: f32,
        hunger: f32,
        spawn_position: [f32; 3],
        inventory: &PlayerInventory,
    ) -> Self {
        Self {
            position: [position[0] as f64, position[1] as f64, position[2] as f64],
            rotation,
            gamemode,
            health,
            hunger,
            spawn_position: Some([
                spawn_position[0] as f64,
                spawn_position[1] as f64,
                spawn_position[2] as f64,
            ]),
            inventory: SavedPlayerInventory::from_runtime(inventory),
        }
    }
}

/// Load player data from disk. Returns None if no save exists.
pub fn load_player(xuid: &str) -> Option<PlayerSaveData> {
    let path = format!("{}/{}.json", PLAYERS_DIR, xuid);
    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(data) => {
                info!("Loaded player data for XUID {}", xuid);
                Some(data)
            }
            Err(e) => {
                warn!("Failed to parse player data {}: {}", path, e);
                None
            }
        },
        Err(_) => None,
    }
}

/// Save player data to disk.
pub fn save_player(xuid: &str, data: &PlayerSaveData) -> std::io::Result<()> {
    fs::create_dir_all(PLAYERS_DIR)?;
    let path = format!("{}/{}.json", PLAYERS_DIR, xuid);
    let json = serde_json::to_string_pretty(data)?;
    fs::write(&path, json)?;
    info!("Saved player data for XUID {}", xuid);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_inventory_round_trips_runtime_data() {
        let mut inventory = PlayerInventory::new();
        inventory.slots[0] = ItemStackWrapper::new(ItemStack::new(1, 32, 500), 11);
        inventory.armor[1] = ItemStackWrapper::new(ItemStack::new(307, 1, 0), 12);
        inventory.offhand = ItemStackWrapper::new(ItemStack::new(50, 5, 0), 13);
        inventory.held_slot = 2;

        let restored = SavedPlayerInventory::from_runtime(&inventory).into_runtime();

        assert_eq!(restored.slots[0].item.id, 1);
        assert_eq!(restored.slots[0].item.count, 32);
        assert_eq!(restored.slots[0].stack_id, 11);
        assert_eq!(restored.armor[1].item.id, 307);
        assert_eq!(restored.offhand.item.id, 50);
        assert_eq!(restored.held_slot, 2);
    }
}
