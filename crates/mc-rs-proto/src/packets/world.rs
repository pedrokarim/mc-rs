use crate::io::ProtoWriter;

// ── StartGame (S→C, 0x0B) ──
// Aligned EXACTLY with PocketMine-MP StartGamePacket.php (protocol 924)

pub struct StartGame {
    // ── Top-level fields ──
    pub actor_unique_id: i64,
    pub actor_runtime_id: u64,
    pub player_gamemode: i32,
    pub player_position: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,

    // ── LevelSettings (inlined) ──
    pub seed: u64,
    pub spawn_biome_type: u16,
    pub custom_biome_name: String,
    pub dimension: i32,
    pub generator: i32,
    pub world_gamemode: i32,
    pub hardcore: bool,
    pub difficulty: i32,
    pub spawn_position: [i32; 3],
    pub achievements_disabled: bool,
    pub editor_world_type: i32,
    pub created_in_editor: bool,
    pub exported_from_editor: bool,
    pub day_cycle_lock_time: i32,
    pub edu_offer: i32,
    pub edu_features: bool,
    pub edu_product_uuid: String,
    pub rain_level: f32,
    pub lightning_level: f32,
    pub confirmed_platform_locked: bool,
    pub multiplayer_game: bool,
    pub lan_broadcast: bool,
    pub xbox_live_broadcast_mode: i32,
    pub platform_broadcast_mode: i32,
    pub commands_enabled: bool,
    pub texture_packs_required: bool,
    pub game_rules: Vec<GameRule>,
    pub experiments: Vec<(String, bool)>,
    pub experiments_previously_toggled: bool,
    pub bonus_chest: bool,
    pub start_with_map: bool,
    pub default_player_permission: i32,
    pub server_chunk_tick_radius: i32,
    pub has_locked_behavior_pack: bool,
    pub has_locked_resource_pack: bool,
    pub is_from_locked_world_template: bool,
    pub msa_gamertags_only: bool,
    pub is_from_world_template: bool,
    pub is_world_template_option_locked: bool,
    pub only_spawn_v1_villagers: bool,
    pub disable_persona: bool,
    pub disable_custom_skins: bool,
    pub mute_emote_announcements: bool,
    pub vanilla_version: String,
    pub limited_world_width: i32,
    pub limited_world_length: i32,
    pub is_new_nether: bool,
    pub edu_shared_uri_button: String,
    pub edu_shared_uri_link: String,
    pub experimental_gameplay_override: Option<bool>,
    pub chat_restriction_level: u8,
    pub disable_player_interactions: bool,
    // ── End LevelSettings ──

    pub level_id: String,
    pub world_name: String,
    pub premium_world_template_id: String,
    pub is_trial: bool,

    // ── PlayerMovementSettings ──
    pub rewind_history_size: i32,
    pub server_authoritative_block_breaking: bool,

    pub current_tick: u64,
    pub enchantment_seed: i32,

    // Block palette (empty when using hashes)
    // We write count=0

    pub multiplayer_correlation_id: String,
    pub enable_new_inventory_system: bool,
    pub server_software_version: String,

    // playerActorProperties: empty NBT CompoundTag
    pub player_actor_properties_nbt: Vec<u8>,

    pub block_palette_checksum: u64,

    // worldTemplateId: UUID (2x i64_le)
    pub world_template_id: [u8; 16],

    pub enable_client_side_chunk_generation: bool,
    pub block_network_ids_are_hashes: bool,

    // NetworkPermissions
    pub disable_client_sounds: bool,

    // ServerJoinInformation: Optional (we write false = not present)

    // ServerTelemetryData
    pub server_id: String,
    pub scenario_id: String,
    pub world_id: String,
    pub owner_id: String,
}

impl StartGame {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(2048);

        // ── Top-level ──
        w.write_var_i64(self.actor_unique_id);
        w.write_var_u64(self.actor_runtime_id);
        w.write_var_i32(self.player_gamemode);

        w.write_f32_le(self.player_position[0]);
        w.write_f32_le(self.player_position[1]);
        w.write_f32_le(self.player_position[2]);

        w.write_f32_le(self.pitch);
        w.write_f32_le(self.yaw);

        // ── LevelSettings ──
        w.write_u64_le(self.seed);

        // SpawnSettings
        w.write_u16_le(self.spawn_biome_type);
        w.write_string(&self.custom_biome_name);
        w.write_var_i32(self.dimension);

        w.write_var_i32(self.generator);
        w.write_var_i32(self.world_gamemode);
        w.write_bool(self.hardcore);
        w.write_var_i32(self.difficulty);

        // BlockPosition (spawn)
        w.write_var_i32(self.spawn_position[0]);
        w.write_var_u32(self.spawn_position[1] as u32);
        w.write_var_i32(self.spawn_position[2]);

        w.write_bool(self.achievements_disabled);
        w.write_var_i32(self.editor_world_type);
        w.write_bool(self.created_in_editor);
        w.write_bool(self.exported_from_editor);
        w.write_var_i32(self.day_cycle_lock_time);
        w.write_var_i32(self.edu_offer);
        w.write_bool(self.edu_features);
        w.write_string(&self.edu_product_uuid);
        w.write_f32_le(self.rain_level);
        w.write_f32_le(self.lightning_level);
        w.write_bool(self.confirmed_platform_locked);
        w.write_bool(self.multiplayer_game);
        w.write_bool(self.lan_broadcast);
        w.write_var_i32(self.xbox_live_broadcast_mode);
        w.write_var_i32(self.platform_broadcast_mode);
        w.write_bool(self.commands_enabled);
        w.write_bool(self.texture_packs_required);

        // Game rules
        w.write_var_u32(self.game_rules.len() as u32);
        for rule in &self.game_rules {
            rule.encode(&mut w);
        }

        // Experiments
        w.write_u32_le(self.experiments.len() as u32);
        for (name, enabled) in &self.experiments {
            w.write_string(name);
            w.write_bool(*enabled);
        }
        w.write_bool(self.experiments_previously_toggled);

        w.write_bool(self.bonus_chest);
        w.write_bool(self.start_with_map);
        w.write_var_i32(self.default_player_permission);
        w.write_i32_le(self.server_chunk_tick_radius);
        w.write_bool(self.has_locked_behavior_pack);
        w.write_bool(self.has_locked_resource_pack);
        w.write_bool(self.is_from_locked_world_template);
        w.write_bool(self.msa_gamertags_only);
        w.write_bool(self.is_from_world_template);
        w.write_bool(self.is_world_template_option_locked);
        w.write_bool(self.only_spawn_v1_villagers);
        w.write_bool(self.disable_persona);
        w.write_bool(self.disable_custom_skins);
        w.write_bool(self.mute_emote_announcements);
        w.write_string(&self.vanilla_version);
        w.write_i32_le(self.limited_world_width);
        w.write_i32_le(self.limited_world_length);
        w.write_bool(self.is_new_nether);

        // EducationUriResource
        w.write_string(&self.edu_shared_uri_button);
        w.write_string(&self.edu_shared_uri_link);

        // experimentalGameplayOverride: Optional<bool>
        match self.experimental_gameplay_override {
            Some(v) => {
                w.write_bool(true);
                w.write_bool(v);
            }
            None => {
                w.write_bool(false);
            }
        }

        w.write_u8(self.chat_restriction_level);
        w.write_bool(self.disable_player_interactions);
        // ── End LevelSettings ──

        w.write_string(&self.level_id);
        w.write_string(&self.world_name);
        w.write_string(&self.premium_world_template_id);
        w.write_bool(self.is_trial);

        // PlayerMovementSettings
        w.write_var_i32(self.rewind_history_size);
        w.write_bool(self.server_authoritative_block_breaking);

        w.write_u64_le(self.current_tick);
        w.write_var_i32(self.enchantment_seed);

        // Block palette (empty when using hashes)
        w.write_var_u32(0);

        w.write_string(&self.multiplayer_correlation_id);
        w.write_bool(self.enable_new_inventory_system);
        w.write_string(&self.server_software_version);

        // playerActorProperties NBT
        w.write_raw(&self.player_actor_properties_nbt);

        w.write_u64_le(self.block_palette_checksum);

        // worldTemplateId UUID (2x i64_le, bytes swapped)
        // First 8 bytes reversed, then next 8 bytes reversed
        let mut uuid_part1 = [0u8; 8];
        let mut uuid_part2 = [0u8; 8];
        uuid_part1.copy_from_slice(&self.world_template_id[0..8]);
        uuid_part2.copy_from_slice(&self.world_template_id[8..16]);
        uuid_part1.reverse();
        uuid_part2.reverse();
        w.write_raw(&uuid_part1);
        w.write_raw(&uuid_part2);

        w.write_bool(self.enable_client_side_chunk_generation);
        w.write_bool(self.block_network_ids_are_hashes);

        // NetworkPermissions
        w.write_bool(self.disable_client_sounds);

        // ServerJoinInformation: Optional — write false (not present)
        w.write_bool(false);

        // ServerTelemetryData
        w.write_string(&self.server_id);
        w.write_string(&self.scenario_id);
        w.write_string(&self.world_id);
        w.write_string(&self.owner_id);

        w.into_bytes()
    }

    pub fn default_flat_with_id(entity_id: i64) -> Self {
        Self {
            actor_unique_id: entity_id,
            actor_runtime_id: entity_id as u64,
            player_gamemode: 1, // creative
            player_position: [0.5, -57.0, 0.5],
            pitch: 0.0,
            yaw: 0.0,
            seed: 0,
            spawn_biome_type: 0, // default
            custom_biome_name: String::new(),
            dimension: 0,  // overworld
            generator: 2,  // flat
            world_gamemode: 1, // creative
            hardcore: false,
            difficulty: 1, // easy
            spawn_position: [0, -59, 0],
            achievements_disabled: true,
            editor_world_type: 0,
            created_in_editor: false,
            exported_from_editor: false,
            day_cycle_lock_time: 0,
            edu_offer: 0,
            edu_features: false,
            edu_product_uuid: String::new(),
            rain_level: 0.0,
            lightning_level: 0.0,
            confirmed_platform_locked: false,
            multiplayer_game: true,
            lan_broadcast: true,
            xbox_live_broadcast_mode: 4,
            platform_broadcast_mode: 4,
            commands_enabled: true,
            texture_packs_required: false,
            game_rules: Vec::new(),
            experiments: Vec::new(),
            experiments_previously_toggled: false,
            bonus_chest: false,
            start_with_map: false,
            default_player_permission: 1, // member
            server_chunk_tick_radius: 4,
            has_locked_behavior_pack: false,
            has_locked_resource_pack: false,
            is_from_locked_world_template: false,
            msa_gamertags_only: false,
            is_from_world_template: false,
            is_world_template_option_locked: false,
            only_spawn_v1_villagers: false,
            disable_persona: false,
            disable_custom_skins: false,
            mute_emote_announcements: false,
            vanilla_version: "1.26.2".to_string(),
            limited_world_width: 0,
            limited_world_length: 0,
            is_new_nether: true,
            edu_shared_uri_button: String::new(),
            edu_shared_uri_link: String::new(),
            experimental_gameplay_override: None,
            chat_restriction_level: 0,
            disable_player_interactions: false,
            level_id: "mcrs".to_string(),
            world_name: "MC-RS World".to_string(),
            premium_world_template_id: String::new(),
            is_trial: false,
            rewind_history_size: 0,
            server_authoritative_block_breaking: true,
            current_tick: 0,
            enchantment_seed: 0,
            multiplayer_correlation_id: String::new(),
            enable_new_inventory_system: false,
            server_software_version: "1.26.2".to_string(),
            // Empty NBT compound tag (network LE format):
            // tag_type=10 (compound), name_length=0 (VarUInt=0x00), end_tag=0x00
            player_actor_properties_nbt: vec![0x0A, 0x00, 0x00],
            block_palette_checksum: 0,
            world_template_id: [0u8; 16],
            enable_client_side_chunk_generation: false,
            block_network_ids_are_hashes: false,
            disable_client_sounds: true,
            server_id: String::new(),
            scenario_id: String::new(),
            world_id: String::new(),
            owner_id: String::new(),
        }
    }
}

// ── Game Rule ──

pub enum GameRule {
    Bool(String, bool, bool),
    Int(String, bool, i32),
    Float(String, bool, f32),
}

impl GameRule {
    pub fn encode(&self, w: &mut ProtoWriter) {
        match self {
            Self::Bool(name, editable, value) => {
                w.write_string(name);
                w.write_bool(*editable);
                w.write_var_u32(1);
                w.write_bool(*value);
            }
            Self::Int(name, editable, value) => {
                w.write_string(name);
                w.write_bool(*editable);
                w.write_var_u32(2);
                w.write_var_i32(*value);
            }
            Self::Float(name, editable, value) => {
                w.write_string(name);
                w.write_bool(*editable);
                w.write_var_u32(3);
                w.write_f32_le(*value);
            }
        }
    }
}

// ── BiomeDefinitionList (S→C, 0x7A) ──

pub struct BiomeDefinitionList {
    pub nbt_data: Vec<u8>,
}

impl BiomeDefinitionList {
    pub fn encode(&self) -> Vec<u8> {
        self.nbt_data.clone()
    }
}

// ── AvailableActorIdentifiers (S→C, 0x77) ──

pub struct AvailableActorIdentifiers {
    pub nbt_data: Vec<u8>,
}

impl AvailableActorIdentifiers {
    pub fn encode(&self) -> Vec<u8> {
        self.nbt_data.clone()
    }
}

// ── AvailableCommands (S→C, 0x4C) ──

pub struct AvailableCommands;

impl AvailableCommands {
    /// Encode a minimal AvailableCommands packet with simple commands.
    /// Each command has 1 overload with 1 optional RAWTEXT arg.
    /// Format matches PMMP AvailableCommandsPacketAssembler exactly.
    pub fn encode_simple(commands: &[(&str, &str)]) -> Vec<u8> {
        use crate::io::ProtoWriter;
        let mut w = ProtoWriter::with_capacity(512);

        // Enum values (string pool) — empty
        w.write_var_u32(0);
        // Chained sub command values — empty
        w.write_var_u32(0);
        // Postfixes — empty
        w.write_var_u32(0);
        // Enums — empty
        w.write_var_u32(0);
        // Chained sub command data — empty
        w.write_var_u32(0);

        // Command data
        w.write_var_u32(commands.len() as u32);
        for &(name, description) in commands {
            w.write_string(name);         // command name (without /)
            w.write_string(description);  // description
            w.write_u16_le(0);            // flags
            w.write_string("any");        // permission level string (PMMP uses string, not u8!)
            w.write_i32_le(-1);           // alias enum index (-1 = none)
            w.write_var_u32(0);           // chained sub command indices count

            // 1 overload with 1 optional RAWTEXT param
            w.write_var_u32(1);           // overload count
            w.write_bool(false);          // chaining = false
            w.write_var_u32(1);           // parameter count
            // Parameter "args":
            w.write_string("args");
            // typeInfo = ARG_FLAG_VALID(0x100000) | RAWTEXT(70) = 0x00100046
            w.write_u32_le(0x00100046);
            w.write_bool(true);           // optional = true
            w.write_u8(0);                // flags = 0
        }

        // Soft enums — empty
        w.write_var_u32(0);
        // Constraints — empty
        w.write_var_u32(0);

        w.into_bytes()
    }
}

// ── CraftingData (S→C, 0x34) — empty ──

pub struct CraftingData;

impl CraftingData {
    pub fn encode_empty() -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(32);
        w.write_var_u32(0); // recipes count
        w.write_var_u32(0); // potion type recipes count
        w.write_var_u32(0); // potion container recipes count
        w.write_var_u32(0); // material reducer recipes count
        w.write_bool(true); // clear recipes
        w.into_bytes()
    }
}

// ── CreativeContent (S→C) — empty ──

pub struct CreativeContent;

impl CreativeContent {
    pub fn encode_empty() -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(8);
        w.write_var_u32(0); // groups count
        w.write_var_u32(0); // items count
        w.into_bytes()
    }
}

// ── ItemRegistry (S→C, 0x161) — empty ──

pub struct ItemRegistry;

impl ItemRegistry {
    pub fn encode_empty() -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(8);
        w.write_var_u32(0);
        w.into_bytes()
    }
}

// ── SetTime (S→C, 0x0A) ──

pub struct SetTime {
    pub time: i32,
}

impl SetTime {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(4);
        w.write_var_i32(self.time);
        w.into_bytes()
    }
}

// ── SetDifficulty (S→C, 0x3C) ──

pub struct SetDifficulty {
    pub difficulty: u32,
}

impl SetDifficulty {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(4);
        w.write_var_u32(self.difficulty);
        w.into_bytes()
    }
}

// ── SetSpawnPosition (S→C, 0x2B) ──

pub struct SetSpawnPosition {
    pub spawn_type: i32,
    pub position: [i32; 3],
    pub dimension: i32,
    pub spawn_position: [i32; 3],
}

impl SetSpawnPosition {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(32);
        w.write_var_i32(self.spawn_type);
        w.write_var_i32(self.position[0]);
        w.write_var_u32(self.position[1] as u32);
        w.write_var_i32(self.position[2]);
        w.write_var_i32(self.dimension);
        w.write_var_i32(self.spawn_position[0]);
        w.write_var_u32(self.spawn_position[1] as u32);
        w.write_var_i32(self.spawn_position[2]);
        w.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_startgame_hex() {
        let sg = StartGame::default_flat();
        let bytes = sg.encode();
        println!("\n=== StartGame: {} bytes ===", bytes.len());
        for (i, chunk) in bytes.chunks(32).enumerate() {
            print!("{:04X}: ", i * 32);
            for b in chunk { print!("{:02X} ", b); }
            println!();
        }
        println!("=== END ===\n");
    }
}
