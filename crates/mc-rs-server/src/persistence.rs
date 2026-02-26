//! World metadata (level.dat) and player data persistence.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::BufMut;
use mc_rs_nbt::{NbtRoot, NbtTag};
use mc_rs_proto::item_stack::ItemStack;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::connection::{ActiveEffect, PlayerConnection};

// ─── level.dat ──────────────────────────────────────────────────────────────

/// World metadata stored in level.dat.
pub struct LevelDat {
    pub level_name: String,
    pub game_type: i32,
    pub difficulty: i32,
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub spawn_z: i32,
    pub random_seed: i64,
    pub time: i64,
    pub current_tick: i64,
    pub last_played: i64,
    pub generator: i32,
    pub storage_version: i32,
}

impl LevelDat {
    /// Create a new level.dat with default values.
    pub fn new(name: &str, seed: i64, generator_str: &str, spawn: (i32, i32, i32)) -> Self {
        let generator = match generator_str {
            "flat" => 2,
            _ => 1, // default/overworld
        };
        Self {
            level_name: name.to_string(),
            game_type: 0,
            difficulty: 2,
            spawn_x: spawn.0,
            spawn_y: spawn.1,
            spawn_z: spawn.2,
            random_seed: seed,
            time: 0,
            current_tick: 0,
            last_played: unix_timestamp(),
            generator,
            storage_version: 10,
        }
    }

    /// Load level.dat from a file.
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read(path)?;
        if data.len() < 8 {
            return Err("level.dat too short".into());
        }

        let _storage_version = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let data_length = i32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;

        if data.len() < 8 + data_length {
            return Err("level.dat data truncated".into());
        }

        let mut cursor = Cursor::new(&data[8..8 + data_length]);
        let root = mc_rs_nbt::read_nbt_le(&mut cursor)?;
        let c = &root.compound;

        Ok(Self {
            level_name: c
                .get("LevelName")
                .and_then(|t| t.as_string())
                .unwrap_or("world")
                .to_string(),
            game_type: c.get("GameType").and_then(|t| t.as_int()).unwrap_or(0),
            difficulty: c.get("Difficulty").and_then(|t| t.as_int()).unwrap_or(2),
            spawn_x: c.get("SpawnX").and_then(|t| t.as_int()).unwrap_or(0),
            spawn_y: c.get("SpawnY").and_then(|t| t.as_int()).unwrap_or(64),
            spawn_z: c.get("SpawnZ").and_then(|t| t.as_int()).unwrap_or(0),
            random_seed: c.get("RandomSeed").and_then(|t| t.as_long()).unwrap_or(0),
            time: c.get("Time").and_then(|t| t.as_long()).unwrap_or(0),
            current_tick: c.get("currentTick").and_then(|t| t.as_long()).unwrap_or(0),
            last_played: c.get("LastPlayed").and_then(|t| t.as_long()).unwrap_or(0),
            generator: c.get("Generator").and_then(|t| t.as_int()).unwrap_or(1),
            storage_version: c
                .get("StorageVersion")
                .and_then(|t| t.as_int())
                .unwrap_or(10),
        })
    }

    /// Save level.dat to a file.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        // Backup old file
        if path.exists() {
            let backup = path.with_extension("dat_old");
            std::fs::copy(path, backup).ok();
        }

        let mut compound = HashMap::new();
        compound.insert("LevelName".into(), NbtTag::String(self.level_name.clone()));
        compound.insert("GameType".into(), NbtTag::Int(self.game_type));
        compound.insert("Difficulty".into(), NbtTag::Int(self.difficulty));
        compound.insert("SpawnX".into(), NbtTag::Int(self.spawn_x));
        compound.insert("SpawnY".into(), NbtTag::Int(self.spawn_y));
        compound.insert("SpawnZ".into(), NbtTag::Int(self.spawn_z));
        compound.insert("RandomSeed".into(), NbtTag::Long(self.random_seed));
        compound.insert("Time".into(), NbtTag::Long(self.time));
        compound.insert("currentTick".into(), NbtTag::Long(self.current_tick));
        compound.insert("LastPlayed".into(), NbtTag::Long(self.last_played));
        compound.insert("Generator".into(), NbtTag::Int(self.generator));
        compound.insert("StorageVersion".into(), NbtTag::Int(self.storage_version));
        compound.insert("NetworkVersion".into(), NbtTag::Int(766));

        let root = NbtRoot::new("", compound);

        // Serialize NBT to bytes
        let mut nbt_buf = Vec::new();
        mc_rs_nbt::write_nbt_le(&mut nbt_buf, &root);

        // Write header + NBT
        let mut file_buf = Vec::with_capacity(8 + nbt_buf.len());
        file_buf.put_i32_le(self.storage_version);
        file_buf.put_i32_le(nbt_buf.len() as i32);
        file_buf.extend_from_slice(&nbt_buf);

        std::fs::write(path, &file_buf)?;
        Ok(())
    }

    /// Update timestamps before saving.
    pub fn update_on_save(&mut self, tick: u64) {
        self.last_played = unix_timestamp();
        self.current_tick = tick as i64;
    }
}

fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ─── Player data ────────────────────────────────────────────────────────────

/// Serializable player data for JSON persistence.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerData {
    pub position: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub gamemode: i32,
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
    pub exhaustion: f32,
    pub fire_ticks: i32,
    pub air_ticks: i32,
    pub fall_distance: f32,
    pub inventory: SerializedInventory,
    pub effects: Vec<SerializedEffect>,
    #[serde(default)]
    pub xp_level: i32,
    #[serde(default)]
    pub xp_total: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializedInventory {
    pub main: Vec<SerializedItem>,
    pub armor: Vec<SerializedItem>,
    pub offhand: SerializedItem,
    pub held_slot: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedItem {
    pub runtime_id: i32,
    pub count: u16,
    pub metadata: u16,
    pub block_runtime_id: i32,
    pub nbt_data: Vec<u8>,
    pub can_place_on: Vec<String>,
    pub can_destroy: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializedEffect {
    pub effect_id: i32,
    pub amplifier: i32,
    pub remaining_ticks: i32,
}

impl SerializedItem {
    fn from_item_stack(item: &ItemStack) -> Self {
        Self {
            runtime_id: item.runtime_id,
            count: item.count,
            metadata: item.metadata,
            block_runtime_id: item.block_runtime_id,
            nbt_data: item.nbt_data.clone(),
            can_place_on: item.can_place_on.clone(),
            can_destroy: item.can_destroy.clone(),
        }
    }

    fn to_item_stack(&self, stack_network_id: i32) -> ItemStack {
        ItemStack {
            runtime_id: self.runtime_id,
            count: self.count,
            metadata: self.metadata,
            block_runtime_id: self.block_runtime_id,
            nbt_data: self.nbt_data.clone(),
            can_place_on: self.can_place_on.clone(),
            can_destroy: self.can_destroy.clone(),
            stack_network_id,
        }
    }

    #[cfg(test)]
    fn empty() -> Self {
        Self {
            runtime_id: 0,
            count: 0,
            metadata: 0,
            block_runtime_id: 0,
            nbt_data: Vec::new(),
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
        }
    }
}

impl PlayerData {
    /// Extract persistent state from a PlayerConnection.
    pub fn from_connection(conn: &PlayerConnection) -> Self {
        Self {
            position: [conn.position.x, conn.position.y, conn.position.z],
            pitch: conn.pitch,
            yaw: conn.yaw,
            head_yaw: conn.head_yaw,
            gamemode: conn.gamemode,
            health: conn.health,
            food: conn.food,
            saturation: conn.saturation,
            exhaustion: conn.exhaustion,
            fire_ticks: conn.fire_ticks,
            air_ticks: conn.air_ticks,
            fall_distance: conn.fall_distance,
            inventory: SerializedInventory {
                main: conn
                    .inventory
                    .main
                    .iter()
                    .map(SerializedItem::from_item_stack)
                    .collect(),
                armor: conn
                    .inventory
                    .armor
                    .iter()
                    .map(SerializedItem::from_item_stack)
                    .collect(),
                offhand: SerializedItem::from_item_stack(&conn.inventory.offhand),
                held_slot: conn.inventory.held_slot,
            },
            effects: conn
                .effects
                .iter()
                .map(|e| SerializedEffect {
                    effect_id: e.effect_id,
                    amplifier: e.amplifier,
                    remaining_ticks: e.remaining_ticks,
                })
                .collect(),
            xp_level: conn.xp_level,
            xp_total: conn.xp_total,
        }
    }

    /// Apply loaded data to a PlayerConnection, overwriting defaults.
    pub fn apply_to_connection(&self, conn: &mut PlayerConnection) {
        conn.position.x = self.position[0];
        conn.position.y = self.position[1];
        conn.position.z = self.position[2];
        conn.pitch = self.pitch;
        conn.yaw = self.yaw;
        conn.head_yaw = self.head_yaw;
        conn.gamemode = self.gamemode;
        conn.health = self.health;
        conn.food = self.food;
        conn.saturation = self.saturation;
        conn.exhaustion = self.exhaustion;
        conn.fire_ticks = self.fire_ticks;
        conn.air_ticks = self.air_ticks;
        conn.fall_distance = self.fall_distance;

        // Restore inventory
        for (i, item) in self.inventory.main.iter().enumerate() {
            if i < conn.inventory.main.len() {
                conn.inventory.main[i] = item.to_item_stack(0);
            }
        }
        for (i, item) in self.inventory.armor.iter().enumerate() {
            if i < conn.inventory.armor.len() {
                conn.inventory.armor[i] = item.to_item_stack(0);
            }
        }
        conn.inventory.offhand = self.inventory.offhand.to_item_stack(0);
        conn.inventory.held_slot = self.inventory.held_slot;

        // Restore XP
        conn.xp_level = self.xp_level;
        conn.xp_total = self.xp_total;

        // Restore effects
        conn.effects = self
            .effects
            .iter()
            .map(|e| ActiveEffect {
                effect_id: e.effect_id,
                amplifier: e.amplifier,
                remaining_ticks: e.remaining_ticks,
            })
            .collect();
    }

    /// Load player data from a JSON file.
    pub fn load(world_dir: &Path, uuid: &str) -> Option<Self> {
        let path = world_dir.join("players").join(format!("{uuid}.json"));
        let data = std::fs::read_to_string(&path).ok()?;
        match serde_json::from_str(&data) {
            Ok(player) => Some(player),
            Err(e) => {
                warn!("Failed to parse player data for {uuid}: {e}");
                None
            }
        }
    }

    /// Save player data to a JSON file.
    pub fn save(&self, world_dir: &Path, uuid: &str) -> std::io::Result<()> {
        let dir = world_dir.join("players");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{uuid}.json"));
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mc_rs_persist_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn level_dat_roundtrip() {
        let dir = temp_dir();
        let path = dir.join("level.dat");

        let dat = LevelDat::new("TestWorld", 12345, "default", (10, 64, -20));
        dat.save(&path).unwrap();

        let loaded = LevelDat::load(&path).unwrap();
        assert_eq!(loaded.level_name, "TestWorld");
        assert_eq!(loaded.random_seed, 12345);
        assert_eq!(loaded.generator, 1); // default
        assert_eq!(loaded.spawn_x, 10);
        assert_eq!(loaded.spawn_y, 64);
        assert_eq!(loaded.spawn_z, -20);
        assert_eq!(loaded.storage_version, 10);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn level_dat_flat_generator() {
        let dir = temp_dir();
        let path = dir.join("level.dat");

        let dat = LevelDat::new("FlatWorld", 0, "flat", (0, 4, 0));
        dat.save(&path).unwrap();

        let loaded = LevelDat::load(&path).unwrap();
        assert_eq!(loaded.generator, 2); // flat

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn level_dat_backup_created() {
        let dir = temp_dir();
        let path = dir.join("level.dat");

        let dat = LevelDat::new("World1", 1, "default", (0, 64, 0));
        dat.save(&path).unwrap();
        // Save again — should create level.dat_old
        dat.save(&path).unwrap();

        assert!(dir.join("level.dat_old").exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn player_data_roundtrip() {
        let dir = temp_dir();

        let mut inv = SerializedInventory {
            main: (0..36).map(|_| SerializedItem::empty()).collect(),
            armor: (0..4).map(|_| SerializedItem::empty()).collect(),
            offhand: SerializedItem::empty(),
            held_slot: 3,
        };
        // Put a diamond sword in slot 0
        inv.main[0] = SerializedItem {
            runtime_id: 799,
            count: 1,
            metadata: 0,
            block_runtime_id: 0,
            nbt_data: vec![0x0A, 0x00, 0x00],
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
        };

        let data = PlayerData {
            position: [10.5, 65.62, -20.3],
            pitch: 15.0,
            yaw: 90.0,
            head_yaw: 90.0,
            gamemode: 0,
            health: 18.5,
            food: 15,
            saturation: 3.5,
            exhaustion: 1.2,
            fire_ticks: 0,
            air_ticks: 300,
            fall_distance: 0.0,
            inventory: inv,
            effects: vec![SerializedEffect {
                effect_id: 1,
                amplifier: 0,
                remaining_ticks: 600,
            }],
            xp_level: 5,
            xp_total: 160,
        };

        data.save(&dir, "test-uuid-1234").unwrap();

        let loaded = PlayerData::load(&dir, "test-uuid-1234").unwrap();
        assert_eq!(loaded.position, [10.5, 65.62, -20.3]);
        assert_eq!(loaded.health, 18.5);
        assert_eq!(loaded.food, 15);
        assert_eq!(loaded.inventory.held_slot, 3);
        assert_eq!(loaded.inventory.main[0].runtime_id, 799);
        assert_eq!(loaded.inventory.main[0].count, 1);
        assert_eq!(loaded.inventory.main[0].nbt_data, vec![0x0A, 0x00, 0x00]);
        assert_eq!(loaded.effects.len(), 1);
        assert_eq!(loaded.effects[0].effect_id, 1);
        assert_eq!(loaded.effects[0].remaining_ticks, 600);
        assert_eq!(loaded.xp_level, 5);
        assert_eq!(loaded.xp_total, 160);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn player_data_missing_returns_none() {
        let dir = temp_dir();
        assert!(PlayerData::load(&dir, "nonexistent-uuid").is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn level_dat_nbt_header_format() {
        let dir = temp_dir();
        let path = dir.join("level.dat");

        let dat = LevelDat::new("Test", 0, "default", (0, 64, 0));
        dat.save(&path).unwrap();

        let raw = std::fs::read(&path).unwrap();
        // First 4 bytes: storage_version (10 as i32_le)
        assert_eq!(i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]), 10);
        // Next 4 bytes: data length
        let data_len = i32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]) as usize;
        assert_eq!(raw.len(), 8 + data_len);
        // NBT starts with TAG_Compound (0x0A)
        assert_eq!(raw[8], 0x0A);

        std::fs::remove_dir_all(&dir).ok();
    }
}
