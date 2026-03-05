pub mod available_actor_identifiers;
pub mod available_commands;
pub mod biome_definition_list;
pub mod chunk_radius_updated;
pub mod crafting_data;
pub mod creative_content;
pub mod item_registry;
pub mod level_chunk;
pub mod move_player;
pub mod network_chunk_publisher_update;
pub mod network_settings;
pub mod play_status;
pub mod player_list;
pub mod resource_pack_stack;
pub mod resource_packs_info;
pub mod set_actor_data;
pub mod set_difficulty;
pub mod set_player_game_type;
pub mod set_spawn_position;
pub mod set_time;
pub mod start_game;
pub mod update_abilities;
pub mod update_adventure_settings;
pub mod update_attributes;

use crate::codec::write_unsigned_varint32;
use bytes::BytesMut;

// Packet IDs (from BedrockProtocol)
pub const ID_LOGIN: u32 = 0x01;
pub const ID_PLAY_STATUS: u32 = 0x02;
pub const ID_SERVER_TO_CLIENT_HANDSHAKE: u32 = 0x03;
pub const ID_CLIENT_TO_SERVER_HANDSHAKE: u32 = 0x04;
pub const ID_DISCONNECT: u32 = 0x05;
pub const ID_RESOURCE_PACKS_INFO: u32 = 0x06;
pub const ID_RESOURCE_PACK_STACK: u32 = 0x07;
pub const ID_RESOURCE_PACK_CLIENT_RESPONSE: u32 = 0x08;
pub const ID_SET_TIME: u32 = 0x0A;
pub const ID_START_GAME: u32 = 0x0B;
pub const ID_MOVE_PLAYER: u32 = 0x13;
pub const ID_UPDATE_ATTRIBUTES: u32 = 0x1D;
pub const ID_SET_ACTOR_DATA: u32 = 0x27;
pub const ID_SET_SPAWN_POSITION: u32 = 0x2B;
pub const ID_CRAFTING_DATA: u32 = 0x34;
pub const ID_LEVEL_CHUNK: u32 = 0x3A;
pub const ID_SET_DIFFICULTY: u32 = 0x3C;
pub const ID_SET_PLAYER_GAME_TYPE: u32 = 0x3E;
pub const ID_PLAYER_LIST: u32 = 0x3F;
pub const ID_REQUEST_CHUNK_RADIUS: u32 = 0x45;
pub const ID_CHUNK_RADIUS_UPDATED: u32 = 0x46;
pub const ID_AVAILABLE_COMMANDS: u32 = 0x4C;
pub const ID_SET_LOCAL_PLAYER_AS_INITIALIZED: u32 = 0x71;
pub const ID_AVAILABLE_ACTOR_IDENTIFIERS: u32 = 0x77;
pub const ID_NETWORK_CHUNK_PUBLISHER_UPDATE: u32 = 0x79;
pub const ID_BIOME_DEFINITION_LIST: u32 = 0x7A;
pub const ID_CLIENT_CACHE_STATUS: u32 = 0x81;
pub const ID_NETWORK_SETTINGS: u32 = 0x8F;
pub const ID_CREATIVE_CONTENT: u32 = 0x91;
pub const ID_ITEM_REGISTRY: u32 = 0xA2;
pub const ID_UPDATE_ABILITIES: u32 = 0xBB;
pub const ID_UPDATE_ADVENTURE_SETTINGS: u32 = 0xBC;
pub const ID_REQUEST_NETWORK_SETTINGS: u32 = 0xC1;
pub const ID_SERVERBOUND_LOADING_SCREEN: u32 = 0x138;

/// Encode a game packet: [varuint(total_len)][varuint(packet_id)][body]
/// The header VarUInt32 contains: packet_id in bits 0-9, sub-client IDs in bits 10-13.
/// For normal server packets, sub-client IDs are 0, so it's just the packet ID.
pub fn frame_packet(packet_id: u32, body: &[u8]) -> BytesMut {
    let mut header_buf = BytesMut::new();
    write_unsigned_varint32(&mut header_buf, packet_id);

    let total_len = header_buf.len() + body.len();
    let mut frame = BytesMut::new();
    write_unsigned_varint32(&mut frame, total_len as u32);
    frame.extend_from_slice(&header_buf);
    frame.extend_from_slice(body);
    frame
}
