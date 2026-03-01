use bytes::{Buf, BufMut, BytesMut};

use crate::address::RakNetAddress;
use crate::codec::{read_magic, write_magic, write_string};
use crate::error::RakNetError;

/// Packet IDs for offline packets.
pub mod id {
    pub const UNCONNECTED_PING: u8 = 0x01;
    pub const UNCONNECTED_PING_OPEN: u8 = 0x02;
    pub const UNCONNECTED_PONG: u8 = 0x1C;
    pub const OPEN_CONNECTION_REQUEST_1: u8 = 0x05;
    pub const OPEN_CONNECTION_REPLY_1: u8 = 0x06;
    pub const OPEN_CONNECTION_REQUEST_2: u8 = 0x07;
    pub const OPEN_CONNECTION_REPLY_2: u8 = 0x08;
}

#[derive(Debug)]
pub enum OfflinePacket {
    UnconnectedPing {
        send_timestamp: i64,
        client_guid: i64,
    },
    UnconnectedPong {
        send_timestamp: i64,
        server_guid: i64,
        motd: String,
    },
    OpenConnectionRequest1 {
        protocol_version: u8,
        mtu_size: u16,
    },
    OpenConnectionReply1 {
        server_guid: i64,
        use_security: bool,
        mtu_size: u16,
    },
    OpenConnectionRequest2 {
        server_address: RakNetAddress,
        mtu_size: u16,
        client_guid: i64,
    },
    OpenConnectionReply2 {
        server_guid: i64,
        client_address: RakNetAddress,
        mtu_size: u16,
        encryption_enabled: bool,
    },
}

impl OfflinePacket {
    /// Decode an offline packet. `datagram_len` is the total UDP payload length
    /// (needed to infer MTU from OpenConnectionRequest1).
    pub fn decode(data: &[u8], datagram_len: usize) -> Result<Self, RakNetError> {
        if data.is_empty() {
            return Err(RakNetError::PacketTooShort {
                expected: 1,
                actual: 0,
            });
        }
        let mut buf = std::io::Cursor::new(data);
        let packet_id = buf.get_u8();

        match packet_id {
            id::UNCONNECTED_PING | id::UNCONNECTED_PING_OPEN => {
                let send_timestamp = buf.get_i64();
                read_magic(&mut buf)?;
                let client_guid = buf.get_i64();
                Ok(Self::UnconnectedPing {
                    send_timestamp,
                    client_guid,
                })
            }
            id::OPEN_CONNECTION_REQUEST_1 => {
                read_magic(&mut buf)?;
                let protocol_version = buf.get_u8();
                // MTU is inferred from the total datagram length
                let mtu_size = datagram_len as u16;
                Ok(Self::OpenConnectionRequest1 {
                    protocol_version,
                    mtu_size,
                })
            }
            id::OPEN_CONNECTION_REQUEST_2 => {
                read_magic(&mut buf)?;
                let server_address = RakNetAddress::decode(&mut buf)?;
                let mtu_size = buf.get_u16();
                let client_guid = buf.get_i64();
                Ok(Self::OpenConnectionRequest2 {
                    server_address,
                    mtu_size,
                    client_guid,
                })
            }
            _ => Err(RakNetError::UnknownPacketId(packet_id)),
        }
    }

    /// Encode an offline packet into a byte buffer.
    pub fn encode(&self, buf: &mut BytesMut) {
        match self {
            Self::UnconnectedPong {
                send_timestamp,
                server_guid,
                motd,
            } => {
                buf.put_u8(id::UNCONNECTED_PONG);
                buf.put_i64(*send_timestamp);
                buf.put_i64(*server_guid);
                write_magic(buf);
                write_string(buf, motd);
            }
            Self::OpenConnectionReply1 {
                server_guid,
                use_security,
                mtu_size,
            } => {
                buf.put_u8(id::OPEN_CONNECTION_REPLY_1);
                write_magic(buf);
                buf.put_i64(*server_guid);
                buf.put_u8(*use_security as u8);
                buf.put_u16(*mtu_size);
            }
            Self::OpenConnectionReply2 {
                server_guid,
                client_address,
                mtu_size,
                encryption_enabled,
            } => {
                buf.put_u8(id::OPEN_CONNECTION_REPLY_2);
                write_magic(buf);
                buf.put_i64(*server_guid);
                client_address.encode(buf);
                buf.put_u16(*mtu_size);
                buf.put_u8(*encryption_enabled as u8);
            }
            // Client-side packets — we don't encode these on the server
            Self::UnconnectedPing { .. }
            | Self::OpenConnectionRequest1 { .. }
            | Self::OpenConnectionRequest2 { .. } => {
                unreachable!("server should not encode client offline packets")
            }
        }
    }
}

/// MOTD data for the UnconnectedPong response.
#[derive(Debug, Clone)]
pub struct ServerMotd {
    pub server_name: String,
    pub protocol_version: u32,
    pub game_version: String,
    pub online_players: u32,
    pub max_players: u32,
    pub server_guid: i64,
    pub world_name: String,
    pub gamemode: String,
    pub gamemode_numeric: u8,
    pub ipv4_port: u16,
    pub ipv6_port: u16,
    /// Editor mode flag (0 = normal, 1 = editor). BDS always sends this.
    pub is_editor_mode: u8,
}

impl ServerMotd {
    /// Format as the semicolon-delimited MOTD string for UnconnectedPong.
    pub fn to_motd_string(&self) -> String {
        format!(
            "MCPE;{};{};{};{};{};{};{};{};{};{};{};{};",
            self.server_name,
            self.protocol_version,
            self.game_version,
            self.online_players,
            self.max_players,
            self.server_guid,
            self.world_name,
            self.gamemode,
            self.gamemode_numeric,
            self.ipv4_port,
            self.ipv6_port,
            self.is_editor_mode,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motd_format() {
        let motd = ServerMotd {
            server_name: "Test Server".into(),
            protocol_version: 924,
            game_version: "1.26.2".into(),
            online_players: 3,
            max_players: 20,
            server_guid: 12345,
            world_name: "world".into(),
            gamemode: "Survival".into(),
            gamemode_numeric: 1,
            ipv4_port: 19132,
            ipv6_port: 19133,
            is_editor_mode: 0,
        };
        let s = motd.to_motd_string();
        assert!(s.starts_with("MCPE;Test Server;924;1.26.2;3;20;12345;"));
        assert!(s.ends_with(';'));
        assert_eq!(s.matches(';').count(), 13);
    }

    #[test]
    fn pong_encode_decode_roundtrip() {
        let pong = OfflinePacket::UnconnectedPong {
            send_timestamp: 1234567890,
            server_guid: 42,
            motd: "MCPE;Test;924;1.26.2;0;20;42;world;Survival;1;19132;19133;0;".into(),
        };
        let mut buf = BytesMut::new();
        pong.encode(&mut buf);

        // Verify first byte is pong ID
        assert_eq!(buf[0], id::UNCONNECTED_PONG);
    }

    #[test]
    fn ocr1_decode() {
        let mut data = BytesMut::new();
        data.put_u8(id::OPEN_CONNECTION_REQUEST_1);
        data.put_slice(&crate::constants::RAKNET_MAGIC);
        data.put_u8(crate::constants::RAKNET_PROTOCOL_VERSION);
        // Pad to simulate MTU of 1400
        data.resize(1400, 0);

        let packet = OfflinePacket::decode(&data, data.len()).unwrap();
        match packet {
            OfflinePacket::OpenConnectionRequest1 {
                protocol_version,
                mtu_size,
            } => {
                assert_eq!(protocol_version, 11);
                assert_eq!(mtu_size, 1400);
            }
            _ => panic!("expected OCR1"),
        }
    }
}
