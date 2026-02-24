//! InventoryTransaction (0x1E) — Client → Server.
//!
//! Sent when the player interacts with blocks or items.
//! We only parse TransactionType == 2 (UseItem); all others are ignored.

use bytes::Buf;

use crate::codec::{read_string, ProtoDecode};
use crate::error::ProtoError;
use crate::types::{BlockPos, VarInt, VarUInt32, VarUInt64, Vec3};

/// Action type within a UseItem transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseItemAction {
    /// Place a block (click on a block face).
    ClickBlock = 0,
    /// Right-click in the air.
    ClickAir = 1,
    /// Break a block.
    BreakBlock = 2,
}

impl UseItemAction {
    fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::ClickBlock),
            1 => Some(Self::ClickAir),
            2 => Some(Self::BreakBlock),
            _ => None,
        }
    }
}

/// Data from a UseItem transaction.
#[derive(Debug, Clone)]
pub struct UseItemData {
    pub action: UseItemAction,
    pub block_position: BlockPos,
    pub face: i32,
    pub hotbar_slot: i32,
    pub held_item_block_runtime_id: i32,
    pub player_position: Vec3,
    pub click_position: Vec3,
    pub block_runtime_id: u32,
}

/// Action type within a UseItemOnEntity transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseItemOnEntityAction {
    /// Right-click interact (e.g., trade with villager).
    Interact = 0,
    /// Left-click attack.
    Attack = 1,
}

/// Data from a UseItemOnEntity transaction (type 3).
#[derive(Debug, Clone)]
pub struct UseItemOnEntityData {
    pub entity_runtime_id: u64,
    pub action: UseItemOnEntityAction,
    pub hotbar_slot: i32,
    pub player_position: Vec3,
    pub click_position: Vec3,
}

/// Parsed InventoryTransaction.
#[derive(Debug, Clone)]
pub struct InventoryTransaction {
    pub use_item: Option<UseItemData>,
    pub use_item_on_entity: Option<UseItemOnEntityData>,
}

// ---------------------------------------------------------------------------
// NBT skip helper (network variant)
// ---------------------------------------------------------------------------

/// Skip a single NBT tag value (network variant: VarInt ints, VarUInt32 string lengths).
fn skip_nbt_tag(buf: &mut impl Buf, tag_type: u8) -> Result<(), ProtoError> {
    match tag_type {
        0 => {} // TAG_End — nothing
        1 => {
            // TAG_Byte
            ensure_remaining(buf, 1)?;
            buf.advance(1);
        }
        2 => {
            // TAG_Short
            ensure_remaining(buf, 2)?;
            buf.advance(2);
        }
        3 => {
            // TAG_Int (VarInt zigzag)
            let _ = VarInt::proto_decode(buf)?;
        }
        4 => {
            // TAG_Long
            ensure_remaining(buf, 8)?;
            buf.advance(8);
        }
        5 => {
            // TAG_Float
            ensure_remaining(buf, 4)?;
            buf.advance(4);
        }
        6 => {
            // TAG_Double
            ensure_remaining(buf, 8)?;
            buf.advance(8);
        }
        7 => {
            // TAG_ByteArray: VarInt(len) + bytes
            let len = VarInt::proto_decode(buf)?.0;
            if len > 0 {
                ensure_remaining(buf, len as usize)?;
                buf.advance(len as usize);
            }
        }
        8 => {
            // TAG_String: VarUInt32(len) + UTF-8
            let len = VarUInt32::proto_decode(buf)?.0 as usize;
            if len > 0 {
                ensure_remaining(buf, len)?;
                buf.advance(len);
            }
        }
        9 => {
            // TAG_List: element_type(u8) + VarInt(count) + elements
            ensure_remaining(buf, 1)?;
            let elem_type = buf.get_u8();
            let count = VarInt::proto_decode(buf)?.0;
            for _ in 0..count {
                skip_nbt_tag(buf, elem_type)?;
            }
        }
        10 => {
            // TAG_Compound: tags until TAG_End
            loop {
                ensure_remaining(buf, 1)?;
                let child_type = buf.get_u8();
                if child_type == 0 {
                    break; // TAG_End
                }
                // Skip name
                let name_len = VarUInt32::proto_decode(buf)?.0 as usize;
                if name_len > 0 {
                    ensure_remaining(buf, name_len)?;
                    buf.advance(name_len);
                }
                skip_nbt_tag(buf, child_type)?;
            }
        }
        11 => {
            // TAG_IntArray: VarInt(count) + count * VarInt
            let count = VarInt::proto_decode(buf)?.0;
            for _ in 0..count {
                let _ = VarInt::proto_decode(buf)?;
            }
        }
        12 => {
            // TAG_LongArray: VarInt(count) + count * 8 bytes
            let count = VarInt::proto_decode(buf)?.0;
            if count > 0 {
                ensure_remaining(buf, count as usize * 8)?;
                buf.advance(count as usize * 8);
            }
        }
        _ => {
            return Err(ProtoError::InvalidLogin(format!(
                "unknown NBT tag type: {tag_type}"
            )));
        }
    }
    Ok(())
}

/// Skip an entire network NBT root compound (TAG_Compound + name + children).
fn skip_nbt_network(buf: &mut impl Buf) -> Result<(), ProtoError> {
    ensure_remaining(buf, 1)?;
    let root_type = buf.get_u8();
    if root_type != 10 {
        return Err(ProtoError::InvalidLogin(format!(
            "expected NBT compound (10), got {root_type}"
        )));
    }
    // Root name
    let name_len = VarUInt32::proto_decode(buf)?.0 as usize;
    if name_len > 0 {
        ensure_remaining(buf, name_len)?;
        buf.advance(name_len);
    }
    // Children until TAG_End
    loop {
        ensure_remaining(buf, 1)?;
        let child_type = buf.get_u8();
        if child_type == 0 {
            break;
        }
        let child_name_len = VarUInt32::proto_decode(buf)?.0 as usize;
        if child_name_len > 0 {
            ensure_remaining(buf, child_name_len)?;
            buf.advance(child_name_len);
        }
        skip_nbt_tag(buf, child_type)?;
    }
    Ok(())
}

fn ensure_remaining(buf: &impl Buf, needed: usize) -> Result<(), ProtoError> {
    if buf.remaining() < needed {
        return Err(ProtoError::BufferTooShort {
            needed,
            remaining: buf.remaining(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Item stack descriptor helpers
// ---------------------------------------------------------------------------

/// Skip a NetworkInventoryAction from the buffer.
fn skip_inventory_action(buf: &mut impl Buf) -> Result<(), ProtoError> {
    let source_type = VarUInt32::proto_decode(buf)?.0;
    match source_type {
        0 => {
            // Container: WindowID (VarInt)
            let _ = VarInt::proto_decode(buf)?;
        }
        1 => {
            // Global: nothing extra
        }
        2 => {
            // WorldInteraction: Flags (VarUInt32)
            let _ = VarUInt32::proto_decode(buf)?;
        }
        3 => {
            // Creative: nothing extra
        }
        // 100+ are crafting types (ContainerID varint)
        _ if source_type >= 100 => {
            let _ = VarInt::proto_decode(buf)?;
        }
        _ => {
            // Unknown source type — skip nothing extra
        }
    }
    let _ = VarUInt32::proto_decode(buf)?; // Slot
    skip_item_stack_descriptor(buf)?; // OldItem
    skip_item_stack_descriptor(buf)?; // NewItem
    Ok(())
}

/// Skip a NetworkItemStackDescriptor without extracting data.
fn skip_item_stack_descriptor(buf: &mut impl Buf) -> Result<(), ProtoError> {
    let runtime_id = VarInt::proto_decode(buf)?.0;
    if runtime_id == 0 {
        return Ok(()); // empty slot
    }
    ensure_remaining(buf, 2)?;
    buf.advance(2); // Count (u16)
    let _ = VarUInt32::proto_decode(buf)?; // Metadata

    // HasStackID + optional StackID
    ensure_remaining(buf, 1)?;
    if buf.get_u8() != 0 {
        let _ = VarInt::proto_decode(buf)?; // StackNetworkID
    }

    let _ = VarInt::proto_decode(buf)?; // BlockRuntimeID

    skip_user_data(buf)?;

    // CanPlaceOn + CanDestroy
    for _ in 0..2 {
        let count = VarInt::proto_decode(buf)?.0;
        for _ in 0..count {
            let _ = read_string(buf)?;
        }
    }

    Ok(())
}

/// Read a NetworkItemStackDescriptor and extract the BlockRuntimeID.
fn read_item_block_runtime_id(buf: &mut impl Buf) -> Result<i32, ProtoError> {
    let runtime_id = VarInt::proto_decode(buf)?.0;
    if runtime_id == 0 {
        return Ok(0); // empty slot
    }
    ensure_remaining(buf, 2)?;
    buf.advance(2); // Count (u16)
    let _ = VarUInt32::proto_decode(buf)?; // Metadata

    // HasStackID + optional StackID
    ensure_remaining(buf, 1)?;
    if buf.get_u8() != 0 {
        let _ = VarInt::proto_decode(buf)?;
    }

    let block_runtime_id = VarInt::proto_decode(buf)?.0; // BlockRuntimeID

    skip_user_data(buf)?;

    // CanPlaceOn + CanDestroy
    for _ in 0..2 {
        let count = VarInt::proto_decode(buf)?.0;
        for _ in 0..count {
            let _ = read_string(buf)?;
        }
    }

    Ok(block_runtime_id)
}

/// Skip the user data section of a NetworkItemStackDescriptor.
fn skip_user_data(buf: &mut impl Buf) -> Result<(), ProtoError> {
    let marker = VarUInt32::proto_decode(buf)?.0;
    if marker == 0xFFFF_FFFF {
        // Network NBT with version byte
        ensure_remaining(buf, 1)?;
        let version = buf.get_u8();
        if version == 1 {
            skip_nbt_network(buf)?;
        }
    } else if marker > 0 {
        ensure_remaining(buf, marker as usize)?;
        buf.advance(marker as usize);
    }
    Ok(())
}

impl ProtoDecode for InventoryTransaction {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        // LegacyRequestID (VarInt zigzag) — should be 0
        let legacy_request_id = VarInt::proto_decode(buf)?.0;
        if legacy_request_id != 0 {
            // Has legacy slot info — skip it
            let legacy_count = VarUInt32::proto_decode(buf)?.0;
            for _ in 0..legacy_count {
                let _ = read_string(buf)?; // ContainerID string
                let change_count = VarUInt32::proto_decode(buf)?.0;
                for _ in 0..change_count {
                    buf.advance(1); // Slot
                    buf.advance(1); // OldItem (simplified)
                    buf.advance(1); // NewItem (simplified)
                                    // Note: this is simplified; real legacy slots are complex
                }
            }
        }

        let transaction_type = VarUInt32::proto_decode(buf)?.0;

        // Skip actions array
        let action_count = VarUInt32::proto_decode(buf)?.0;
        for _ in 0..action_count {
            skip_inventory_action(buf)?;
        }

        if transaction_type == 3 {
            // UseItemOnEntity
            let entity_runtime_id = VarUInt64::proto_decode(buf)?.0;
            let action_type_raw = VarUInt32::proto_decode(buf)?.0;
            let action = match action_type_raw {
                0 => UseItemOnEntityAction::Interact,
                1 => UseItemOnEntityAction::Attack,
                _ => {
                    return Ok(Self {
                        use_item: None,
                        use_item_on_entity: None,
                    })
                }
            };
            let hotbar_slot = VarInt::proto_decode(buf)?.0;
            skip_item_stack_descriptor(buf)?; // held item
            let player_position = Vec3::proto_decode(buf)?;
            let click_position = Vec3::proto_decode(buf)?;
            return Ok(Self {
                use_item: None,
                use_item_on_entity: Some(UseItemOnEntityData {
                    entity_runtime_id,
                    action,
                    hotbar_slot,
                    player_position,
                    click_position,
                }),
            });
        }

        if transaction_type != 2 {
            // Not UseItem or UseItemOnEntity — ignore
            return Ok(Self {
                use_item: None,
                use_item_on_entity: None,
            });
        }

        // Parse UseItem data
        let action_type_raw = VarUInt32::proto_decode(buf)?.0;
        let action = UseItemAction::from_u32(action_type_raw);

        let block_position = BlockPos::proto_decode(buf)?;
        let face = VarInt::proto_decode(buf)?.0;
        let hotbar_slot = VarInt::proto_decode(buf)?.0;
        let held_item_block_runtime_id = read_item_block_runtime_id(buf)?;
        let player_position = Vec3::proto_decode(buf)?;
        let click_position = Vec3::proto_decode(buf)?;
        let block_runtime_id = VarUInt32::proto_decode(buf)?.0;

        let action = match action {
            Some(a) => a,
            None => {
                return Ok(Self {
                    use_item: None,
                    use_item_on_entity: None,
                })
            }
        };

        Ok(Self {
            use_item: Some(UseItemData {
                action,
                block_position,
                face,
                hotbar_slot,
                held_item_block_runtime_id,
                player_position,
                click_position,
                block_runtime_id,
            }),
            use_item_on_entity: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::ProtoEncode;
    use bytes::BytesMut;

    /// Encode an empty item stack (runtime_id = 0).
    fn encode_empty_item(buf: &mut BytesMut) {
        VarInt(0).proto_encode(buf);
    }

    /// Encode a minimal item stack with a given block_runtime_id.
    fn encode_item_with_block_id(buf: &mut BytesMut, block_runtime_id: i32) {
        VarInt(1).proto_encode(buf); // runtime_id (non-zero = has item)
        buf.extend_from_slice(&1u16.to_le_bytes()); // Count
        VarUInt32(0).proto_encode(buf); // Metadata
        buf.extend_from_slice(&[0u8]); // HasStackID = false
        VarInt(block_runtime_id).proto_encode(buf); // BlockRuntimeID
        VarUInt32(0).proto_encode(buf); // UserData marker = 0 (no data)
        VarInt(0).proto_encode(buf); // CanPlaceOn count
        VarInt(0).proto_encode(buf); // CanDestroy count
    }

    fn encode_break_block(position: BlockPos) -> BytesMut {
        let mut buf = BytesMut::new();
        VarInt(0).proto_encode(&mut buf); // LegacyRequestID
        VarUInt32(2).proto_encode(&mut buf); // TransactionType = UseItem
        VarUInt32(0).proto_encode(&mut buf); // Actions count = 0
        VarUInt32(2).proto_encode(&mut buf); // ActionType = BreakBlock
        position.proto_encode(&mut buf); // BlockPosition
        VarInt(1).proto_encode(&mut buf); // Face (Up)
        VarInt(0).proto_encode(&mut buf); // HotbarSlot
        encode_empty_item(&mut buf); // HeldItem
        Vec3::ZERO.proto_encode(&mut buf); // PlayerPosition
        Vec3::ZERO.proto_encode(&mut buf); // ClickPosition
        VarUInt32(100).proto_encode(&mut buf); // BlockRuntimeID
        buf
    }

    fn encode_place_block(position: BlockPos, face: i32, block_runtime_id: i32) -> BytesMut {
        let mut buf = BytesMut::new();
        VarInt(0).proto_encode(&mut buf); // LegacyRequestID
        VarUInt32(2).proto_encode(&mut buf); // TransactionType = UseItem
        VarUInt32(0).proto_encode(&mut buf); // Actions count = 0
        VarUInt32(0).proto_encode(&mut buf); // ActionType = ClickBlock
        position.proto_encode(&mut buf); // BlockPosition
        VarInt(face).proto_encode(&mut buf); // Face
        VarInt(0).proto_encode(&mut buf); // HotbarSlot
        encode_item_with_block_id(&mut buf, block_runtime_id); // HeldItem
        Vec3::ZERO.proto_encode(&mut buf); // PlayerPosition
        Vec3::new(0.5, 1.0, 0.5).proto_encode(&mut buf); // ClickPosition
        VarUInt32(200).proto_encode(&mut buf); // BlockRuntimeID
        buf
    }

    #[test]
    fn decode_break_block() {
        let buf = encode_break_block(BlockPos::new(10, 3, -5));
        let pkt = InventoryTransaction::proto_decode(&mut buf.freeze()).unwrap();
        let data = pkt.use_item.expect("should be UseItem");
        assert_eq!(data.action, UseItemAction::BreakBlock);
        assert_eq!(data.block_position, BlockPos::new(10, 3, -5));
        assert_eq!(data.block_runtime_id, 100);
    }

    #[test]
    fn decode_place_block() {
        let buf = encode_place_block(BlockPos::new(5, 3, 5), 1, 42);
        let pkt = InventoryTransaction::proto_decode(&mut buf.freeze()).unwrap();
        let data = pkt.use_item.expect("should be UseItem");
        assert_eq!(data.action, UseItemAction::ClickBlock);
        assert_eq!(data.block_position, BlockPos::new(5, 3, 5));
        assert_eq!(data.face, 1);
        assert_eq!(data.held_item_block_runtime_id, 42);
    }

    #[test]
    fn skip_non_use_item() {
        let mut buf = BytesMut::new();
        VarInt(0).proto_encode(&mut buf); // LegacyRequestID
        VarUInt32(0).proto_encode(&mut buf); // TransactionType = Normal
        VarUInt32(0).proto_encode(&mut buf); // Actions count = 0
        let pkt = InventoryTransaction::proto_decode(&mut buf.freeze()).unwrap();
        assert!(pkt.use_item.is_none());
    }

    #[test]
    fn decode_with_creative_action() {
        // Simulate a UseItem transaction with one Creative source action
        let mut buf = BytesMut::new();
        VarInt(0).proto_encode(&mut buf); // LegacyRequestID
        VarUInt32(2).proto_encode(&mut buf); // TransactionType = UseItem

        // 1 action: Creative source
        VarUInt32(1).proto_encode(&mut buf); // Actions count = 1
        VarUInt32(3).proto_encode(&mut buf); // SourceType = Creative
        VarUInt32(0).proto_encode(&mut buf); // Slot
        encode_empty_item(&mut buf); // OldItem
        encode_empty_item(&mut buf); // NewItem

        // UseItem data
        VarUInt32(2).proto_encode(&mut buf); // ActionType = BreakBlock
        BlockPos::new(0, 3, 0).proto_encode(&mut buf);
        VarInt(1).proto_encode(&mut buf); // Face
        VarInt(0).proto_encode(&mut buf); // HotbarSlot
        encode_empty_item(&mut buf); // HeldItem
        Vec3::ZERO.proto_encode(&mut buf);
        Vec3::ZERO.proto_encode(&mut buf);
        VarUInt32(50).proto_encode(&mut buf); // BlockRuntimeID

        let pkt = InventoryTransaction::proto_decode(&mut buf.freeze()).unwrap();
        let data = pkt.use_item.expect("should be UseItem");
        assert_eq!(data.action, UseItemAction::BreakBlock);
        assert_eq!(data.block_runtime_id, 50);
    }

    fn encode_attack_entity(entity_runtime_id: u64, action: u32) -> BytesMut {
        let mut buf = BytesMut::new();
        VarInt(0).proto_encode(&mut buf); // LegacyRequestID
        VarUInt32(3).proto_encode(&mut buf); // TransactionType = UseItemOnEntity
        VarUInt32(0).proto_encode(&mut buf); // Actions count = 0
        VarUInt64(entity_runtime_id).proto_encode(&mut buf);
        VarUInt32(action).proto_encode(&mut buf); // ActionType
        VarInt(0).proto_encode(&mut buf); // HotbarSlot
        encode_empty_item(&mut buf); // HeldItem
        Vec3::new(1.0, 64.0, 1.0).proto_encode(&mut buf); // PlayerPosition
        Vec3::new(0.0, 1.0, 0.0).proto_encode(&mut buf); // ClickPosition
        buf
    }

    #[test]
    fn decode_attack_entity() {
        let buf = encode_attack_entity(42, 1);
        let pkt = InventoryTransaction::proto_decode(&mut buf.freeze()).unwrap();
        assert!(pkt.use_item.is_none());
        let data = pkt.use_item_on_entity.expect("should be UseItemOnEntity");
        assert_eq!(data.entity_runtime_id, 42);
        assert_eq!(data.action, UseItemOnEntityAction::Attack);
        assert_eq!(data.hotbar_slot, 0);
    }

    #[test]
    fn decode_interact_entity() {
        let buf = encode_attack_entity(10, 0);
        let pkt = InventoryTransaction::proto_decode(&mut buf.freeze()).unwrap();
        assert!(pkt.use_item.is_none());
        let data = pkt.use_item_on_entity.expect("should be UseItemOnEntity");
        assert_eq!(data.entity_runtime_id, 10);
        assert_eq!(data.action, UseItemOnEntityAction::Interact);
    }
}
