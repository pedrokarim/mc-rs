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

/// Sous-ensemble de `DisconnectFailReason` (schéma Mojang 2169,
/// `__protocoldoc.json`) : 1 et 2 y désignent « pas d'Internet » et « pas de
/// permission », pas l'arrêt ni l'expulsion.
#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum DisconnectReason {
    Unknown = 0,
    Kicked = 55,
    ServerShutdown = 68,
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

// ── ResourcePackClientResponse (C→S, 0x08) ──

/// Réponse du client pendant la négociation des packs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourcePackClientResponse {
    /// Le client refuse les packs (il se déconnecte).
    Cancel,
    /// Le client demande le téléchargement des packs listés (`<uuid>_<version>`).
    Downloading(Vec<String>),
    /// Téléchargements terminés : le serveur envoie ResourcePackStack.
    DownloadingFinished,
    /// Pile appliquée : le serveur passe à la phase de spawn.
    ResourcePackStackFinished,
}

impl ResourcePackClientResponse {
    /// Protocole 2168 : variante taguée. Index VarUInt32 (0 cancel,
    /// 1 downloading, 2 downloadingfinished, 3 resourcepackstackfinished), nom
    /// de la réponse en chaîne, puis la liste des packs pour « downloading »
    /// uniquement. La numérotation a glissé d'un cran par rapport à l'octet de
    /// statut 1001 (1 refused … 4 completed).
    /// Réf : gophertunnel `packet/resource_pack_client_response.go`,
    /// CloudburstProtocol `ResourcePackClientResponseSerializer_v2168`, schéma
    /// `ResourcePackClientResponsePacketPayload.json`.
    pub fn decode(reader: &mut ProtoReader) -> Result<Self, crate::io::reader::ProtoReadError> {
        let index = reader.read_var_u32()?;
        let _name = reader.read_string()?;
        Ok(match index {
            0 => Self::Cancel,
            1 => {
                let count = reader.read_var_u32()?;
                if count > 65_535 {
                    return Err(crate::io::reader::ProtoReadError::LimitExceeded {
                        what: "resource pack response packs",
                        value: u64::from(count),
                        max: 65_535,
                    });
                }
                let mut packs = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    packs.push(reader.read_string()?);
                }
                Self::Downloading(packs)
            }
            2 => Self::DownloadingFinished,
            3 => Self::ResourcePackStackFinished,
            other => {
                return Err(crate::io::reader::ProtoReadError::InvalidValue {
                    what: "resource pack client response",
                    value: i64::from(other),
                })
            }
        })
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
    fn resource_pack_client_response_decodes_tagged_variants() {
        let mut w = ProtoWriter::new();
        w.write_var_u32(1);
        w.write_string("downloading");
        w.write_var_u32(1);
        w.write_string("uuid_1.0.0");
        let bytes = w.into_bytes();
        assert_eq!(
            ResourcePackClientResponse::decode(&mut ProtoReader::new(&bytes)).unwrap(),
            ResourcePackClientResponse::Downloading(vec!["uuid_1.0.0".into()])
        );

        let mut w = ProtoWriter::new();
        w.write_var_u32(3);
        w.write_string("resourcepackstackfinished");
        let bytes = w.into_bytes();
        assert_eq!(
            ResourcePackClientResponse::decode(&mut ProtoReader::new(&bytes)).unwrap(),
            ResourcePackClientResponse::ResourcePackStackFinished
        );
    }

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
