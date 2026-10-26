//! Persistance des métadonnées du monde dans `worlds/<name>/level.dat.json`.
//!
//! Stocke : seed, generator, time, weather state, spawn du monde (`/setworldspawn`).
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
    /// Spawn du monde fixé par `/setworldspawn` ; absent, il est calculé sur le terrain. Les anciens champs
    /// `spawn_x`/`spawn_y`/`spawn_z` (toujours 0,5 / 64 / 0,5, jamais relus) sont ignorés à la lecture.
    #[serde(default)]
    pub world_spawn: Option<[f32; 3]>,
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

    /// Fixe le spawn du monde dans le fichier existant (ou dans un nouveau), sans toucher au reste.
    pub fn save_world_spawn(world_dir: &Path, spawn: Option<[f32; 3]>) -> std::io::Result<()> {
        let mut data = Self::load(world_dir).unwrap_or_default();
        data.world_spawn = spawn;
        data.save(world_dir)
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
            world_spawn: Some([0.5, 65.0, 0.5]),
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
        assert_eq!(loaded.world_spawn, Some([0.5, 65.0, 0.5]));

        // Les anciens fichiers portent spawn_x/y/z et pas de world_spawn : spawn calculé sur le terrain.
        std::fs::write(
            LevelData::path_for_world(&tmp),
            r#"{"seed":1,"generator":"noise","time":0,"spawn_x":0.5,"spawn_y":64.0,"spawn_z":0.5}"#,
        )
        .unwrap();
        assert_eq!(LevelData::load(&tmp).unwrap().world_spawn, None);
        LevelData::save_world_spawn(&tmp, Some([1.0, 2.0, 3.0])).unwrap();
        let loaded = LevelData::load(&tmp).unwrap();
        assert_eq!(loaded.world_spawn, Some([1.0, 2.0, 3.0]));
        assert_eq!(loaded.generator, "noise");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
