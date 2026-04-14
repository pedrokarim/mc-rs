//! Server-side settings (equivalent of server.properties).

#[derive(Debug, Clone)]
pub struct ServerSettings {
    pub motd: String,
    pub port: u16,
    pub max_players: u32,
    pub view_distance: u32,
    pub simulation_distance: u32,
    pub white_list: bool,
    pub enforce_whitelist: bool,
    pub online_mode: bool,
    pub pvp: bool,
    pub difficulty: u8,
    pub gamemode: u8,
    pub allow_nether: bool,
    pub allow_end: bool,
    pub allow_flight: bool,
    pub force_gamemode: bool,
    pub spawn_protection_radius: u32,
    pub max_world_size: u64,
    pub network_compression_threshold: i32,
    pub rcon_port: Option<u16>,
    pub rcon_password: Option<String>,
    pub server_name: String,
    pub tick_rate: u32,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            motd: "A Minecraft Bedrock Server".into(),
            port: 19132,
            max_players: 20,
            view_distance: 10,
            simulation_distance: 10,
            white_list: false,
            enforce_whitelist: false,
            online_mode: true,
            pvp: true,
            difficulty: 2,
            gamemode: 0,
            allow_nether: true,
            allow_end: true,
            allow_flight: false,
            force_gamemode: false,
            spawn_protection_radius: 16,
            max_world_size: 29_999_984,
            network_compression_threshold: 256,
            rcon_port: None,
            rcon_password: None,
            server_name: "mc-rs server".into(),
            tick_rate: 20,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_port_19132() {
        assert_eq!(ServerSettings::default().port, 19132);
    }
}
