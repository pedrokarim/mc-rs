//! ContainerSetData (0x33) — Server → Client.
//!
//! Updates a single property of an open container (e.g. furnace cook progress,
//! fuel burn time). Used to animate the furnace UI progress bars.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::VarInt;

/// Updates a container property.
#[derive(Debug, Clone)]
pub struct ContainerSetData {
    /// The window ID of the container.
    pub window_id: u8,
    /// Property index (0=cook progress, 1=lit time, 2=lit duration).
    pub property: i32,
    /// Property value.
    pub value: i32,
}

impl ProtoEncode for ContainerSetData {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.window_id);
        VarInt(self.property).proto_encode(buf);
        VarInt(self.value).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_container_set_data() {
        let pkt = ContainerSetData {
            window_id: 5,
            property: 0,
            value: 100,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // u8(5) + VarInt(0) + VarInt(100)
        assert!(buf.len() >= 3);
        assert_eq!(buf[0], 5);
    }
}
