//! Persistance des métadonnées du monde dans `worlds/<name>/level.dat.json`.
//!
//! Stocke : seed, generator, time, weather state, gameRules, spawn position.
//! Format JSON simple (pas de NBT comme Bedrock vanilla, pour simplicité).
//!
//! Chargé au boot du monde (si présent) → restaure le state.
//! Sauvé périodiquement + au shutdown.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LevelData {
    pub seed: u64,
    pub generator: String,
    pub time: i32,
    pub spawn_x: f32,
    pub spawn_y: f32,
    pub spawn_z: f32,
    #[serde(default)]
    pub do_daylight_cycle: bool,
    #[serde(default)]
    pub do_weather_cycle: bool,
    #[serde(default)]
    pub is_raining: bool,
    #[serde(default)]
    pub is_thundering: bool,
    #[serde(default)]
    pub last_played_unix: u64,
    #[serde(default)]
    pub world_name: String,
}

impl LevelData {
    pub fn path_for_world(world_dir: &Path) -> PathBuf {
        world_dir.join("level.dat.json")
    }

    pub fn load(world_dir: &Path) -> Option<Self> {
        let path = Self::path_for_world(world_dir);
        let raw = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&raw).ok()
    }

    pub fn save(&self, world_dir: &Path) -> std::io::Result<()> {
        let path = Self::path_for_world(world_dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let tmp = std::env::temp_dir().join(format!("mc-rs-leveldat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let data = LevelData {
            seed: 42,
            generator: "normal".into(),
            time: 6000,
            spawn_x: 0.5,
            spawn_y: 65.0,
            spawn_z: 0.5,
            do_daylight_cycle: true,
            do_weather_cycle: false,
            is_raining: false,
            is_thundering: false,
            last_played_unix: 1234567890,
            world_name: "world".into(),
        };
        data.save(&tmp).unwrap();
        let loaded = LevelData::load(&tmp).unwrap();
        assert_eq!(loaded.seed, 42);
        assert_eq!(loaded.time, 6000);
        assert_eq!(loaded.world_name, "world");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
