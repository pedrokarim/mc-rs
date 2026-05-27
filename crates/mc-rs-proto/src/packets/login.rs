use crate::io::{ProtoReader, ProtoWriter};

// ── RequestNetworkSettings (C→S, 0xC1) ──

/// Client sends this as the very first MCPE packet.
/// Layout: protocol_version (i32 BE)
pub struct RequestNetworkSettings {
    pub protocol_version: i32,
}

impl RequestNetworkSettings {
    pub fn decode(reader: &mut ProtoReader) -> Result<Self, crate::io::reader::ProtoReadError> {
        let protocol_version = reader.read_i32_be()?;
        Ok(Self { protocol_version })
    }
}

// ── NetworkSettings (S→C, 0x8F) ──

/// Server responds with compression settings.
pub struct NetworkSettings {
    pub compression_threshold: u16,
    pub compression_algorithm: u16,
    pub client_throttle_enabled: bool,
    pub client_throttle_threshold: u8,
    pub client_throttle_scalar: f32,
}

impl NetworkSettings {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = ProtoWriter::with_capacity(16);
        writer.write_u16_le(self.compression_threshold);
        writer.write_u16_le(self.compression_algorithm);
        writer.write_bool(self.client_throttle_enabled);
        writer.write_u8(self.client_throttle_threshold);
        writer.write_f32_le(self.client_throttle_scalar);
        writer.into_bytes()
    }

    /// Default settings matching PocketMine.
    pub fn default_settings() -> Self {
        Self {
            compression_threshold: 1, // compress almost everything
            compression_algorithm: 0, // zlib
            client_throttle_enabled: false,
            client_throttle_threshold: 0,
            client_throttle_scalar: 0.0,
        }
    }
}

// ── PlayStatus (S→C, 0x02) ──

#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum PlayStatusType {
    LoginSuccess = 0,
    LoginFailedClient = 1,
    LoginFailedServer = 2,
    PlayerSpawn = 3,
    LoginFailedInvalidTenant = 4,
    LoginFailedVanillaEdu = 5,
    LoginFailedEduVanilla = 6,
    LoginFailedServerFull = 7,
    LoginFailedEditorVanilla = 8,
    LoginFailedVanillaEditor = 9,
}

pub struct PlayStatus {
    pub status: PlayStatusType,
}

impl PlayStatus {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = ProtoWriter::with_capacity(4);
        writer.write_i32_be(self.status as i32);
        writer.into_bytes()
    }
}

// ── ServerToClientHandshake (S→C, 0x03) ──

pub struct ServerToClientHandshake {
    pub jwt: String,
}

impl ServerToClientHandshake {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = ProtoWriter::with_capacity(self.jwt.len() + 5);
        writer.write_string(&self.jwt);
        writer.into_bytes()
    }
}

// ── ClientToServerHandshake (C→S, 0x04) ──

/// Empty packet — just signals encryption readiness.
pub struct ClientToServerHandshake;

impl ClientToServerHandshake {
    pub fn decode(_reader: &mut ProtoReader) -> Result<Self, crate::io::reader::ProtoReadError> {
        Ok(Self)
    }
}

// ── Disconnect (S→C, 0x05) ──

pub struct Disconnect {
    pub reason: DisconnectReason,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum DisconnectReason {
    Unknown = 0,
    ServerShutdown = 1,
    Kicked = 2,
}

impl Disconnect {
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = ProtoWriter::with_capacity(32);
        writer.write_var_i32(self.reason as i32);
        match &self.message {
            Some(msg) => {
                writer.write_bool(false); // skip message = false
                writer.write_string(msg);
                writer.write_string(msg); // filtered message
            }
            None => {
                writer.write_bool(true); // skip message = true
            }
        }
        writer.into_bytes()
    }
}

// ── Login (C→S, 0x01) ──

/// The Login packet — contains JWT chain and client data.
pub struct Login {
    pub protocol_version: i32,
    pub chain_data: String,
    pub client_data_jwt: String,
}

impl Login {
    pub fn decode(reader: &mut ProtoReader) -> Result<Self, crate::io::reader::ProtoReadError> {
        let protocol_version = reader.read_i32_be()?;

        // The rest is a byte array containing the JWT chain + client data
        let payload = reader.read_byte_array()?;
        let mut payload_reader = ProtoReader::new(&payload);

        // Chain data: i32_le length + JSON string
        let chain_len = payload_reader.read_i32_le()? as usize;
        let chain_data = if chain_len > 0 && payload_reader.remaining() >= chain_len {
            let bytes = payload_reader.read_raw(chain_len)?;
            String::from_utf8(bytes).unwrap_or_default()
        } else {
            String::new()
        };

        // Client data: i32_le length + JWT string
        let client_len = payload_reader.read_i32_le()? as usize;
        let client_data_jwt = if client_len > 0 && payload_reader.remaining() >= client_len {
            let bytes = payload_reader.read_raw(client_len)?;
            String::from_utf8(bytes).unwrap_or_default()
        } else {
            String::new()
        };

        Ok(Self {
            protocol_version,
            chain_data,
            client_data_jwt,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_settings_encode() {
        let settings = NetworkSettings::default_settings();
        let encoded = settings.encode();
        let mut reader = ProtoReader::new(&encoded);
        assert_eq!(reader.read_u16_le().unwrap(), 1); // threshold
        assert_eq!(reader.read_u16_le().unwrap(), 0); // algorithm (zlib)
        assert!(!reader.read_bool().unwrap()); // throttle
    }

    #[test]
    fn test_play_status_encode() {
        let status = PlayStatus {
            status: PlayStatusType::LoginSuccess,
        };
        let encoded = status.encode();
        assert_eq!(encoded.len(), 4);
        let mut reader = ProtoReader::new(&encoded);
        assert_eq!(reader.read_i32_be().unwrap(), 0);
    }

    #[test]
    fn test_play_status_player_spawn() {
        let status = PlayStatus {
            status: PlayStatusType::PlayerSpawn,
        };
        let encoded = status.encode();
        let mut reader = ProtoReader::new(&encoded);
        assert_eq!(reader.read_i32_be().unwrap(), 3);
    }

    #[test]
    fn test_disconnect_with_message() {
        let pkt = Disconnect {
            reason: DisconnectReason::Kicked,
            message: Some("You have been kicked".to_string()),
        };
        let encoded = pkt.encode();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_disconnect_no_message() {
        let pkt = Disconnect {
            reason: DisconnectReason::ServerShutdown,
            message: None,
        };
        let encoded = pkt.encode();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_request_network_settings_decode() {
        let mut writer = ProtoWriter::new();
        writer.write_i32_be(924);
        let mut reader = ProtoReader::new(writer.as_bytes());
        let pkt = RequestNetworkSettings::decode(&mut reader).unwrap();
        assert_eq!(pkt.protocol_version, 924);
    }

    #[test]
    fn test_handshake_encode() {
        let pkt = ServerToClientHandshake {
            jwt: "eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzM4NCJ9.test.sig".to_string(),
        };
        let encoded = pkt.encode();
        let mut reader = ProtoReader::new(&encoded);
        let jwt = reader.read_string().unwrap();
        assert_eq!(jwt, "eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzM4NCJ9.test.sig");
    }
}
