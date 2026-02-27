//! BlockActorData (0x38) — Bidirectional.
//!
//! Synchronizes block entity NBT data between server and client.
//! Used for signs (text), chests, furnaces, etc.

use bytes::{Buf, BufMut};

use crate::codec::{ProtoDecode, ProtoEncode};
use crate::error::ProtoError;
use crate::types::BlockPos;

/// Block entity data packet carrying raw network NBT.
#[derive(Debug, Clone)]
pub struct BlockActorData {
    /// Position of the block entity.
    pub position: BlockPos,
    /// Raw network NBT bytes.
    pub nbt_data: Vec<u8>,
}

impl ProtoEncode for BlockActorData {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        self.position.proto_encode(buf);
        buf.put_slice(&self.nbt_data);
    }
}

impl ProtoDecode for BlockActorData {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let position = BlockPos::proto_decode(buf)?;
        let mut nbt_data = vec![0u8; buf.remaining()];
        buf.copy_to_slice(&mut nbt_data);
        Ok(Self { position, nbt_data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_block_actor_data() {
        let pkt = BlockActorData {
            position: BlockPos::new(10, 64, -5),
            nbt_data: vec![0x0A, 0x00, 0x00], // empty NBT compound
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // BlockPos (3 VarInts) + 3 bytes NBT
        assert!(buf.len() >= 6);
        // Last 3 bytes should be the NBT
        let len = buf.len();
        assert_eq!(&buf[len - 3..], &[0x0A, 0x00, 0x00]);
    }

    #[test]
    fn decode_block_actor_data() {
        // Encode then decode roundtrip
        let original = BlockActorData {
            position: BlockPos::new(0, 64, 0),
            nbt_data: vec![0x0A, 0x00, 0x00],
        };
        let mut buf = BytesMut::new();
        original.proto_encode(&mut buf);

        let decoded = BlockActorData::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(decoded.position, original.position);
        assert_eq!(decoded.nbt_data, original.nbt_data);
    }
}
