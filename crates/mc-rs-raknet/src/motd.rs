/// MOTD configuration for the server list.
///
/// Format from PocketMine (RakLibInterface.php):
/// MCPE;name;protocol;version;online;max;guid;world_name;gamemode;
///
/// Note: No port fields! The client gets the port from the UDP source.
#[derive(Debug, Clone)]
pub struct Motd {
    pub name: String,
    pub protocol_version: u32,
    pub version_string: String,
    pub online_players: u32,
    pub max_players: u32,
    pub server_guid: i64,
    pub world_name: String,
    pub gamemode: String,
}

impl Motd {
    pub fn to_string_payload(&self) -> String {
        format!(
            "MCPE;{};{};{};{};{};{};{};{};",
            self.name,
            self.protocol_version,
            self.version_string,
            self.online_players,
            self.max_players,
            self.server_guid,
            self.world_name,
            self.gamemode,
        )
    }
}

impl Default for Motd {
    fn default() -> Self {
        Self {
            name: "MC-RS Server".to_string(),
            protocol_version: 975,
            version_string: "1.26.20".to_string(),
            online_players: 0,
            max_players: 20,
            server_guid: 0,
            world_name: "world".to_string(),
            gamemode: "Survival".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motd_format() {
        let motd = Motd {
            name: "Test Server".to_string(),
            protocol_version: 975,
            version_string: "1.26.20".to_string(),
            online_players: 0,
            max_players: 20,
            server_guid: 12345,
            world_name: "world".to_string(),
            gamemode: "Survival".to_string(),
        };
        let s = motd.to_string_payload();
        assert_eq!(s, "MCPE;Test Server;975;1.26.20;0;20;12345;world;Survival;");
        // Count semicolons: should be 9 (8 separators + trailing)
        assert_eq!(s.chars().filter(|&c| c == ';').count(), 9);
    }
}
