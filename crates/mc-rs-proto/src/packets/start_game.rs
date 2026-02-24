//! StartGame (0x0B) — Server → Client.
//!
//! The largest packet in the Bedrock protocol. Contains the full world
//! configuration: game rules, spawn position, block palette settings,
//! item table, movement settings, and much more.

use bytes::{BufMut, Bytes};

use crate::codec::{self, ProtoEncode};
use crate::types::{BlockPos, Uuid, VarInt, VarLong, VarUInt32, VarUInt64, Vec2, Vec3};

// ---------------------------------------------------------------------------
// Helper types
// ---------------------------------------------------------------------------

/// A game rule value (bool, int, or float).
#[derive(Debug, Clone)]
pub enum GameRuleValue {
    Bool(bool),
    Int(i32),
    Float(f32),
}

/// A server game rule.
#[derive(Debug, Clone)]
pub struct GameRule {
    pub name: String,
    pub editable: bool,
    pub value: GameRuleValue,
}

/// An experiment toggle.
#[derive(Debug, Clone)]
pub struct Experiment {
    pub name: String,
    pub enabled: bool,
}

/// Education Edition resource URI.
#[derive(Debug, Clone, Default)]
pub struct EduResourceUri {
    pub button_name: String,
    pub link_uri: String,
}

/// Player movement authority settings.
#[derive(Debug, Clone)]
pub struct MovementSettings {
    /// 0 = ClientAuthoritative, 1 = ServerAuthoritative, 2 = ServerAuthWithRewind
    pub auth_type: i32,
    pub rewind_history_size: i32,
    pub server_auth_block_breaking: bool,
}

impl Default for MovementSettings {
    fn default() -> Self {
        Self {
            auth_type: 2,
            rewind_history_size: 40,
            server_auth_block_breaking: false,
        }
    }
}

/// An entry in the item table sent with StartGame.
#[derive(Debug, Clone)]
pub struct ItemTableEntry {
    pub string_id: String,
    pub numeric_id: i16,
    pub is_component_based: bool,
}

/// A custom block property (empty for vanilla).
#[derive(Debug, Clone)]
pub struct BlockProperty {
    pub name: String,
    pub nbt: Bytes,
}

// ---------------------------------------------------------------------------
// StartGame packet
// ---------------------------------------------------------------------------

/// The massive StartGame packet containing all world configuration.
#[derive(Debug, Clone)]
pub struct StartGame {
    pub entity_unique_id: i64,
    pub entity_runtime_id: u64,
    pub player_gamemode: i32,
    pub player_position: Vec3,
    pub rotation: Vec2,
    pub seed: u64,
    pub biome_type: i16,
    pub biome_name: String,
    pub dimension: i32,
    pub generator: i32,
    pub world_gamemode: i32,
    pub difficulty: i32,
    pub spawn_position: BlockPos,
    pub achievements_disabled: bool,
    pub editor_world_type: i32,
    pub created_in_editor: bool,
    pub exported_from_editor: bool,
    pub day_cycle_stop_time: i32,
    pub edu_offer: i32,
    pub edu_features_enabled: bool,
    pub edu_product_uuid: String,
    pub rain_level: f32,
    pub lightning_level: f32,
    pub has_confirmed_platform_locked_content: bool,
    pub is_multiplayer: bool,
    pub broadcast_to_lan: bool,
    pub xbox_live_broadcast_mode: u32,
    pub platform_broadcast_mode: u32,
    pub enable_commands: bool,
    pub are_texture_packs_required: bool,
    pub game_rules: Vec<GameRule>,
    pub experiments: Vec<Experiment>,
    pub experiments_previously_used: bool,
    pub bonus_chest: bool,
    pub map_enabled: bool,
    pub permission_level: i32,
    pub server_chunk_tick_range: i32,
    pub has_locked_behavior_pack: bool,
    pub has_locked_resource_pack: bool,
    pub is_from_locked_world_template: bool,
    pub msa_gamertags_only: bool,
    pub is_from_world_template: bool,
    pub is_world_template_option_locked: bool,
    pub only_spawn_v1_villagers: bool,
    pub persona_disabled: bool,
    pub custom_skins_disabled: bool,
    pub emote_chat_muted: bool,
    pub game_version: String,
    pub limited_world_width: i32,
    pub limited_world_length: i32,
    pub is_new_nether: bool,
    pub edu_resource_uri: EduResourceUri,
    pub experimental_gameplay_override: bool,
    pub chat_restriction_level: u8,
    pub disable_player_interactions: bool,
    pub server_identifier: String,
    pub world_identifier: String,
    pub scenario_identifier: String,
    pub level_id: String,
    pub world_name: String,
    pub premium_world_template_id: String,
    pub is_trial: bool,
    pub movement_settings: MovementSettings,
    pub current_tick: i64,
    pub enchantment_seed: i32,
    pub block_properties: Vec<BlockProperty>,
    pub item_table: Vec<ItemTableEntry>,
    pub multiplayer_correlation_id: String,
    pub server_authoritative_inventory: bool,
    pub game_engine: String,
    pub property_data: Bytes,
    pub block_palette_checksum: u64,
    pub world_template_id: Uuid,
    pub client_side_generation: bool,
    pub block_network_ids_are_hashes: bool,
    pub server_controlled_sounds: bool,
}

/// Pre-encoded empty NBT compound in network format.
/// TAG_Compound(0x0A) + VarUInt name_len(0) + TAG_End(0x00).
const EMPTY_NBT_COMPOUND: &[u8] = &[0x0A, 0x00, 0x00];

impl Default for StartGame {
    fn default() -> Self {
        Self {
            entity_unique_id: 1,
            entity_runtime_id: 1,
            player_gamemode: 1, // creative
            player_position: Vec3::new(0.0, 64.0, 0.0),
            rotation: Vec2::ZERO,
            seed: 0,
            biome_type: 0,
            biome_name: String::new(),
            dimension: 0,      // overworld
            generator: 2,      // flat
            world_gamemode: 1, // creative
            difficulty: 1,     // easy
            spawn_position: BlockPos::new(0, 64, 0),
            achievements_disabled: true,
            editor_world_type: 0,
            created_in_editor: false,
            exported_from_editor: false,
            day_cycle_stop_time: 0,
            edu_offer: 0,
            edu_features_enabled: false,
            edu_product_uuid: String::new(),
            rain_level: 0.0,
            lightning_level: 0.0,
            has_confirmed_platform_locked_content: false,
            is_multiplayer: true,
            broadcast_to_lan: true,
            xbox_live_broadcast_mode: 4,
            platform_broadcast_mode: 4,
            enable_commands: true,
            are_texture_packs_required: false,
            game_rules: default_game_rules(),
            experiments: Vec::new(),
            experiments_previously_used: false,
            bonus_chest: false,
            map_enabled: false,
            permission_level: 1, // operator
            server_chunk_tick_range: 4,
            has_locked_behavior_pack: false,
            has_locked_resource_pack: false,
            is_from_locked_world_template: false,
            msa_gamertags_only: false,
            is_from_world_template: false,
            is_world_template_option_locked: false,
            only_spawn_v1_villagers: false,
            persona_disabled: false,
            custom_skins_disabled: false,
            emote_chat_muted: false,
            game_version: "1.21.50".into(),
            limited_world_width: 0,
            limited_world_length: 0,
            is_new_nether: true,
            edu_resource_uri: EduResourceUri::default(),
            experimental_gameplay_override: false,
            chat_restriction_level: 0,
            disable_player_interactions: false,
            server_identifier: String::new(),
            world_identifier: String::new(),
            scenario_identifier: String::new(),
            level_id: "level".into(),
            world_name: "MC-RS Server".into(),
            premium_world_template_id: String::new(),
            is_trial: false,
            movement_settings: MovementSettings::default(),
            current_tick: 0,
            enchantment_seed: 0,
            block_properties: Vec::new(),
            item_table: Vec::new(),
            multiplayer_correlation_id: String::new(),
            server_authoritative_inventory: false,
            game_engine: "vanilla".into(),
            property_data: Bytes::from_static(EMPTY_NBT_COMPOUND),
            block_palette_checksum: 0,
            world_template_id: Uuid::ZERO,
            client_side_generation: false,
            block_network_ids_are_hashes: true,
            server_controlled_sounds: false,
        }
    }
}

fn default_game_rules() -> Vec<GameRule> {
    vec![
        GameRule {
            name: "dodaylightcycle".into(),
            editable: false,
            value: GameRuleValue::Bool(true),
        },
        GameRule {
            name: "domobspawning".into(),
            editable: false,
            value: GameRuleValue::Bool(true),
        },
        GameRule {
            name: "doweathercycle".into(),
            editable: false,
            value: GameRuleValue::Bool(true),
        },
        GameRule {
            name: "pvp".into(),
            editable: false,
            value: GameRuleValue::Bool(true),
        },
        GameRule {
            name: "showcoordinates".into(),
            editable: false,
            value: GameRuleValue::Bool(true),
        },
    ]
}

fn encode_game_rules(buf: &mut impl BufMut, rules: &[GameRule]) {
    VarUInt32(rules.len() as u32).proto_encode(buf);
    for rule in rules {
        codec::write_string(buf, &rule.name);
        buf.put_u8(rule.editable as u8);
        match &rule.value {
            GameRuleValue::Bool(v) => {
                VarUInt32(1).proto_encode(buf);
                buf.put_u8(*v as u8);
            }
            GameRuleValue::Int(v) => {
                VarUInt32(2).proto_encode(buf);
                VarInt(*v).proto_encode(buf);
            }
            GameRuleValue::Float(v) => {
                VarUInt32(3).proto_encode(buf);
                buf.put_f32_le(*v);
            }
        }
    }
}

impl ProtoEncode for StartGame {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarLong(self.entity_unique_id).proto_encode(buf);
        VarUInt64(self.entity_runtime_id).proto_encode(buf);
        VarInt(self.player_gamemode).proto_encode(buf);
        self.player_position.proto_encode(buf);
        self.rotation.proto_encode(buf);
        buf.put_u64_le(self.seed);
        buf.put_i16_le(self.biome_type);
        codec::write_string(buf, &self.biome_name);
        VarInt(self.dimension).proto_encode(buf);
        VarInt(self.generator).proto_encode(buf);
        VarInt(self.world_gamemode).proto_encode(buf);
        VarInt(self.difficulty).proto_encode(buf);
        self.spawn_position.proto_encode(buf);
        buf.put_u8(self.achievements_disabled as u8);
        VarInt(self.editor_world_type).proto_encode(buf);
        buf.put_u8(self.created_in_editor as u8);
        buf.put_u8(self.exported_from_editor as u8);
        VarInt(self.day_cycle_stop_time).proto_encode(buf);
        VarInt(self.edu_offer).proto_encode(buf);
        buf.put_u8(self.edu_features_enabled as u8);
        codec::write_string(buf, &self.edu_product_uuid);
        buf.put_f32_le(self.rain_level);
        buf.put_f32_le(self.lightning_level);
        buf.put_u8(self.has_confirmed_platform_locked_content as u8);
        buf.put_u8(self.is_multiplayer as u8);
        buf.put_u8(self.broadcast_to_lan as u8);
        VarUInt32(self.xbox_live_broadcast_mode).proto_encode(buf);
        VarUInt32(self.platform_broadcast_mode).proto_encode(buf);
        buf.put_u8(self.enable_commands as u8);
        buf.put_u8(self.are_texture_packs_required as u8);
        encode_game_rules(buf, &self.game_rules);
        VarUInt32(self.experiments.len() as u32).proto_encode(buf);
        for exp in &self.experiments {
            codec::write_string(buf, &exp.name);
            buf.put_u8(exp.enabled as u8);
        }
        buf.put_u8(self.experiments_previously_used as u8);
        buf.put_u8(self.bonus_chest as u8);
        buf.put_u8(self.map_enabled as u8);
        VarInt(self.permission_level).proto_encode(buf);
        buf.put_i32_le(self.server_chunk_tick_range);
        buf.put_u8(self.has_locked_behavior_pack as u8);
        buf.put_u8(self.has_locked_resource_pack as u8);
        buf.put_u8(self.is_from_locked_world_template as u8);
        buf.put_u8(self.msa_gamertags_only as u8);
        buf.put_u8(self.is_from_world_template as u8);
        buf.put_u8(self.is_world_template_option_locked as u8);
        buf.put_u8(self.only_spawn_v1_villagers as u8);
        buf.put_u8(self.persona_disabled as u8);
        buf.put_u8(self.custom_skins_disabled as u8);
        buf.put_u8(self.emote_chat_muted as u8);
        codec::write_string(buf, &self.game_version);
        buf.put_i32_le(self.limited_world_width);
        buf.put_i32_le(self.limited_world_length);
        buf.put_u8(self.is_new_nether as u8);
        codec::write_string(buf, &self.edu_resource_uri.button_name);
        codec::write_string(buf, &self.edu_resource_uri.link_uri);
        buf.put_u8(self.experimental_gameplay_override as u8);
        buf.put_u8(self.chat_restriction_level);
        buf.put_u8(self.disable_player_interactions as u8);
        codec::write_string(buf, &self.server_identifier);
        codec::write_string(buf, &self.world_identifier);
        codec::write_string(buf, &self.scenario_identifier);
        codec::write_string(buf, &self.level_id);
        codec::write_string(buf, &self.world_name);
        codec::write_string(buf, &self.premium_world_template_id);
        buf.put_u8(self.is_trial as u8);
        VarInt(self.movement_settings.auth_type).proto_encode(buf);
        VarInt(self.movement_settings.rewind_history_size).proto_encode(buf);
        buf.put_u8(self.movement_settings.server_auth_block_breaking as u8);
        buf.put_i64_le(self.current_tick);
        VarInt(self.enchantment_seed).proto_encode(buf);
        // Block properties
        VarUInt32(self.block_properties.len() as u32).proto_encode(buf);
        for bp in &self.block_properties {
            codec::write_string(buf, &bp.name);
            buf.put_slice(&bp.nbt);
        }
        // Item table
        VarUInt32(self.item_table.len() as u32).proto_encode(buf);
        for item in &self.item_table {
            codec::write_string(buf, &item.string_id);
            buf.put_i16_le(item.numeric_id);
            buf.put_u8(item.is_component_based as u8);
        }
        codec::write_string(buf, &self.multiplayer_correlation_id);
        buf.put_u8(self.server_authoritative_inventory as u8);
        codec::write_string(buf, &self.game_engine);
        buf.put_slice(&self.property_data);
        buf.put_u64_le(self.block_palette_checksum);
        self.world_template_id.proto_encode(buf);
        buf.put_u8(self.client_side_generation as u8);
        buf.put_u8(self.block_network_ids_are_hashes as u8);
        buf.put_u8(self.server_controlled_sounds as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_default_does_not_panic() {
        let pkt = StartGame::default();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // Should produce a reasonable amount of data
        assert!(buf.len() > 100, "StartGame too small: {} bytes", buf.len());
    }

    #[test]
    fn encode_starts_with_entity_ids() {
        let pkt = StartGame::default();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarLong(1) with ZigZag: (1 << 1) ^ (1 >> 63) = 2 -> [0x02]
        assert_eq!(buf[0], 0x02, "entity_unique_id VarLong(1) should be 0x02");
        // VarUInt64(1) without ZigZag: 1 -> [0x01]
        assert_eq!(
            buf[1], 0x01,
            "entity_runtime_id VarUInt64(1) should be 0x01"
        );
    }

    #[test]
    fn game_rule_encoding() {
        let rules = vec![GameRule {
            name: "pvp".into(),
            editable: false,
            value: GameRuleValue::Bool(true),
        }];
        let mut buf = BytesMut::new();
        encode_game_rules(&mut buf, &rules);
        // VarUInt32(1) + String("pvp") + bool(false) + VarUInt32(1=bool type) + bool(true)
        // = 1 + (1+3) + 1 + 1 + 1 = 8
        assert_eq!(buf.len(), 8);
    }
}
