use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub server: ServerSection,
    pub world: WorldSection,
    pub logging: LoggingSection,
    #[serde(default)]
    pub permissions: PermissionsSection,
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
}

#[derive(Debug, Deserialize)]
pub struct LoggingSection {
    pub level: String,
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
        assert_eq!(config.logging.level, "debug");
        // permissions section defaults when absent
        assert!(!config.permissions.whitelist_enabled);
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
