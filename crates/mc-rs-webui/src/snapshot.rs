//! `ServerSnapshot` : état serveur exposé au webui pour le dashboard.
//!
//! Mis à jour par la main loop tokio (tous les N server ticks) et lu par les
//! handlers HTTP/WS. Un `RwLock` suffit : écriture ~20 Hz, lectures courtes.
//!
//! Les `HistoryRing` gardent 300 derniers échantillons à ~1 Hz (5 min glissantes)
//! pour alimenter les graphs time-series côté dashboard. Downsampling géré côté
//! serveur : la main loop update le snapshot à 20 Hz, mais ne push dans les
//! rings qu'une fois par seconde.

use serde::Serialize;
use std::collections::VecDeque;
use std::time::Instant;

/// Nombre maximal d'échantillons gardés (5 min @ 1 Hz).
pub const HISTORY_CAPACITY: usize = 300;

/// Buffer circulaire sérialisable d'un historique de métriques.
#[derive(Debug, Clone, Serialize)]
pub struct HistoryRing {
    /// Valeurs (float pour pouvoir mixer compteurs et moyennes).
    pub values: VecDeque<f32>,
    /// Timestamp unix seconds du dernier point (le client dérive les abscisses).
    pub last_ts: i64,
}

impl Default for HistoryRing {
    fn default() -> Self {
        Self {
            values: VecDeque::with_capacity(HISTORY_CAPACITY),
            last_ts: 0,
        }
    }
}

impl HistoryRing {
    pub fn push(&mut self, value: f32, ts: i64) {
        if self.values.len() >= HISTORY_CAPACITY {
            self.values.pop_front();
        }
        self.values.push_back(value);
        self.last_ts = ts;
    }

    pub fn latest(&self) -> f32 {
        self.values.back().copied().unwrap_or(0.0)
    }
}

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

/// Instantané système + process.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SystemStats {
    /// RSS process en MB.
    pub mem_mb: f32,
    /// Part de mémoire système utilisée (0.0..1.0).
    pub mem_total_mb: f32,
    /// CPU utilisé par le process (0.0..100.0 sur un seul coeur ; >100 possible sur multi-core).
    pub cpu_percent: f32,
    pub threads: u32,
    pub pid: u32,
    pub host_cpu_count: u32,
}

/// Compteurs réseau cumulés depuis le boot.
#[derive(Debug, Clone, Serialize, Default)]
pub struct NetStats {
    pub bytes_in_total: u64,
    pub bytes_out_total: u64,
    /// Rate calculé entre deux updates du ring (1 Hz).
    pub bytes_in_per_sec: u32,
    pub bytes_out_per_sec: u32,
    pub active_sessions: u32,
}

/// Compteurs d'entités gérées par la main loop.
#[derive(Debug, Clone, Serialize, Default)]
pub struct EntityStats {
    pub players: u32,
    pub mobs: u32,
    pub items: u32,
    pub passive: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerSnapshot {
    // ── État instantané ──
    pub tps: f32,
    pub total_ticks: u64,
    pub uptime_start_unix: i64,
    pub chunks_loaded: usize,
    pub players: Vec<PlayerSnapshot>,
    pub world_time: i64,
    pub weather: String,
    pub difficulty: i32,
    pub gamemode: i32,
    pub motd: String,
    pub world_name: String,
    pub max_players: u32,
    pub system: SystemStats,
    pub net: NetStats,
    pub entities: EntityStats,

    // ── Historiques (ring-buffers 300 × 1 Hz = 5 min) ──
    pub history_tps: HistoryRing,
    pub history_players: HistoryRing,
    pub history_chunks: HistoryRing,
    pub history_mem_mb: HistoryRing,
    pub history_cpu_percent: HistoryRing,
    pub history_bytes_in_per_sec: HistoryRing,
    pub history_bytes_out_per_sec: HistoryRing,
    pub history_entities_total: HistoryRing,

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
            system: SystemStats::default(),
            net: NetStats::default(),
            entities: EntityStats::default(),
            history_tps: HistoryRing::default(),
            history_players: HistoryRing::default(),
            history_chunks: HistoryRing::default(),
            history_mem_mb: HistoryRing::default(),
            history_cpu_percent: HistoryRing::default(),
            history_bytes_in_per_sec: HistoryRing::default(),
            history_bytes_out_per_sec: HistoryRing::default(),
            history_entities_total: HistoryRing::default(),
            boot_instant: None,
        }
    }
}

impl ServerSnapshot {
    pub fn uptime_seconds(&self) -> u64 {
        self.boot_instant
            .map(|inst| inst.elapsed().as_secs())
            .unwrap_or(0)
    }
}
