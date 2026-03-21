use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_server")]
    pub server: ServerSection,
    #[serde(default)]
    pub world: WorldSection,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ServerSection {
    #[serde(default = "default_motd")]
    pub motd: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_max_players")]
    pub max_players: u32,
    #[serde(default)]
    pub online_mode: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WorldSection {
    #[serde(default = "default_world_name")]
    pub name: String,
    #[serde(default = "default_generator")]
    pub generator: String,
}

fn default_server() -> ServerSection {
    ServerSection {
        motd: default_motd(),
        port: default_port(),
        max_players: default_max_players(),
        online_mode: false,
    }
}

fn default_motd() -> String {
    "MC-RS Server".to_string()
}
fn default_port() -> u16 {
    19132
}
fn default_max_players() -> u32 {
    20
}
fn default_world_name() -> String {
    "world".to_string()
}
fn default_generator() -> String {
    "flat".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: default_server(),
            world: WorldSection::default(),
        }
    }
}

impl Default for WorldSection {
    fn default() -> Self {
        Self {
            name: default_world_name(),
            generator: default_generator(),
        }
    }
}

impl ServerConfig {
    pub fn load(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }
}
