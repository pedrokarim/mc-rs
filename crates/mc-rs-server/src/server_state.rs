use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use tracing::{info, warn};

const SERVER_STATE_FILE: &str = "server-state.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistentServerState {
    #[serde(default)]
    pub ops: BTreeSet<String>,
    #[serde(default)]
    pub banned_names: BTreeSet<String>,
    #[serde(default)]
    pub banned_ips: BTreeSet<String>,
    #[serde(default)]
    pub whitelist_enabled: bool,
    #[serde(default)]
    pub whitelist: BTreeSet<String>,
    #[serde(default)]
    pub world_spawn: Option<[f32; 3]>,
    pub default_gamemode: Option<i32>,
    pub difficulty: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ServerState {
    pub persistent: PersistentServerState,
    pub auto_save_enabled: bool,
    pub server_motd: String,
    pub world_name: String,
    pub world_seed: u64,
    pub max_players: u32,
}

impl ServerState {
    pub fn load(
        server_motd: String,
        world_name: String,
        world_seed: u64,
        max_players: u32,
    ) -> Self {
        let persistent = match fs::read_to_string(SERVER_STATE_FILE) {
            Ok(contents) => match serde_json::from_str::<PersistentServerState>(&contents) {
                Ok(state) => {
                    info!("Loaded persistent server state from {}", SERVER_STATE_FILE);
                    state
                }
                Err(error) => {
                    warn!(
                        "Failed to parse {}: {}. Using defaults.",
                        SERVER_STATE_FILE, error
                    );
                    PersistentServerState::default()
                }
            },
            Err(_) => PersistentServerState::default(),
        };

        Self {
            persistent,
            auto_save_enabled: true,
            server_motd,
            world_name,
            world_seed,
            max_players,
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(&self.persistent)?;
        fs::write(SERVER_STATE_FILE, json)?;
        Ok(())
    }

    pub fn effective_default_gamemode(&self, fallback: i32) -> i32 {
        self.persistent.default_gamemode.unwrap_or(fallback)
    }

    pub fn effective_difficulty(&self, fallback: i32) -> i32 {
        self.persistent.difficulty.unwrap_or(fallback)
    }

    pub fn is_op(&self, name: &str) -> bool {
        self.persistent.ops.contains(&normalize_name(name))
    }

    pub fn set_op(&mut self, name: &str, value: bool) {
        let key = normalize_name(name);
        if value {
            self.persistent.ops.insert(key);
        } else {
            self.persistent.ops.remove(&key);
        }
    }

    pub fn is_name_banned(&self, name: &str) -> bool {
        self.persistent.banned_names.contains(&normalize_name(name))
    }

    pub fn set_name_ban(&mut self, name: &str, value: bool) {
        let key = normalize_name(name);
        if value {
            self.persistent.banned_names.insert(key);
        } else {
            self.persistent.banned_names.remove(&key);
        }
    }

    pub fn is_ip_banned(&self, ip: &str) -> bool {
        self.persistent.banned_ips.contains(ip)
    }

    pub fn set_ip_ban(&mut self, ip: &str, value: bool) {
        if value {
            self.persistent.banned_ips.insert(ip.to_string());
        } else {
            self.persistent.banned_ips.remove(ip);
        }
    }

    pub fn whitelist_contains(&self, name: &str) -> bool {
        self.persistent.whitelist.contains(&normalize_name(name))
    }

    pub fn set_whitelist_entry(&mut self, name: &str, value: bool) {
        let key = normalize_name(name);
        if value {
            self.persistent.whitelist.insert(key);
        } else {
            self.persistent.whitelist.remove(&key);
        }
    }
}

pub fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}
