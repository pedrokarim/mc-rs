use serde::Deserialize;
use tracing::info;

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

fn default_motd() -> String { "MC-RS Server".to_string() }
fn default_sub_motd() -> String { "Powered by Rust".to_string() }
fn default_port() -> u16 { 19132 }
fn default_max_players() -> u32 { 20 }
fn default_view_distance() -> i32 { 8 }
fn default_tick_rate() -> u64 { 10 } // ms per tick (100 TPS)
fn default_world_name() -> String { "world".to_string() }
fn default_generator() -> String { "flat".to_string() }
fn default_gamemode() -> String { "creative".to_string() }
fn default_difficulty() -> String { "normal".to_string() }
fn default_true() -> bool { true }
fn default_spawn_protection() -> i32 { 16 }

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
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let config: Self = toml::from_str(&content).unwrap_or_default();
                config
            }
            Err(_) => {
                info!("No {} found, using defaults", path);
                Self::default()
            }
        }
    }
}
