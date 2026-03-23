use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

/// Default server.toml content with comments explaining each option.
const DEFAULT_CONFIG_TOML: &str = r#"# ══════════════════════════════════════════════════
#         MC-RS Server Configuration
# ══════════════════════════════════════════════════

[server]
# Message affiché dans la liste des serveurs
motd = "MC-RS Server"

# Sous-titre du serveur (deuxième ligne dans la liste)
sub_motd = "Powered by Rust"

# Port UDP d'écoute
port = 19132

# Nombre maximum de joueurs simultanés
max_players = 20

# Authentification Xbox Live (true = online, false = cracké)
online_mode = false

# Distance de vue maximale en chunks (2-32)
# Le client peut demander moins, mais jamais plus
view_distance = 8

# Intervalle de tick serveur en millisecondes (10 = 100 TPS)
tick_rate = 10

[world]
# Nom du dossier monde (stocké dans worlds/<name>/)
name = "world"

# Générateur de terrain : "normal" ou "flat"
generator = "normal"

# Seed du monde (0 = aléatoire). Même seed = même terrain.
seed = 0

[gameplay]
# Mode de jeu par défaut : "survival", "creative", "adventure", "spectator"
gamemode = "creative"

# Difficulté : "peaceful", "easy", "normal", "hard"
difficulty = "normal"

# Activer le PvP entre joueurs
pvp = true

# Activer le cycle jour/nuit (journées de 20 minutes)
do_daylight_cycle = true

# Activer les changements de météo (pluie, orage)
do_weather_cycle = false

# Rayon de protection du spawn en blocs (0 = désactivé)
spawn_protection = 16
"#;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_server")]
    pub server: ServerSection,
    #[serde(default)]
    pub world: WorldSection,
    #[serde(default)]
    pub gameplay: GameplaySection,
}

#[derive(Debug, Deserialize)]
pub struct ServerSection {
    #[serde(default = "default_motd")]
    pub motd: String,
    #[serde(default = "default_sub_motd")]
    pub sub_motd: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_max_players")]
    pub max_players: u32,
    #[serde(default)]
    pub online_mode: bool,
    #[serde(default = "default_view_distance")]
    pub view_distance: i32,
    #[serde(default = "default_tick_rate")]
    pub tick_rate: u64,
}

#[derive(Debug, Deserialize)]
pub struct WorldSection {
    #[serde(default = "default_world_name")]
    pub name: String,
    #[serde(default = "default_generator")]
    pub generator: String,
    #[serde(default)]
    pub seed: i64,
}

#[derive(Debug, Deserialize)]
pub struct GameplaySection {
    #[serde(default = "default_gamemode")]
    pub gamemode: String,
    #[serde(default = "default_difficulty")]
    pub difficulty: String,
    #[serde(default = "default_true")]
    pub pvp: bool,
    #[serde(default = "default_true")]
    pub do_daylight_cycle: bool,
    #[serde(default)]
    pub do_weather_cycle: bool,
    #[serde(default = "default_spawn_protection")]
    pub spawn_protection: i32,
}

/// Subset of config values needed by each Connection.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub default_gamemode: i32,
    pub difficulty: i32,
    pub world_name: String,
    pub max_view_distance: i32,
    pub generator_id: i32, // 1=infinite, 2=flat for StartGame
    pub world_seed: u64,
}

fn default_server() -> ServerSection {
    ServerSection {
        motd: default_motd(),
        sub_motd: default_sub_motd(),
        port: default_port(),
        max_players: default_max_players(),
        online_mode: false,
        view_distance: default_view_distance(),
        tick_rate: default_tick_rate(),
    }
}

fn default_motd() -> String {
    "MC-RS Server".to_string()
}
fn default_sub_motd() -> String {
    "Powered by Rust".to_string()
}
fn default_port() -> u16 {
    19132
}
fn default_max_players() -> u32 {
    20
}
fn default_view_distance() -> i32 {
    8
}
fn default_tick_rate() -> u64 {
    10
} // ms per tick (100 TPS)
fn default_world_name() -> String {
    "world".to_string()
}
fn default_generator() -> String {
    "normal".to_string()
}
fn default_gamemode() -> String {
    "creative".to_string()
}
fn default_difficulty() -> String {
    "normal".to_string()
}
fn default_true() -> bool {
    true
}
fn default_spawn_protection() -> i32 {
    16
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: default_server(),
            world: WorldSection::default(),
            gameplay: GameplaySection::default(),
        }
    }
}

impl Default for WorldSection {
    fn default() -> Self {
        Self {
            name: default_world_name(),
            generator: default_generator(),
            seed: 0,
        }
    }
}

impl Default for GameplaySection {
    fn default() -> Self {
        Self {
            gamemode: default_gamemode(),
            difficulty: default_difficulty(),
            pvp: true,
            do_daylight_cycle: true,
            do_weather_cycle: false,
            spawn_protection: default_spawn_protection(),
        }
    }
}

impl GameplaySection {
    pub fn gamemode_id(&self) -> i32 {
        match self.gamemode.to_lowercase().as_str() {
            "survival" | "0" => 0,
            "creative" | "1" => 1,
            "adventure" | "2" => 2,
            "spectator" | "3" => 3,
            _ => 1, // default creative
        }
    }

    pub fn gamemode_display(&self) -> &str {
        match self.gamemode.to_lowercase().as_str() {
            "survival" | "0" => "Survival",
            "creative" | "1" => "Creative",
            "adventure" | "2" => "Adventure",
            "spectator" | "3" => "Spectator",
            _ => "Creative",
        }
    }

    pub fn difficulty_id(&self) -> i32 {
        match self.difficulty.to_lowercase().as_str() {
            "peaceful" | "0" => 0,
            "easy" | "1" => 1,
            "normal" | "2" => 2,
            "hard" | "3" => 3,
            _ => 2, // default normal
        }
    }
}

impl ServerConfig {
    pub fn load(path: &str) -> Self {
        // Auto-generate default config if missing
        if !std::path::Path::new(path).exists() {
            if let Err(e) = std::fs::write(path, DEFAULT_CONFIG_TOML) {
                info!("Could not write default {}: {}", path, e);
            } else {
                info!("Generated default {}", path);
            }
        }

        match std::fs::read_to_string(path) {
            Ok(content) => {
                let config: Self = toml::from_str(&content).unwrap_or_default();
                info!(
                    "Config loaded from {}\n  Server: {}:{} (max {} players)\n  World: \"{}\" ({}, seed={})\n  Gameplay: {}, {}, daylight={}, weather={}",
                    path,
                    config.server.motd,
                    config.server.port,
                    config.server.max_players,
                    config.world.name,
                    config.world.generator,
                    config.world.seed,
                    config.gameplay.gamemode,
                    config.gameplay.difficulty,
                    if config.gameplay.do_daylight_cycle { "on" } else { "off" },
                    if config.gameplay.do_weather_cycle { "on" } else { "off" },
                );
                config
            }
            Err(e) => {
                info!("Could not read {}: {}, using defaults", path, e);
                Self::default()
            }
        }
    }

    /// Build a ConnectionConfig from the full server config.
    pub fn connection_config(&self) -> Arc<ConnectionConfig> {
        Arc::new(ConnectionConfig {
            default_gamemode: self.gameplay.gamemode_id(),
            difficulty: self.gameplay.difficulty_id(),
            world_name: self.world.name.clone(),
            max_view_distance: self.server.view_distance,
            generator_id: match self.world.generator.to_lowercase().as_str() {
                "normal" => 1, // infinite
                "flat" => 2,   // flat
                _ => 2,
            },
            world_seed: self.world.seed as u64,
        })
    }
}
