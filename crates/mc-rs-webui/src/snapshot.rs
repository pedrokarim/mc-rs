//! `ServerSnapshot` : état du serveur exposé au webui pour le dashboard.
//!
//! Mis à jour par la main loop tokio (tous les N server ticks) et lu par les
//! handlers HTTP/WS. Un `RwLock` suffit : écriture ~20 Hz, lectures courtes.

use serde::Serialize;
use std::time::Instant;

/// Clone immuable du registre joueurs à un instant T.
#[derive(Debug, Clone, Serialize)]
pub struct PlayerSnapshot {
    pub addr: String,
    pub name: String,
    pub uuid: [u8; 16],
    pub xuid: String,
    pub entity_id: u64,
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub gamemode: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerSnapshot {
    /// TPS mesuré (fenêtre glissante 1s). 0.0 si pas encore de mesure.
    pub tps: f32,
    /// Tick server cumulé depuis le boot.
    pub total_ticks: u64,
    /// Timestamp unix (secondes) du boot.
    pub uptime_start_unix: i64,
    /// Nombre de chunks chargés en mémoire côté ChunkCache.
    pub chunks_loaded: usize,
    pub players: Vec<PlayerSnapshot>,
    /// Time of day (0..24000).
    pub world_time: i64,
    /// "clear" / "rain" / "thunder" (simplifié V1).
    pub weather: String,
    /// 0..3 (peaceful/easy/normal/hard).
    pub difficulty: i32,
    pub gamemode: i32,
    /// MOTD + infos serveur (statiques).
    pub motd: String,
    pub world_name: String,
    pub max_players: u32,

    /// `None` si pas initialisé (bench / stub). Utilisé pour afficher l'uptime.
    #[serde(skip)]
    pub boot_instant: Option<Instant>,
}

impl Default for ServerSnapshot {
    fn default() -> Self {
        Self {
            tps: 0.0,
            total_ticks: 0,
            uptime_start_unix: 0,
            chunks_loaded: 0,
            players: Vec::new(),
            world_time: 0,
            weather: "clear".to_string(),
            difficulty: 2,
            gamemode: 0,
            motd: String::new(),
            world_name: String::new(),
            max_players: 0,
            boot_instant: None,
        }
    }
}

impl ServerSnapshot {
    /// Uptime en secondes depuis le boot (0 si pas initialisé).
    pub fn uptime_seconds(&self) -> u64 {
        self.boot_instant
            .map(|inst| inst.elapsed().as_secs())
            .unwrap_or(0)
    }
}
