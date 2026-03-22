use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{info, warn};

const PLAYERS_DIR: &str = "players";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSaveData {
    pub position: [f64; 3],
    pub rotation: [f32; 2], // yaw, pitch
    pub gamemode: i32,
    pub health: f32,
    pub hunger: f32,
}

impl Default for PlayerSaveData {
    fn default() -> Self {
        Self {
            position: [0.5, -58.379, 0.5],
            rotation: [0.0, 0.0],
            gamemode: 0,
            health: 20.0,
            hunger: 20.0,
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
