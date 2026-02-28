//! Game packet definitions for Bedrock Edition.

pub mod add_actor;
pub mod add_item_entity;
pub mod add_player;
pub mod animate;
pub mod available_commands;
pub mod available_entity_identifiers;
pub mod biome_definition_list;
pub mod block_actor_data;
pub mod boss_event;
pub mod change_dimension;
pub mod chunk_radius_updated;
pub mod client_to_server_handshake;
pub mod command_output;
pub mod command_request;
pub mod container_close;
pub mod container_open;
pub mod container_set_data;
pub mod crafting_data;
pub mod creative_content;
pub mod disconnect;
pub mod entity_event;
pub mod game_rules_changed;
pub mod inventory_content;
pub mod inventory_slot;
pub mod inventory_transaction;
pub mod item_registry;
pub mod item_stack_request;
pub mod item_stack_response;
pub mod level_chunk;
pub mod level_event;
pub mod login;
pub mod mob_effect;
pub mod mob_equipment;
pub mod modal_form_request;
pub mod modal_form_response;
pub mod move_actor_absolute;
pub mod move_player;
pub mod network_chunk_publisher_update;
pub mod network_settings;
pub mod play_sound;
pub mod play_status;
pub mod player_action;
pub mod player_auth_input;
pub mod player_enchant_options;
pub mod player_list;
pub mod player_skin;
pub mod remove_entity;
pub mod request_chunk_radius;
pub mod request_network_settings;
pub mod resource_pack_chunk_data;
pub mod resource_pack_chunk_request;
pub mod resource_pack_client_response;
pub mod resource_pack_data_info;
pub mod resource_pack_stack;
pub mod resource_packs_info;
pub mod respawn;
pub mod server_to_client_handshake;
pub mod set_display_objective;
pub mod set_entity_motion;
pub mod set_local_player_as_initialized;
pub mod set_player_game_type;
pub mod set_score;
pub mod set_time;
pub mod set_title;
pub mod spawn_particle_effect;
pub mod start_game;
pub mod take_item_entity;
pub mod text;
pub mod transfer;
pub mod update_abilities;
pub mod update_attributes;
pub mod update_block;

pub use add_actor::{ActorAttribute, AddActor};
pub use add_item_entity::AddItemEntity;
pub use add_player::{AddPlayer, EntityMetadataEntry, MetadataValue};
pub use animate::Animate;
pub use available_commands::AvailableCommands;
pub use available_entity_identifiers::AvailableEntityIdentifiers;
pub use biome_definition_list::BiomeDefinitionList;
pub use block_actor_data::BlockActorData;
pub use boss_event::BossEvent;
pub use change_dimension::ChangeDimension;
pub use chunk_radius_updated::ChunkRadiusUpdated;
pub use client_to_server_handshake::ClientToServerHandshake;
pub use command_output::CommandOutput;
pub use command_request::{CommandOrigin, CommandRequest};
pub use container_close::ContainerClose;
pub use container_open::ContainerOpen;
pub use container_set_data::ContainerSetData;
pub use crafting_data::CraftingData;
pub use creative_content::CreativeContent;
pub use disconnect::Disconnect;
pub use entity_event::EntityEvent;
pub use game_rules_changed::GameRulesChanged;
pub use inventory_content::InventoryContent;
pub use inventory_slot::InventorySlot;
pub use inventory_transaction::{
    InventoryTransaction, UseItemAction, UseItemData, UseItemOnEntityAction, UseItemOnEntityData,
};
pub use item_registry::{ItemRegistry, ItemRegistryEntry};
pub use item_stack_request::ItemStackRequest;
pub use item_stack_response::{
    ItemStackResponse, StackResponseContainer, StackResponseEntry, StackResponseSlot,
};
pub use level_chunk::LevelChunk;
pub use level_event::LevelEvent;
pub use login::LoginPacket;
pub use mob_effect::MobEffect;
pub use mob_equipment::MobEquipment;
pub use modal_form_request::ModalFormRequest;
pub use modal_form_response::ModalFormResponse;
pub use move_actor_absolute::MoveActorAbsolute;
pub use move_player::{MoveMode, MovePlayer};
pub use network_chunk_publisher_update::NetworkChunkPublisherUpdate;
pub use network_settings::NetworkSettings;
pub use play_sound::PlaySound;
pub use play_status::{PlayStatus, PlayStatusType};
pub use player_action::{PlayerAction, PlayerActionType};
pub use player_auth_input::PlayerAuthInput;
pub use player_enchant_options::PlayerEnchantOptions;
pub use player_list::{PlayerListAdd, PlayerListAddPacket, PlayerListRemove};
pub use player_skin::PlayerSkin;
pub use remove_entity::RemoveEntity;
pub use request_chunk_radius::RequestChunkRadius;
pub use request_network_settings::RequestNetworkSettings;
pub use resource_pack_chunk_data::ResourcePackChunkData;
pub use resource_pack_chunk_request::ResourcePackChunkRequest;
pub use resource_pack_client_response::{ResourcePackClientResponse, ResourcePackResponseStatus};
pub use resource_pack_data_info::ResourcePackDataInfo;
pub use resource_pack_stack::ResourcePackStack;
pub use resource_packs_info::ResourcePacksInfo;
pub use respawn::Respawn;
pub use server_to_client_handshake::ServerToClientHandshake;
pub use set_display_objective::SetDisplayObjective;
pub use set_entity_motion::SetEntityMotion;
pub use set_local_player_as_initialized::SetLocalPlayerAsInitialized;
pub use set_player_game_type::SetPlayerGameType;
pub use set_score::{ScoreEntry, SetScore};
pub use set_time::SetTime;
pub use set_title::SetTitle;
pub use spawn_particle_effect::SpawnParticleEffect;
pub use start_game::{GameRule, GameRuleValue, StartGame};
pub use take_item_entity::TakeItemEntity;
pub use text::{Text, TextType};
pub use transfer::Transfer;
pub use update_abilities::UpdateAbilities;
pub use update_attributes::{AttributeEntry, UpdateAttributes};
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
    pub const RESOURCE_PACK_DATA_INFO: u32 = 0x52;
    pub const RESOURCE_PACK_CHUNK_DATA: u32 = 0x53;
    pub const RESOURCE_PACK_CHUNK_REQUEST: u32 = 0x54;
    pub const MODAL_FORM_REQUEST: u32 = 0x64;
    pub const MODAL_FORM_RESPONSE: u32 = 0x65;
    pub const TEXT: u32 = 0x09;
    pub const SET_TIME: u32 = 0x0A;
    pub const START_GAME: u32 = 0x0B;
    pub const ADD_PLAYER: u32 = 0x0C;
    pub const ADD_ACTOR: u32 = 0x0D;
    pub const REMOVE_ENTITY: u32 = 0x0E;
    pub const ADD_ITEM_ENTITY: u32 = 0x0F;
    pub const MOVE_ACTOR_ABSOLUTE: u32 = 0x10;
    pub const TAKE_ITEM_ENTITY: u32 = 0x11;
    pub const SET_ENTITY_MOTION: u32 = 0x12;
    pub const MOVE_PLAYER: u32 = 0x13;
    pub const UPDATE_BLOCK: u32 = 0x15;
    pub const ANIMATE: u32 = 0x2C;
    pub const RESPAWN: u32 = 0x2D;
    pub const LEVEL_EVENT: u32 = 0x19;
    pub const ENTITY_EVENT: u32 = 0x1B;
    pub const MOB_EFFECT: u32 = 0x1C;
    pub const UPDATE_ATTRIBUTES: u32 = 0x1D;
    pub const INVENTORY_TRANSACTION: u32 = 0x1E;
    pub const MOB_EQUIPMENT: u32 = 0x1F;
    pub const PLAYER_ACTION: u32 = 0x24;
    pub const LEVEL_CHUNK: u32 = 0x3A;
    pub const CHANGE_DIMENSION: u32 = 0x3D;
    pub const SET_PLAYER_GAME_TYPE: u32 = 0x3E;
    pub const PLAYER_LIST: u32 = 0x3F;
    pub const REQUEST_CHUNK_RADIUS: u32 = 0x45;
    pub const CHUNK_RADIUS_UPDATED: u32 = 0x46;
    pub const GAME_RULES_CHANGED: u32 = 0x48;
    pub const AVAILABLE_COMMANDS: u32 = 0x4C;
    pub const COMMAND_REQUEST: u32 = 0x4D;
    pub const COMMAND_OUTPUT: u32 = 0x4F;
    pub const SET_LOCAL_PLAYER_AS_INITIALIZED: u32 = 0x71;
    pub const AVAILABLE_ENTITY_IDENTIFIERS: u32 = 0x78;
    pub const NETWORK_CHUNK_PUBLISHER_UPDATE: u32 = 0x7A;
    pub const BIOME_DEFINITION_LIST: u32 = 0x7B;
    pub const NETWORK_SETTINGS: u32 = 0x8F;
    pub const PLAYER_AUTH_INPUT: u32 = 0x90;
    pub const INVENTORY_CONTENT: u32 = 0x31;
    pub const INVENTORY_SLOT: u32 = 0x32;
    pub const CRAFTING_DATA: u32 = 0x34;
    pub const CREATIVE_CONTENT: u32 = 0x91;
    pub const PLAYER_ENCHANT_OPTIONS: u32 = 0x92;
    pub const ITEM_STACK_REQUEST: u32 = 0x93;
    pub const ITEM_STACK_RESPONSE: u32 = 0x94;
    pub const CONTAINER_OPEN: u32 = 0x2E;
    pub const CONTAINER_CLOSE: u32 = 0x2F;
    pub const CONTAINER_SET_DATA: u32 = 0x33;
    pub const PLAYER_SKIN: u32 = 0x5D;
    pub const BLOCK_ACTOR_DATA: u32 = 0x38;
    pub const BOSS_EVENT: u32 = 0x4A;
    pub const TRANSFER: u32 = 0x55;
    pub const PLAY_SOUND: u32 = 0x56;
    pub const SET_TITLE: u32 = 0x58;
    pub const SET_DISPLAY_OBJECTIVE: u32 = 0x6B;
    pub const SET_SCORE: u32 = 0x6C;
    pub const SPAWN_PARTICLE_EFFECT: u32 = 0x76;
    pub const UPDATE_ABILITIES: u32 = 0xBB;
    pub const REQUEST_NETWORK_SETTINGS: u32 = 0xC1;
    pub const ITEM_REGISTRY: u32 = 0xA2;
    pub const SERVERBOUND_LOADING_SCREEN: u32 = 0x138;
}

/// Target protocol version (Minecraft Bedrock 1.26.0).
pub const PROTOCOL_VERSION: i32 = 924;

/// Minimum supported protocol version (Minecraft Bedrock 1.26.0).
pub const MIN_PROTOCOL_VERSION: i32 = 924;

/// Check whether a client protocol version is supported.
pub fn is_supported_version(v: i32) -> bool {
    (MIN_PROTOCOL_VERSION..=PROTOCOL_VERSION).contains(&v)
}

/// Return the game version string for a supported protocol version.
pub fn game_version_for_protocol(v: i32) -> &'static str {
    match v {
        924 => "1.26.0",
        _ => "1.26.0", // fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_version_range() {
        assert!(is_supported_version(924));
        assert!(!is_supported_version(923));
        assert!(!is_supported_version(925));
        assert!(!is_supported_version(0));
    }

    #[test]
    fn protocol_version_constants() {
        assert_eq!(PROTOCOL_VERSION, 924);
        assert_eq!(MIN_PROTOCOL_VERSION, 924);
        const { assert!(MIN_PROTOCOL_VERSION <= PROTOCOL_VERSION) };
    }

    #[test]
    fn game_version_mapping() {
        assert_eq!(game_version_for_protocol(924), "1.26.0");
    }

    #[test]
    fn game_version_current() {
        let v = game_version_for_protocol(PROTOCOL_VERSION);
        assert_eq!(v, "1.26.0");
    }
}
