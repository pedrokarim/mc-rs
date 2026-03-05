use bytes::{Buf, BufMut, BytesMut};

use crate::address::RakNetAddress;
use crate::error::RakNetError;

/// Packet IDs for online (connected) packets.
pub mod id {
    pub const CONNECTED_PING: u8 = 0x00;
    pub const CONNECTED_PONG: u8 = 0x03;
    pub const CONNECTION_REQUEST: u8 = 0x09;
    pub const CONNECTION_REQUEST_ACCEPTED: u8 = 0x10;
    pub const NEW_INCOMING_CONNECTION: u8 = 0x13;
    pub const DISCONNECTION_NOTIFICATION: u8 = 0x15;
}

/// Number of system addresses in connection packets.
const NUM_SYSTEM_ADDRESSES: usize = 20;

#[derive(Debug)]
pub enum OnlinePacket {
    ConnectedPing {
        timestamp: i64,
    },
    ConnectedPong {
        ping_timestamp: i64,
        pong_timestamp: i64,
    },
    ConnectionRequest {
        client_guid: i64,
        timestamp: i64,
        use_security: bool,
    },
    ConnectionRequestAccepted {
        client_address: RakNetAddress,
        system_index: u16,
        system_addresses: [RakNetAddress; NUM_SYSTEM_ADDRESSES],
        request_timestamp: i64,
        accept_timestamp: i64,
    },
    NewIncomingConnection {
        server_address: RakNetAddress,
        system_addresses: [RakNetAddress; NUM_SYSTEM_ADDRESSES],
        request_timestamp: i64,
        accept_timestamp: i64,
    },
    DisconnectionNotification,
}

impl OnlinePacket {
    /// Decode an online packet from the raw frame body.
    pub fn decode(data: &[u8]) -> Result<Self, RakNetError> {
        if data.is_empty() {
            return Err(RakNetError::PacketTooShort {
                expected: 1,
                actual: 0,
            });
        }
        let mut buf = std::io::Cursor::new(data);
        let packet_id = buf.get_u8();

        match packet_id {
            id::CONNECTED_PING => {
                let timestamp = buf.get_i64();
                Ok(Self::ConnectedPing { timestamp })
            }
            id::CONNECTED_PONG => {
                let ping_timestamp = buf.get_i64();
                let pong_timestamp = buf.get_i64();
                Ok(Self::ConnectedPong {
                    ping_timestamp,
                    pong_timestamp,
                })
            }
            id::CONNECTION_REQUEST => {
                let client_guid = buf.get_i64();
                let timestamp = buf.get_i64();
                let use_security = buf.get_u8() != 0;
                Ok(Self::ConnectionRequest {
                    client_guid,
                    timestamp,
                    use_security,
                })
            }
            id::NEW_INCOMING_CONNECTION => {
                let server_address = RakNetAddress::decode(&mut buf)?;
                let mut system_addresses = [RakNetAddress::EMPTY_V4; NUM_SYSTEM_ADDRESSES];
                for addr in &mut system_addresses {
                    if buf.remaining() >= 7 {
                        *addr = RakNetAddress::decode(&mut buf)?;
                    }
                }
                let request_timestamp = if buf.remaining() >= 8 {
                    buf.get_i64()
                } else {
                    0
                };
                let accept_timestamp = if buf.remaining() >= 8 {
                    buf.get_i64()
                } else {
                    0
                };
                Ok(Self::NewIncomingConnection {
                    server_address,
                    system_addresses,
                    request_timestamp,
                    accept_timestamp,
                })
            }
            id::DISCONNECTION_NOTIFICATION => Ok(Self::DisconnectionNotification),
            _ => Err(RakNetError::UnknownPacketId(packet_id)),
        }
    }

    /// Encode an online packet into a buffer.
    pub fn encode(&self, buf: &mut BytesMut) {
        match self {
            Self::ConnectedPing { timestamp } => {
                buf.put_u8(id::CONNECTED_PING);
                buf.put_i64(*timestamp);
            }
            Self::ConnectedPong {
                ping_timestamp,
                pong_timestamp,
            } => {
                buf.put_u8(id::CONNECTED_PONG);
                buf.put_i64(*ping_timestamp);
                buf.put_i64(*pong_timestamp);
            }
            Self::ConnectionRequestAccepted {
                client_address,
                system_index,
                system_addresses,
                request_timestamp,
                accept_timestamp,
            } => {
                buf.put_u8(id::CONNECTION_REQUEST_ACCEPTED);
                client_address.encode(buf);
                buf.put_u16(*system_index);
                for addr in system_addresses {
                    addr.encode(buf);
                }
                buf.put_i64(*request_timestamp);
                buf.put_i64(*accept_timestamp);
            }
            Self::DisconnectionNotification => {
                buf.put_u8(id::DISCONNECTION_NOTIFICATION);
            }
            // Client-side packets
            Self::ConnectionRequest { .. } | Self::NewIncomingConnection { .. } => {
                unreachable!("server should not encode client online packets")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connected_ping_roundtrip() {
        let packet = OnlinePacket::ConnectedPing { timestamp: 42 };
        let mut buf = BytesMut::new();
        packet.encode(&mut buf);
        let decoded = OnlinePacket::decode(&buf).unwrap();
        match decoded {
            OnlinePacket::ConnectedPing { timestamp } => assert_eq!(timestamp, 42),
            _ => panic!("expected ConnectedPing"),
        }
    }

    #[test]
    fn connected_pong_roundtrip() {
        let packet = OnlinePacket::ConnectedPong {
            ping_timestamp: 10,
            pong_timestamp: 20,
        };
        let mut buf = BytesMut::new();
        packet.encode(&mut buf);
        let decoded = OnlinePacket::decode(&buf).unwrap();
        match decoded {
            OnlinePacket::ConnectedPong {
                ping_timestamp,
                pong_timestamp,
            } => {
                assert_eq!(ping_timestamp, 10);
                assert_eq!(pong_timestamp, 20);
            }
            _ => panic!("expected ConnectedPong"),
        }
    }

    #[test]
    fn disconnection_notification_roundtrip() {
        let packet = OnlinePacket::DisconnectionNotification;
        let mut buf = BytesMut::new();
        packet.encode(&mut buf);
        assert_eq!(buf.len(), 1);
        let decoded = OnlinePacket::decode(&buf).unwrap();
        assert!(matches!(decoded, OnlinePacket::DisconnectionNotification));
    }
}
