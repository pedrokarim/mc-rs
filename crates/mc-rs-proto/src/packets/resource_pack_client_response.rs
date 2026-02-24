//! ResourcePackClientResponse (0x08) — Client → Server.

use bytes::Buf;

use crate::codec::{self, ProtoDecode};
use crate::error::ProtoError;
use crate::types::VarUInt32;

/// Status sent by the client in ResourcePackClientResponse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourcePackResponseStatus {
    None = 0,
    Refused = 1,
    SendPacks = 2,
    HaveAllPacks = 3,
    Completed = 4,
}

impl ResourcePackResponseStatus {
    fn from_u8(v: u8) -> Result<Self, ProtoError> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::Refused),
            2 => Ok(Self::SendPacks),
            3 => Ok(Self::HaveAllPacks),
            4 => Ok(Self::Completed),
            other => Err(ProtoError::InvalidLogin(format!(
                "unknown ResourcePackResponseStatus: {other}"
            ))),
        }
    }
}

/// The client's response to resource pack info or stack.
#[derive(Debug, Clone)]
pub struct ResourcePackClientResponse {
    pub status: ResourcePackResponseStatus,
    pub resource_pack_ids: Vec<String>,
}

impl ProtoDecode for ResourcePackClientResponse {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if !buf.has_remaining() {
            return Err(ProtoError::BufferTooShort {
                needed: 1,
                remaining: 0,
            });
        }
        let status = ResourcePackResponseStatus::from_u8(buf.get_u8())?;

        let count = VarUInt32::proto_decode(buf)?.0 as usize;
        let mut resource_pack_ids = Vec::with_capacity(count.min(64));
        for _ in 0..count {
            resource_pack_ids.push(codec::read_string(buf)?);
        }

        Ok(Self {
            status,
            resource_pack_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{BufMut, BytesMut};

    #[test]
    fn decode_have_all_packs() {
        let mut buf = BytesMut::new();
        buf.put_u8(3); // HaveAllPacks
        buf.put_u8(0); // 0 pack IDs
        let pkt = ResourcePackClientResponse::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(pkt.status, ResourcePackResponseStatus::HaveAllPacks);
        assert!(pkt.resource_pack_ids.is_empty());
    }

    #[test]
    fn decode_completed() {
        let mut buf = BytesMut::new();
        buf.put_u8(4); // Completed
        buf.put_u8(0); // 0 pack IDs
        let pkt = ResourcePackClientResponse::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(pkt.status, ResourcePackResponseStatus::Completed);
        assert!(pkt.resource_pack_ids.is_empty());
    }

    #[test]
    fn decode_with_pack_ids() {
        let mut buf = BytesMut::new();
        buf.put_u8(2); // SendPacks
        buf.put_u8(1); // 1 pack ID
                       // VarUInt32 string length = 4, then "test"
        buf.put_u8(4);
        buf.put_slice(b"test");
        let pkt = ResourcePackClientResponse::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(pkt.status, ResourcePackResponseStatus::SendPacks);
        assert_eq!(pkt.resource_pack_ids, vec!["test"]);
    }

    #[test]
    fn decode_unknown_status() {
        let mut buf = BytesMut::new();
        buf.put_u8(99);
        buf.put_u8(0);
        assert!(ResourcePackClientResponse::proto_decode(&mut buf.freeze().as_ref()).is_err());
    }
}
