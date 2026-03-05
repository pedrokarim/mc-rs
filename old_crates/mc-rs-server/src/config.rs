use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub server: ServerSection,
    pub world: WorldSection,
    pub logging: LoggingSection,
    #[serde(default)]
    pub permissions: PermissionsSection,
    #[serde(default)]
    pub packs: PacksSection,
    #[serde(default)]
    pub rcon: RconSection,
    #[serde(default)]
    pub query: QuerySection,
}

#[derive(Debug, Default, Deserialize)]
pub struct PermissionsSection {
    #[serde(default)]
    pub whitelist_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct ServerSection {
    pub address: String,
    pub port: u16,
    pub motd: String,
    pub max_players: u32,
    pub gamemode: String,
    pub difficulty: String,
    pub online_mode: bool,
}

#[derive(Debug, Deserialize)]
pub struct WorldSection {
    pub name: String,
    pub generator: String,
    pub seed: i64,
    /// Auto-save interval in seconds. 0 = disabled. Default: 300 (5 minutes).
    #[serde(default = "default_auto_save_interval")]
    pub auto_save_interval: u64,
}

fn default_auto_save_interval() -> u64 {
    300
}

#[derive(Debug, Deserialize)]
pub struct PacksSection {
    #[serde(default = "default_packs_directory")]
    pub directory: String,
    #[serde(default)]
    pub force_packs: bool,
}

fn default_packs_directory() -> String {
    "packs".into()
}

impl Default for PacksSection {
    fn default() -> Self {
        Self {
            directory: default_packs_directory(),
            force_packs: false,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoggingSection {
    pub level: String,
}

#[derive(Debug, Deserialize)]
pub struct RconSection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rcon_port")]
    pub port: u16,
    #[serde(default)]
    pub password: String,
}

fn default_rcon_port() -> u16 {
    25575
}

impl Default for RconSection {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_rcon_port(),
            password: String::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct QuerySection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_query_port")]
    pub port: u16,
}

fn default_query_port() -> u16 {
    19132
}

impl Default for QuerySection {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_query_port(),
        }
    }
}

impl ServerConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config() {
        let toml_str = r#"
            [server]
            address = "0.0.0.0"
            port = 19132
            motd = "Test Server"
            max_players = 20
            gamemode = "survival"
            difficulty = "normal"
            online_mode = true

            [world]
            name = "world"
            generator = "flat"
            seed = 12345

            [logging]
            level = "debug"
        "#;
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.port, 19132);
        assert_eq!(config.server.motd, "Test Server");
        assert_eq!(config.server.max_players, 20);
        assert!(config.server.online_mode);
        assert_eq!(config.world.name, "world");
        assert_eq!(config.world.generator, "flat");
        assert_eq!(config.world.seed, 12345);
        assert_eq!(config.world.auto_save_interval, 300); // default
        assert_eq!(config.logging.level, "debug");
        // permissions section defaults when absent
        assert!(!config.permissions.whitelist_enabled);
        // packs section defaults when absent
        assert_eq!(config.packs.directory, "packs");
        assert!(!config.packs.force_packs);
        // rcon section defaults when absent
        assert!(!config.rcon.enabled);
        assert_eq!(config.rcon.port, 25575);
        assert!(config.rcon.password.is_empty());
        // query section defaults when absent
        assert!(!config.query.enabled);
        assert_eq!(config.query.port, 19132);
    }

    #[test]
    fn parse_config_with_permissions() {
        let toml_str = r#"
            [server]
            address = "0.0.0.0"
            port = 19132
            motd = "Test"
            max_players = 20
            gamemode = "survival"
            difficulty = "normal"
            online_mode = false

            [world]
            name = "world"
            generator = "flat"
            seed = 0

            [logging]
            level = "info"

            [permissions]
            whitelist_enabled = true
        "#;
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert!(config.permissions.whitelist_enabled);
    }
}
