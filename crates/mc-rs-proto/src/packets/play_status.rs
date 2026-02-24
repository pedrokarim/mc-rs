//! PlayStatus (0x02) — Server → Client.

use bytes::{Buf, BufMut};

use crate::codec::{ProtoDecode, ProtoEncode};
use crate::error::ProtoError;

/// Status codes for the PlayStatus packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PlayStatusType {
    /// Login accepted.
    LoginSuccess = 0,
    /// Client is too old (needs update).
    FailedClient = 1,
    /// Server is too old.
    FailedServer = 2,
    /// Player can spawn into the world.
    PlayerSpawn = 3,
    /// Invalid tenant.
    FailedInvalidTenant = 4,
    /// Vanilla Edu mismatch.
    FailedVanillaEdu = 5,
    /// Server/client incompatible.
    FailedIncompatible = 6,
    /// Server is full.
    FailedServerFull = 7,
}

/// Sent by the server to indicate login result or player spawn readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayStatus {
    pub status: PlayStatusType,
}

impl ProtoEncode for PlayStatus {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_i32(self.status as i32); // BE
    }
}

impl ProtoDecode for PlayStatus {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if buf.remaining() < 4 {
            return Err(ProtoError::BufferTooShort {
                needed: 4,
                remaining: buf.remaining(),
            });
        }
        let status_code = buf.get_i32(); // BE
        let status = match status_code {
            0 => PlayStatusType::LoginSuccess,
            1 => PlayStatusType::FailedClient,
            2 => PlayStatusType::FailedServer,
            3 => PlayStatusType::PlayerSpawn,
            4 => PlayStatusType::FailedInvalidTenant,
            5 => PlayStatusType::FailedVanillaEdu,
            6 => PlayStatusType::FailedIncompatible,
            7 => PlayStatusType::FailedServerFull,
            _ => {
                return Err(ProtoError::InvalidLogin(format!(
                    "unknown PlayStatus code: {status_code}"
                )))
            }
        };
        Ok(Self { status })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_login_success() {
        let pkt = PlayStatus {
            status: PlayStatusType::LoginSuccess,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf.len(), 4);
        assert_eq!(&buf[..], &[0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn encode_failed_client() {
        let pkt = PlayStatus {
            status: PlayStatusType::FailedClient,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(&buf[..], &[0x00, 0x00, 0x00, 0x01]);
    }

    #[test]
    fn encode_server_full() {
        let pkt = PlayStatus {
            status: PlayStatusType::FailedServerFull,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(&buf[..], &[0x00, 0x00, 0x00, 0x07]);
    }

    #[test]
    fn roundtrip() {
        let pkt = PlayStatus {
            status: PlayStatusType::PlayerSpawn,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let decoded = PlayStatus::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded, pkt);
    }

    #[test]
    fn decode_buffer_too_short() {
        let data = bytes::Bytes::from_static(&[0x00, 0x00]);
        assert!(PlayStatus::proto_decode(&mut data.clone()).is_err());
    }
}
