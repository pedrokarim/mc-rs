use crate::inventory::PlayerInventory;
use crate::item_registry;
use mc_rs_proto::packets::player::{ItemStack, ItemStackWrapper};
use serde::{Deserialize, Serialize};
use std::fs;

use tracing::{info, warn};

const PLAYERS_DIR: &str = "players";

/// Version du format des IDs sauvegardés (items et blocs) :
/// - 0 (champ absent) : table d'items 1.26.0 + palette de blocs 1.26.20 ;
/// - 1 : table d'items et palette 1.26.40 (dragonfly v0.11.4) ;
/// - 2 : table d'items et palette 1.26.50 (dragonfly v0.11.5).
pub const CURRENT_PLAYER_DATA_VERSION: u32 = 2;

/// Libellé de la palette de blocs (`PaletteMigration::label`) de chaque
/// version de sauvegarde antérieure.
fn legacy_palette_label(data_version: u32) -> Option<&'static str> {
    match data_version {
        0 => Some(crate::world::palette_migration::UNMARKED_WORLD_LABEL),
        1 => Some("1.26.40"),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSaveData {
    pub position: [f64; 3],
    pub rotation: [f32; 2], // yaw, pitch
    pub gamemode: i32,
    pub health: f32,
    pub hunger: f32,
    pub spawn_position: Option<[f64; 3]>,
    /// Dimension où le joueur s'est déconnecté (0=overworld, 1=nether, 2=end).
    #[serde(default)]
    pub dimension: i32,
    #[serde(default)]
    pub inventory: SavedPlayerInventory,
    /// Voir [`CURRENT_PLAYER_DATA_VERSION`]. Absent des sauvegardes antérieures.
    #[serde(default)]
    pub data_version: u32,
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
            dimension: 0,
            inventory: SavedPlayerInventory::default(),
            data_version: CURRENT_PLAYER_DATA_VERSION,
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
    #[allow(clippy::too_many_arguments)]
    pub fn from_runtime(
        position: [f32; 3],
        rotation: [f32; 2],
        gamemode: i32,
        health: f32,
        hunger: f32,
        spawn_position: [f32; 3],
        dimension: i32,
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
            dimension,
            inventory: SavedPlayerInventory::from_runtime(inventory),
            data_version: CURRENT_PLAYER_DATA_VERSION,
        }
    }

    /// Convertit les IDs d'une sauvegarde antérieure vers les tables courantes.
    fn migrate(&mut self) {
        if self.data_version >= CURRENT_PLAYER_DATA_VERSION {
            return;
        }
        let version = self.data_version;
        let palette = legacy_palette_label(version)
            .and_then(crate::world::palette_migration::migration_by_label);
        let inv = &mut self.inventory;
        for slot in inv
            .slots
            .iter_mut()
            .chain(inv.armor.iter_mut())
            .chain(std::iter::once(&mut inv.offhand))
        {
            let item = &mut slot.item;
            (item.id, item.meta) = item_registry::migrate_saved_item(version, item.id, item.meta);
            item.block_runtime_id = match (item.block_runtime_id, palette) {
                (0, _) | (_, None) => 0,
                (rid, Some(m)) => m.remap(rid as u32) as i32,
            };
            if item.id == 0 {
                *slot = SavedItemStackWrapperData::default();
            }
        }
        self.data_version = CURRENT_PLAYER_DATA_VERSION;
    }
}

/// Load player data from disk. Returns None if no save exists.
pub fn load_player(xuid: &str) -> Option<PlayerSaveData> {
    let path = format!("{}/{}.json", PLAYERS_DIR, xuid);
    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<PlayerSaveData>(&content) {
            Ok(mut data) => {
                if data.data_version < CURRENT_PLAYER_DATA_VERSION {
                    info!(
                        "Migrating player data for XUID {} (v{} -> v{})",
                        xuid, data.data_version, CURRENT_PLAYER_DATA_VERSION
                    );
                    data.migrate();
                }
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

    #[test]
    fn legacy_save_migrates_item_ids() {
        // Sauvegarde sans `data_version` : IDs de la table 1.26.0 (423 = compass).
        let json = r#"{"position":[0,0,0],"rotation":[0,0],"gamemode":0,"health":20,
            "hunger":20,"spawn_position":null,
            "inventory":{"slots":[{"stack_id":1,"item":{"id":423,"count":1,"meta":0,
            "block_runtime_id":0}}],"armor":[],"held_slot":0}}"#;
        let mut data: PlayerSaveData = serde_json::from_str(json).unwrap();
        assert_eq!(data.data_version, 0);
        data.migrate();
        assert_eq!(data.data_version, CURRENT_PLAYER_DATA_VERSION);
        assert_eq!(
            data.inventory.slots[0].item.id,
            item_registry::network_id("minecraft:compass").unwrap()
        );
    }

    #[test]
    fn v1_save_migrates_items_and_blocks_from_1_26_40() {
        use crate::item_migration_data::LEGACY_ITEM_TABLES;
        // Sauvegarde v1 : IDs de la table d'items et de la palette 1.26.40.
        let table = LEGACY_ITEM_TABLES
            .iter()
            .find(|t| t.data_version == 1)
            .unwrap();
        let &(apple, _) = table
            .ids
            .iter()
            .find(|&&(_, name)| name == "minecraft:apple")
            .unwrap();
        let palette = crate::world::palette_migration::migration_by_label("1.26.40").unwrap();
        let json = format!(
            r#"{{"position":[0,0,0],"rotation":[0,0],"gamemode":0,"health":20,
            "hunger":20,"spawn_position":null,"data_version":1,
            "inventory":{{"slots":[{{"stack_id":1,"item":{{"id":{apple},"count":3,"meta":0,
            "block_runtime_id":42}}}}],"armor":[],"held_slot":0}}}}"#
        );
        let mut data: PlayerSaveData = serde_json::from_str(&json).unwrap();
        data.migrate();
        assert_eq!(data.data_version, CURRENT_PLAYER_DATA_VERSION);
        let item = &data.inventory.slots[0].item;
        assert_eq!(
            item.id,
            item_registry::network_id("minecraft:apple").unwrap()
        );
        assert_eq!(item.count, 3);
        assert_eq!(item.block_runtime_id, palette.remap(42) as i32);
    }
}
