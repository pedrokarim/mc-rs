//! Game packet definitions for Bedrock Edition.

pub mod available_commands;
pub mod available_entity_identifiers;
pub mod biome_definition_list;
pub mod chunk_radius_updated;
pub mod client_to_server_handshake;
pub mod command_output;
pub mod command_request;
pub mod creative_content;
pub mod disconnect;
pub mod inventory_transaction;
pub mod level_chunk;
pub mod level_event;
pub mod login;
pub mod move_player;
pub mod network_chunk_publisher_update;
pub mod network_settings;
pub mod play_status;
pub mod player_action;
pub mod player_auth_input;
pub mod request_chunk_radius;
pub mod request_network_settings;
pub mod resource_pack_client_response;
pub mod resource_pack_stack;
pub mod resource_packs_info;
pub mod server_to_client_handshake;
pub mod set_local_player_as_initialized;
pub mod start_game;
pub mod text;
pub mod update_block;

pub use available_commands::AvailableCommands;
pub use available_entity_identifiers::AvailableEntityIdentifiers;
pub use biome_definition_list::BiomeDefinitionList;
pub use chunk_radius_updated::ChunkRadiusUpdated;
pub use client_to_server_handshake::ClientToServerHandshake;
pub use command_output::CommandOutput;
pub use command_request::{CommandOrigin, CommandRequest};
pub use creative_content::CreativeContent;
pub use disconnect::Disconnect;
pub use inventory_transaction::{InventoryTransaction, UseItemAction, UseItemData};
pub use level_chunk::LevelChunk;
pub use level_event::LevelEvent;
pub use login::LoginPacket;
pub use move_player::{MoveMode, MovePlayer};
pub use network_chunk_publisher_update::NetworkChunkPublisherUpdate;
pub use network_settings::NetworkSettings;
pub use play_status::{PlayStatus, PlayStatusType};
pub use player_action::{PlayerAction, PlayerActionType};
pub use player_auth_input::PlayerAuthInput;
pub use request_chunk_radius::RequestChunkRadius;
pub use request_network_settings::RequestNetworkSettings;
pub use resource_pack_client_response::{ResourcePackClientResponse, ResourcePackResponseStatus};
pub use resource_pack_stack::ResourcePackStack;
pub use resource_packs_info::ResourcePacksInfo;
pub use server_to_client_handshake::ServerToClientHandshake;
pub use set_local_player_as_initialized::SetLocalPlayerAsInitialized;
pub use start_game::StartGame;
pub use text::{Text, TextType};
pub use update_block::UpdateBlock;

/// Game packet IDs.
pub mod id {
    pub const LOGIN: u32 = 0x01;
    pub const PLAY_STATUS: u32 = 0x02;
    pub const SERVER_TO_CLIENT_HANDSHAKE: u32 = 0x03;
    pub const CLIENT_TO_SERVER_HANDSHAKE: u32 = 0x04;
    pub const DISCONNECT: u32 = 0x05;
    pub const RESOURCE_PACKS_INFO: u32 = 0x06;
    pub const RESOURCE_PACK_STACK: u32 = 0x07;
    pub const RESOURCE_PACK_CLIENT_RESPONSE: u32 = 0x08;
    pub const TEXT: u32 = 0x09;
    pub const START_GAME: u32 = 0x0B;
    pub const MOVE_PLAYER: u32 = 0x13;
    pub const UPDATE_BLOCK: u32 = 0x15;
    pub const LEVEL_EVENT: u32 = 0x19;
    pub const INVENTORY_TRANSACTION: u32 = 0x1E;
    pub const PLAYER_ACTION: u32 = 0x24;
    pub const LEVEL_CHUNK: u32 = 0x3A;
    pub const REQUEST_CHUNK_RADIUS: u32 = 0x45;
    pub const CHUNK_RADIUS_UPDATED: u32 = 0x46;
    pub const AVAILABLE_COMMANDS: u32 = 0x4C;
    pub const COMMAND_REQUEST: u32 = 0x4D;
    pub const COMMAND_OUTPUT: u32 = 0x4F;
    pub const SET_LOCAL_PLAYER_AS_INITIALIZED: u32 = 0x71;
    pub const AVAILABLE_ENTITY_IDENTIFIERS: u32 = 0x78;
    pub const NETWORK_CHUNK_PUBLISHER_UPDATE: u32 = 0x7A;
    pub const BIOME_DEFINITION_LIST: u32 = 0x7B;
    pub const NETWORK_SETTINGS: u32 = 0x8F;
    pub const PLAYER_AUTH_INPUT: u32 = 0x90;
    pub const CREATIVE_CONTENT: u32 = 0x91;
    pub const REQUEST_NETWORK_SETTINGS: u32 = 0xC1;
}

/// Target protocol version (Minecraft Bedrock 1.21.50).
pub const PROTOCOL_VERSION: i32 = 766;
