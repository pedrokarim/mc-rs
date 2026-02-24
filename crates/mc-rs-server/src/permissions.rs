//! Permission management: ops, whitelist, bans.
//!
//! Persists data as JSON files in the current working directory.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// A ban entry with a reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanEntry {
    pub reason: String,
}

/// Manages operators, whitelist, and ban lists with JSON persistence.
pub struct PermissionManager {
    /// Display names of operators.
    pub ops: HashSet<String>,
    /// Display names of whitelisted players.
    pub whitelist: HashSet<String>,
    /// Banned players: display_name → ban entry.
    pub banned_players: HashMap<String, BanEntry>,
    /// Banned IPs: IP string → ban entry.
    pub banned_ips: HashMap<String, BanEntry>,
    /// Whether the whitelist is currently enforced (runtime toggle).
    pub whitelist_enabled: bool,
}

const OPS_FILE: &str = "ops.json";
const WHITELIST_FILE: &str = "whitelist.json";
const BANNED_PLAYERS_FILE: &str = "banned-players.json";
const BANNED_IPS_FILE: &str = "banned-ips.json";

impl PermissionManager {
    /// Load all permission data from JSON files. Creates empty defaults if files don't exist.
    pub fn load(whitelist_enabled: bool) -> Self {
        Self {
            ops: load_set(OPS_FILE),
            whitelist: load_set(WHITELIST_FILE),
            banned_players: load_map(BANNED_PLAYERS_FILE),
            banned_ips: load_map(BANNED_IPS_FILE),
            whitelist_enabled,
        }
    }

    /// Save the ops list to disk.
    pub fn save_ops(&self) {
        save_set(OPS_FILE, &self.ops);
    }

    /// Save the whitelist to disk.
    pub fn save_whitelist(&self) {
        save_set(WHITELIST_FILE, &self.whitelist);
    }

    /// Save the banned players list to disk.
    pub fn save_banned_players(&self) {
        save_map(BANNED_PLAYERS_FILE, &self.banned_players);
    }

    /// Save the banned IPs list to disk.
    pub fn save_banned_ips(&self) {
        save_map(BANNED_IPS_FILE, &self.banned_ips);
    }
}

/// Load a HashSet<String> from a JSON array file.
fn load_set(path: &str) -> HashSet<String> {
    if !Path::new(path).exists() {
        return HashSet::new();
    }
    match fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str::<Vec<String>>(&contents) {
            Ok(vec) => {
                info!("Loaded {} entries from {path}", vec.len());
                vec.into_iter().collect()
            }
            Err(e) => {
                warn!("Failed to parse {path}: {e}");
                HashSet::new()
            }
        },
        Err(e) => {
            warn!("Failed to read {path}: {e}");
            HashSet::new()
        }
    }
}

/// Load a HashMap<String, BanEntry> from a JSON object file.
fn load_map(path: &str) -> HashMap<String, BanEntry> {
    if !Path::new(path).exists() {
        return HashMap::new();
    }
    match fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(map) => {
                let map: HashMap<String, BanEntry> = map;
                info!("Loaded {} entries from {path}", map.len());
                map
            }
            Err(e) => {
                warn!("Failed to parse {path}: {e}");
                HashMap::new()
            }
        },
        Err(e) => {
            warn!("Failed to read {path}: {e}");
            HashMap::new()
        }
    }
}

/// Save a HashSet<String> as a sorted JSON array.
fn save_set(path: &str, set: &HashSet<String>) {
    let mut sorted: Vec<&String> = set.iter().collect();
    sorted.sort();
    match serde_json::to_string_pretty(&sorted) {
        Ok(json) => {
            if let Err(e) = fs::write(path, json) {
                warn!("Failed to write {path}: {e}");
            }
        }
        Err(e) => warn!("Failed to serialize {path}: {e}"),
    }
}

/// Save a HashMap<String, BanEntry> as a JSON object.
fn save_map(path: &str, map: &HashMap<String, BanEntry>) {
    match serde_json::to_string_pretty(map) {
        Ok(json) => {
            if let Err(e) = fs::write(path, json) {
                warn!("Failed to write {path}: {e}");
            }
        }
        Err(e) => warn!("Failed to serialize {path}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // set_current_dir is process-global, so we must serialize tests.
    static DIR_LOCK: Mutex<()> = Mutex::new(());

    /// Run a test in a temporary directory so we don't pollute the project.
    fn in_temp_dir<F: FnOnce()>(f: F) {
        let _guard = DIR_LOCK.lock().unwrap();
        let tmp = env::temp_dir().join(format!("mc_rs_perm_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let _ = fs::create_dir_all(&tmp);
        let prev = env::current_dir().unwrap();
        env::set_current_dir(&tmp).unwrap();
        f();
        env::set_current_dir(prev).unwrap();
        let _ = fs::remove_dir_all(tmp);
    }

    #[test]
    fn load_empty_creates_defaults() {
        in_temp_dir(|| {
            let pm = PermissionManager::load(false);
            assert!(pm.ops.is_empty());
            assert!(pm.whitelist.is_empty());
            assert!(pm.banned_players.is_empty());
            assert!(pm.banned_ips.is_empty());
        });
    }

    #[test]
    fn save_and_reload_ops() {
        in_temp_dir(|| {
            let mut pm = PermissionManager::load(false);
            pm.ops.insert("Steve".into());
            pm.ops.insert("Alex".into());
            pm.save_ops();

            let pm2 = PermissionManager::load(false);
            assert_eq!(pm2.ops.len(), 2);
            assert!(pm2.ops.contains("Steve"));
            assert!(pm2.ops.contains("Alex"));
        });
    }

    #[test]
    fn save_and_reload_whitelist() {
        in_temp_dir(|| {
            let mut pm = PermissionManager::load(false);
            pm.whitelist.insert("Bob".into());
            pm.save_whitelist();

            let pm2 = PermissionManager::load(false);
            assert_eq!(pm2.whitelist.len(), 1);
            assert!(pm2.whitelist.contains("Bob"));
        });
    }

    #[test]
    fn save_and_reload_bans() {
        in_temp_dir(|| {
            let mut pm = PermissionManager::load(false);
            pm.banned_players.insert(
                "Hacker".into(),
                BanEntry {
                    reason: "Cheating".into(),
                },
            );
            pm.banned_ips.insert(
                "10.0.0.1".into(),
                BanEntry {
                    reason: "Spam".into(),
                },
            );
            pm.save_banned_players();
            pm.save_banned_ips();

            let pm2 = PermissionManager::load(false);
            assert_eq!(pm2.banned_players.len(), 1);
            assert_eq!(pm2.banned_players["Hacker"].reason, "Cheating");
            assert_eq!(pm2.banned_ips.len(), 1);
            assert_eq!(pm2.banned_ips["10.0.0.1"].reason, "Spam");
        });
    }
}
