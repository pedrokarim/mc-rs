//! ItemStack type and NetworkItemStackDescriptor serialization.
//!
//! Represents an item in a player's inventory or on the ground.
//! The wire format matches Bedrock's `NetworkItemStackDescriptor`.

use bytes::{Buf, BufMut};

use crate::codec::{read_string, write_string, ProtoDecode, ProtoEncode};
use crate::error::ProtoError;
use crate::types::{VarInt, VarUInt32};

/// A single item stack in the Bedrock protocol.
///
/// `runtime_id == 0` means the slot is empty (air).
#[derive(Debug, Clone)]
pub struct ItemStack {
    /// Item runtime ID from the StartGame item table. 0 = air/empty.
    pub runtime_id: i32,
    /// Number of items in this stack (1-255 in practice).
    pub count: u16,
    /// Item damage/variant metadata.
    pub metadata: u16,
    /// Block runtime ID if this item represents a placeable block.
    pub block_runtime_id: i32,
    /// Raw NBT data in network format (if any). Used for enchantments, custom names, etc.
    pub nbt_data: Vec<u8>,
    /// Blocks this item can be placed on (adventure mode).
    pub can_place_on: Vec<String>,
    /// Blocks this item can destroy (adventure mode).
    pub can_destroy: Vec<String>,
    /// Server-assigned unique ID for inventory tracking. 0 = no ID.
    pub stack_network_id: i32,
}

impl ItemStack {
    /// An empty slot (air).
    pub fn empty() -> Self {
        Self {
            runtime_id: 0,
            count: 0,
            metadata: 0,
            block_runtime_id: 0,
            nbt_data: Vec::new(),
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
            stack_network_id: 0,
        }
    }

    /// Create a simple item stack with no NBT or special data.
    pub fn new(runtime_id: i32, count: u16) -> Self {
        Self {
            runtime_id,
            count,
            metadata: 0,
            block_runtime_id: 0,
            nbt_data: Vec::new(),
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
            stack_network_id: 0,
        }
    }

    /// Create an item stack with metadata and a stack network ID.
    pub fn new_with_meta(
        runtime_id: i32,
        count: u16,
        metadata: u16,
        stack_network_id: i32,
    ) -> Self {
        Self {
            runtime_id,
            count,
            metadata,
            block_runtime_id: 0,
            nbt_data: Vec::new(),
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
            stack_network_id,
        }
    }

    /// Whether this slot is empty.
    pub fn is_empty(&self) -> bool {
        self.runtime_id == 0 || self.count == 0
    }
}

/// Encode as `NetworkItemStackDescriptor`.
///
/// Wire format:
/// ```text
/// VarInt(runtime_id)  — 0 = empty, return early
/// u16_le(count)
/// VarUInt32(metadata)
/// u8(has_stack_id) + optional VarInt(stack_network_id)
/// VarInt(block_runtime_id)
/// VarUInt32(user_data_marker) + optional NBT
/// VarInt(can_place_on_count) + strings
/// VarInt(can_destroy_count) + strings
/// ```
impl ProtoEncode for ItemStack {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarInt(self.runtime_id).proto_encode(buf);
        if self.runtime_id == 0 {
            return;
        }

        buf.put_u16_le(self.count);
        VarUInt32(self.metadata as u32).proto_encode(buf);

        // HasStackID + optional StackNetworkID
        if self.stack_network_id != 0 {
            buf.put_u8(1);
            VarInt(self.stack_network_id).proto_encode(buf);
        } else {
            buf.put_u8(0);
        }

        VarInt(self.block_runtime_id).proto_encode(buf);

        // User data section
        if self.nbt_data.is_empty() {
            VarUInt32(0).proto_encode(buf); // No user data
        } else {
            // 0xFFFFFFFF marker = network NBT follows
            VarUInt32(0xFFFF_FFFF).proto_encode(buf);
            buf.put_u8(1); // NBT version = 1 (network format)
            buf.put_slice(&self.nbt_data);
        }

        // CanPlaceOn
        VarInt(self.can_place_on.len() as i32).proto_encode(buf);
        for s in &self.can_place_on {
            write_string(buf, s);
        }

        // CanDestroy
        VarInt(self.can_destroy.len() as i32).proto_encode(buf);
        for s in &self.can_destroy {
            write_string(buf, s);
        }
    }
}

/// Decode from `NetworkItemStackDescriptor`.
impl ProtoDecode for ItemStack {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let runtime_id = VarInt::proto_decode(buf)?.0;
        if runtime_id == 0 {
            return Ok(Self::empty());
        }

        if buf.remaining() < 2 {
            return Err(ProtoError::BufferTooShort {
                needed: 2,
                remaining: buf.remaining(),
            });
        }
        let count = buf.get_u16_le();
        let metadata = VarUInt32::proto_decode(buf)?.0 as u16;

        // HasStackID + optional StackNetworkID
        if buf.remaining() < 1 {
            return Err(ProtoError::BufferTooShort {
                needed: 1,
                remaining: buf.remaining(),
            });
        }
        let has_stack_id = buf.get_u8() != 0;
        let stack_network_id = if has_stack_id {
            VarInt::proto_decode(buf)?.0
        } else {
            0
        };

        let block_runtime_id = VarInt::proto_decode(buf)?.0;

        // User data
        let nbt_data = decode_user_data(buf)?;

        // CanPlaceOn
        let can_place_on_count = VarInt::proto_decode(buf)?.0;
        let mut can_place_on = Vec::with_capacity(can_place_on_count as usize);
        for _ in 0..can_place_on_count {
            can_place_on.push(read_string(buf)?);
        }

        // CanDestroy
        let can_destroy_count = VarInt::proto_decode(buf)?.0;
        let mut can_destroy = Vec::with_capacity(can_destroy_count as usize);
        for _ in 0..can_destroy_count {
            can_destroy.push(read_string(buf)?);
        }

        Ok(Self {
            runtime_id,
            count,
            metadata,
            block_runtime_id,
            nbt_data,
            can_place_on,
            can_destroy,
            stack_network_id,
        })
    }
}

/// Decode the user data section, returning raw NBT bytes (if any).
fn decode_user_data(buf: &mut impl Buf) -> Result<Vec<u8>, ProtoError> {
    let marker = VarUInt32::proto_decode(buf)?.0;
    if marker == 0xFFFF_FFFF {
        // Network NBT with version byte
        if buf.remaining() < 1 {
            return Err(ProtoError::BufferTooShort {
                needed: 1,
                remaining: buf.remaining(),
            });
        }
        let version = buf.get_u8();
        if version == 1 {
            // Read network NBT compound and capture the raw bytes.
            // We read the compound tag and its contents into a buffer.
            let start_remaining = buf.remaining();
            skip_nbt_compound_network(buf)?;
            let bytes_consumed = start_remaining - buf.remaining();

            // Re-read those bytes — we need to re-wind and capture.
            // Since Buf doesn't support seeking, we use a different approach:
            // For the basique implementation, return empty and skip.
            // Full NBT extraction will be added when mc-rs-nbt is integrated.
            let _ = bytes_consumed;
            Ok(Vec::new())
        } else {
            Ok(Vec::new())
        }
    } else if marker > 0 {
        // Raw bytes — skip them
        if buf.remaining() < marker as usize {
            return Err(ProtoError::BufferTooShort {
                needed: marker as usize,
                remaining: buf.remaining(),
            });
        }
        buf.advance(marker as usize);
        Ok(Vec::new())
    } else {
        Ok(Vec::new())
    }
}

/// Skip a network-format NBT compound tag without extracting data.
fn skip_nbt_compound_network(buf: &mut impl Buf) -> Result<(), ProtoError> {
    // Network NBT compound: read tag types until TAG_End (0x00)
    loop {
        if buf.remaining() < 1 {
            return Err(ProtoError::BufferTooShort {
                needed: 1,
                remaining: buf.remaining(),
            });
        }
        let tag_type = buf.get_u8();
        if tag_type == 0 {
            return Ok(()); // TAG_End
        }
        // Read name (VarUInt32 length + UTF-8)
        let name_len = VarUInt32::proto_decode(buf)?.0 as usize;
        if buf.remaining() < name_len {
            return Err(ProtoError::BufferTooShort {
                needed: name_len,
                remaining: buf.remaining(),
            });
        }
        buf.advance(name_len);
        // Skip tag payload
        skip_nbt_tag_network(buf, tag_type)?;
    }
}

/// Skip a single NBT tag payload (network format) based on its type ID.
fn skip_nbt_tag_network(buf: &mut impl Buf, tag_type: u8) -> Result<(), ProtoError> {
    match tag_type {
        1 => {
            // Byte
            ensure_remaining(buf, 1)?;
            buf.advance(1);
        }
        2 => {
            // Short
            ensure_remaining(buf, 2)?;
            buf.advance(2);
        }
        3 => {
            // Int (VarInt in network format)
            let _ = VarInt::proto_decode(buf)?;
        }
        4 => {
            // Long (VarLong in network format)
            let _ = crate::types::VarLong::proto_decode(buf)?;
        }
        5 => {
            // Float
            ensure_remaining(buf, 4)?;
            buf.advance(4);
        }
        6 => {
            // Double
            ensure_remaining(buf, 8)?;
            buf.advance(8);
        }
        7 => {
            // ByteArray (VarInt length + bytes)
            let len = VarInt::proto_decode(buf)?.0 as usize;
            ensure_remaining(buf, len)?;
            buf.advance(len);
        }
        8 => {
            // String (VarUInt32 length + UTF-8)
            let len = VarUInt32::proto_decode(buf)?.0 as usize;
            ensure_remaining(buf, len)?;
            buf.advance(len);
        }
        9 => {
            // List (tag_type byte + VarInt length + elements)
            ensure_remaining(buf, 1)?;
            let element_type = buf.get_u8();
            let count = VarInt::proto_decode(buf)?.0;
            for _ in 0..count {
                skip_nbt_tag_network(buf, element_type)?;
            }
        }
        10 => {
            // Compound
            skip_nbt_compound_network(buf)?;
        }
        11 => {
            // IntArray (VarInt length + VarInt elements)
            let count = VarInt::proto_decode(buf)?.0;
            for _ in 0..count {
                let _ = VarInt::proto_decode(buf)?;
            }
        }
        12 => {
            // LongArray (VarInt length + VarLong elements)
            let count = VarInt::proto_decode(buf)?.0;
            for _ in 0..count {
                let _ = crate::types::VarLong::proto_decode(buf)?;
            }
        }
        _ => {
            return Err(ProtoError::InvalidData(format!(
                "unknown NBT tag type {}",
                tag_type
            )));
        }
    }
    Ok(())
}

fn ensure_remaining(buf: &impl Buf, needed: usize) -> Result<(), ProtoError> {
    if buf.remaining() < needed {
        Err(ProtoError::BufferTooShort {
            needed,
            remaining: buf.remaining(),
        })
    } else {
        Ok(())
    }
}

/// Encode an ItemStack for use inside inventory content packets.
///
/// Unlike the NetworkItemStackDescriptor (used in creative content),
/// `InventoryContentPacket` uses a slightly different encoding where
/// the full_container_name is written with a `VarUInt32(0)` dynamic_container_id.
pub fn write_item_instance(buf: &mut impl BufMut, item: &ItemStack) {
    item.proto_encode(buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn empty_item_roundtrip() {
        let item = ItemStack::empty();
        let mut buf = BytesMut::new();
        item.proto_encode(&mut buf);
        // Empty = just VarInt(0)
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], 0);

        let decoded = ItemStack::proto_decode(&mut buf.freeze()).unwrap();
        assert!(decoded.is_empty());
        assert_eq!(decoded.runtime_id, 0);
    }

    #[test]
    fn simple_item_encode() {
        let item = ItemStack {
            runtime_id: 1, // stone
            count: 64,
            metadata: 0,
            block_runtime_id: 0,
            nbt_data: Vec::new(),
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
            stack_network_id: 0,
        };
        let mut buf = BytesMut::new();
        item.proto_encode(&mut buf);
        // VarInt(1) + u16(64) + VarUInt32(0) + u8(0) + VarInt(0) + VarUInt32(0) + VarInt(0) + VarInt(0)
        assert!(buf.len() > 1);
    }

    #[test]
    fn item_with_stack_id_encode() {
        let item = ItemStack {
            runtime_id: 1,
            count: 32,
            metadata: 0,
            block_runtime_id: 0,
            nbt_data: Vec::new(),
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
            stack_network_id: 42,
        };
        let mut buf = BytesMut::new();
        item.proto_encode(&mut buf);

        let decoded = ItemStack::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.runtime_id, 1);
        assert_eq!(decoded.count, 32);
        assert_eq!(decoded.stack_network_id, 42);
    }

    #[test]
    fn simple_item_roundtrip() {
        let item = ItemStack {
            runtime_id: 3, // dirt
            count: 16,
            metadata: 0,
            block_runtime_id: 123,
            nbt_data: Vec::new(),
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
            stack_network_id: 7,
        };
        let mut buf = BytesMut::new();
        item.proto_encode(&mut buf);
        let decoded = ItemStack::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.runtime_id, 3);
        assert_eq!(decoded.count, 16);
        assert_eq!(decoded.metadata, 0);
        assert_eq!(decoded.block_runtime_id, 123);
        assert_eq!(decoded.stack_network_id, 7);
        assert!(decoded.nbt_data.is_empty());
        assert!(decoded.can_place_on.is_empty());
        assert!(decoded.can_destroy.is_empty());
    }

    #[test]
    fn is_empty_checks() {
        assert!(ItemStack::empty().is_empty());
        assert!(ItemStack::new(0, 10).is_empty());
        assert!(ItemStack::new(1, 0).is_empty());
        assert!(!ItemStack::new(1, 1).is_empty());
    }

    #[test]
    fn new_constructor() {
        let item = ItemStack::new(5, 32);
        assert_eq!(item.runtime_id, 5);
        assert_eq!(item.count, 32);
        assert_eq!(item.metadata, 0);
        assert_eq!(item.block_runtime_id, 0);
        assert_eq!(item.stack_network_id, 0);
    }
}
