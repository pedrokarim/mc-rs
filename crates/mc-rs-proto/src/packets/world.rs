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

        // BlockPosition spawn — gophertunnel 944 `BlockPos`: Varint32 signed
        // sur les 3 axes (writer.go:99). L'ancien VarU32 sur Y désalignait
        // tous les champs suivants dans StartGame (400+ fields).
        w.write_var_i32(self.spawn_position[0]);
        w.write_var_i32(self.spawn_position[1]);
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

    pub fn default_with_id(entity_id: i64, spawn_pos: [f32; 3]) -> Self {
        Self {
            actor_unique_id: entity_id,
            actor_runtime_id: entity_id as u64,
            player_gamemode: 0, // survival
            player_position: spawn_pos,
            pitch: 0.0,
            yaw: 0.0,
            seed: 0,
            spawn_biome_type: 0, // default
            custom_biome_name: String::new(),
            dimension: 0,      // overworld
            generator: 2,      // flat
            world_gamemode: 0, // survival
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
            vanilla_version: "1.26.10".to_string(),
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
            // We currently run Bedrock in the legacy/disabled ItemStackNetManager mode.
            // The client expects ContainerOpen + legacy InventoryContent/Slot semantics in this mode.
            enable_new_inventory_system: false,
            server_software_version: "1.26.10".to_string(),
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

// ── UpdateBlock (S→C, 0x15) ──

pub struct UpdateBlock {
    pub position: [i32; 3], // BlockPos gophertunnel 944 : VarInt32 signé X/Y/Z
    pub runtime_id: u32,    // VarUInt32
    pub flags: u32,         // VarUInt32 (FLAG_NETWORK=2 | FLAG_NEIGHBORS=1 = 3)
    pub layer: u32,         // VarUInt32 (0 = main)
}

impl UpdateBlock {
    pub fn encode(&self) -> Vec<u8> {
        use crate::io::ProtoWriter;
        let mut w = ProtoWriter::with_capacity(20);
        w.write_var_i32(self.position[0]);
        w.write_var_i32(self.position[1]);
        w.write_var_i32(self.position[2]);
        w.write_var_u32(self.runtime_id);
        w.write_var_u32(self.flags);
        w.write_var_u32(self.layer);
        w.into_bytes()
    }
}

// ── AvailableCommands (S→C, 0x4C) ──

// typeInfo flag constants
const ARG_FLAG_VALID: u32 = 0x100000;
const ARG_FLAG_ENUM: u32 = 0x200000;
const ARG_FLAG_SOFT_ENUM: u32 = 0x4000000;

/// Parameter type for the AvailableCommands packet encoder.
pub enum CmdParamType {
    /// Basic type: Int(1), Float(3), String(56), Target(8), Position(65), Message(68), RawText(70)
    Basic(u32),
    /// Hard enum with name and values — client shows as dropdown suggestions
    HardEnum { name: String, values: Vec<String> },
    /// Soft enum with dynamic values — player names etc.
    SoftEnum { name: String, values: Vec<String> },
}

/// A parameter entry for the packet encoder.
pub struct CmdParam {
    pub name: String,
    pub param_type: CmdParamType,
    pub optional: bool,
}

/// An overload (syntax variant) for the packet encoder.
pub struct CmdOverload {
    pub params: Vec<CmdParam>,
}

/// A command entry for the packet encoder.
pub struct CmdEntry {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub overloads: Vec<CmdOverload>,
}

pub struct AvailableCommands;

impl AvailableCommands {
    /// Encode a rich AvailableCommands packet with typed parameters and enums.
    pub fn encode_rich(commands: &[CmdEntry]) -> Vec<u8> {
        use crate::io::ProtoWriter;

        // ── Phase 1: Collect all enum values into a deduplicated global pool ──
        let mut enum_value_pool: Vec<String> = Vec::new();
        let mut enum_value_index = std::collections::HashMap::<String, u32>::new();

        // Collect hard enums (name → list of value indices)
        let mut hard_enums: Vec<(String, Vec<u32>)> = Vec::new();
        let mut hard_enum_index = std::collections::HashMap::<String, u32>::new();
        let mut soft_enums: Vec<(String, Vec<String>)> = Vec::new();
        let mut soft_enum_index = std::collections::HashMap::<String, u32>::new();

        // Helper: add a value to the pool and return its index
        let mut add_enum_value = |value: &str| -> u32 {
            if let Some(&idx) = enum_value_index.get(value) {
                idx
            } else {
                let idx = enum_value_pool.len() as u32;
                enum_value_pool.push(value.to_string());
                enum_value_index.insert(value.to_string(), idx);
                idx
            }
        };

        // First pass: collect all hard enums from parameters and aliases
        for cmd in commands {
            // Alias enum
            if !cmd.aliases.is_empty() {
                let alias_enum_name = format!("{}Aliases", cmd.name);
                if !hard_enum_index.contains_key(&alias_enum_name) {
                    let mut value_indices = Vec::new();
                    // PMMP adds the command name itself as first alias (client bug workaround)
                    value_indices.push(add_enum_value(&cmd.name));
                    for alias in &cmd.aliases {
                        value_indices.push(add_enum_value(alias));
                    }
                    let idx = hard_enums.len() as u32;
                    hard_enum_index.insert(alias_enum_name.clone(), idx);
                    hard_enums.push((alias_enum_name, value_indices));
                }
            }

            // Parameter enums
            for overload in &cmd.overloads {
                for param in &overload.params {
                    if let CmdParamType::HardEnum { name, values } = &param.param_type {
                        let enum_name = name.to_string();
                        if !hard_enum_index.contains_key(&enum_name) {
                            let mut value_indices = Vec::new();
                            for v in values {
                                value_indices.push(add_enum_value(v));
                            }
                            let idx = hard_enums.len() as u32;
                            hard_enum_index.insert(enum_name.clone(), idx);
                            hard_enums.push((enum_name, value_indices));
                        }
                    } else if let CmdParamType::SoftEnum { name, values } = &param.param_type {
                        let enum_name = name.to_string();
                        if !soft_enum_index.contains_key(&enum_name) {
                            let idx = soft_enums.len() as u32;
                            soft_enum_index.insert(enum_name.clone(), idx);
                            soft_enums.push((enum_name, values.clone()));
                        }
                    }
                }
            }
        }

        // ── Phase 2: Encode the packet ──
        let mut w = ProtoWriter::with_capacity(2048);

        // 1. Enum values (global string pool)
        w.write_var_u32(enum_value_pool.len() as u32);
        for val in &enum_value_pool {
            w.write_string(val);
        }

        // 2. Chained sub command values — empty
        w.write_var_u32(0);

        // 3. Postfixes — empty
        w.write_var_u32(0);

        // 4. Hard enums
        w.write_var_u32(hard_enums.len() as u32);
        for (enum_name, value_indices) in &hard_enums {
            w.write_string(enum_name);
            w.write_var_u32(value_indices.len() as u32);
            for &idx in value_indices {
                // PMMP: LE::writeUnsignedInt — always 4-byte LE
                w.write_u32_le(idx);
            }
        }

        // 5. Chained sub command data — empty
        w.write_var_u32(0);

        // 6. Command data
        w.write_var_u32(commands.len() as u32);
        for cmd in commands {
            w.write_string(&cmd.name);
            w.write_string(&cmd.description);
            w.write_u16_le(0); // flags
            w.write_string("any"); // permission level

            // Alias enum index
            let alias_enum_name = format!("{}Aliases", cmd.name);
            if let Some(&idx) = hard_enum_index.get(&alias_enum_name) {
                w.write_i32_le(idx as i32);
            } else {
                w.write_i32_le(-1);
            }

            // Chained sub command indices — none
            w.write_var_u32(0);

            // Overloads
            if cmd.overloads.is_empty() {
                // No overloads defined → send 1 overload with 0 params
                w.write_var_u32(1); // overload count
                w.write_bool(false); // chaining
                w.write_var_u32(0); // 0 parameters
            } else {
                w.write_var_u32(cmd.overloads.len() as u32);
                for overload in &cmd.overloads {
                    w.write_bool(false); // chaining = false
                    w.write_var_u32(overload.params.len() as u32);
                    for param in &overload.params {
                        w.write_string(&param.name);

                        // typeInfo encoding
                        let type_info = match &param.param_type {
                            CmdParamType::Basic(type_id) => ARG_FLAG_VALID | type_id,
                            CmdParamType::HardEnum { name, .. } => {
                                let enum_idx = hard_enum_index.get(name).copied().unwrap_or(0);
                                ARG_FLAG_VALID | ARG_FLAG_ENUM | enum_idx
                            }
                            CmdParamType::SoftEnum { name, .. } => {
                                let enum_idx = soft_enum_index.get(name).copied().unwrap_or(0);
                                ARG_FLAG_VALID | ARG_FLAG_SOFT_ENUM | enum_idx
                            }
                        };
                        w.write_u32_le(type_info);
                        w.write_bool(param.optional);
                        w.write_u8(0); // flags
                    }
                }
            }
        }

        // 7. Soft enums
        w.write_var_u32(soft_enums.len() as u32);
        for (enum_name, values) in &soft_enums {
            w.write_string(enum_name);
            w.write_var_u32(values.len() as u32);
            for value in values {
                w.write_string(value);
            }
        }

        // 8. Constraints — empty
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

// ── CreativeContent (S→C, 0x91) ──

/// Entry pour un groupe de l'inventaire créatif (catégorie).
pub struct CreativeGroupEntry<'a> {
    pub category_id: i32,
    pub category_name: &'a str,
    /// Icône du groupe — encodé via `ItemStackWithoutStackId` (pas de net_id,
    /// pas de extra_data ItemStackExtraData).
    pub icon_item_id: i32,
}

/// Entry pour un item de l'inventaire créatif.
pub struct CreativeItemEntry {
    pub entry_id: u32, // creative_item_net_id (utilisé par client pour CraftCreative)
    pub item_id: i32,
    pub block_runtime_id: i32,
    pub group_id: u32, // index dans le tableau de groupes
}

pub struct CreativeContent;

impl CreativeContent {
    /// Paquet vide (pas d'items en créatif). Utilisé pour les modes non-créatifs.
    pub fn encode_empty() -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(8);
        w.write_var_u32(0); // groups count
        w.write_var_u32(0); // items count
        w.into_bytes()
    }

    /// Encode un CreativeContent avec groupes + items.
    ///
    /// Format PMMP (`CreativeContentPacket::encodePayload`) :
    /// ```
    /// groups_count (VarU32)
    /// for each group: category_id (i32 LE) + category_name (string) + icon (ItemStackWithoutStackId)
    /// items_count (VarU32)
    /// for each item: entry_id (VarU32) + item (ItemStackWithoutStackId) + group_id (VarU32)
    /// ```
    ///
    /// `ItemStackWithoutStackId` = même format que `ItemStackWrapper` MAIS :
    /// - PAS de `has_net_id` bool
    /// - PAS de `stack_id` varint
    /// - sinon id/count/meta/block_rid/extra_data identique
    pub fn encode(groups: &[CreativeGroupEntry], items: &[CreativeItemEntry]) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(16384);

        // Groupes.
        w.write_var_u32(groups.len() as u32);
        for g in groups {
            w.write_i32_le(g.category_id);
            w.write_string(g.category_name);
            write_item_stack_without_id(&mut w, g.icon_item_id, 1, 0);
        }

        // Items.
        w.write_var_u32(items.len() as u32);
        for item in items {
            w.write_var_u32(item.entry_id);
            write_item_stack_without_id(&mut w, item.item_id, 1, item.block_runtime_id);
            w.write_var_u32(item.group_id);
        }

        w.into_bytes()
    }
}

/// Encode un `ItemStackWithoutStackId` : id + count + meta + block_rid + extra_data.
/// Pas de has_net_id/stack_id (contrairement à ItemStackWrapper).
fn write_item_stack_without_id(w: &mut ProtoWriter, item_id: i32, count: u16, block_runtime_id: i32) {
    w.write_var_i32(item_id);
    if item_id == 0 {
        return;
    }
    w.write_u16_le(count);
    w.write_var_u32(0); // meta = 0
    w.write_var_i32(block_runtime_id);
    // extra_data minimal : 2 bytes NBT_len=0 + 4 canPlace + 4 canDestroy
    let extra = [0u8; 10];
    w.write_byte_array(&extra);
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

// ── ResourcePack delivery (0x52, 0x53, 0x54) ──

/// ResourcePackDataInfo (S→C, 0x52). Annonce métadata d'un pack au client.
pub struct ResourcePackDataInfo {
    pub pack_id: String,        // UUID
    pub max_chunk_size: u32,
    pub chunk_count: u32,
    pub compressed_pack_size: u64,
    pub sha256: String,         // hex 64 chars
    pub is_premium: bool,
    pub pack_type: u8,          // 0=Resources, 4=Behavior
}

impl ResourcePackDataInfo {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(128);
        w.write_string(&self.pack_id);
        w.write_u32_le(self.max_chunk_size);
        w.write_u32_le(self.chunk_count);
        w.write_u64_le(self.compressed_pack_size);
        w.write_string(&self.sha256);
        w.write_bool(self.is_premium);
        w.write_u8(self.pack_type);
        w.into_bytes()
    }
}

/// ResourcePackChunkData (S→C, 0x53). Envoie un chunk d'un pack au client.
pub struct ResourcePackChunkData {
    pub pack_id: String,
    pub chunk_index: u32,
    pub offset: u64,
    pub data: Vec<u8>,
}

impl ResourcePackChunkData {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(self.data.len() + 32);
        w.write_string(&self.pack_id);
        w.write_u32_le(self.chunk_index);
        w.write_u64_le(self.offset);
        // PMMP utilise putString → le client attend une string PRECEDED par
        // VarUInt32 length puis raw bytes.
        w.write_byte_array(&self.data);
        w.into_bytes()
    }
}

/// ResourcePackChunkRequest (C→S, 0x54). Le client demande un chunk.
pub struct ResourcePackChunkRequest {
    pub pack_id: String,
    pub chunk_index: u32,
}

impl ResourcePackChunkRequest {
    pub fn decode(reader: &mut crate::io::ProtoReader) -> Result<Self, crate::io::reader::ProtoReadError> {
        let pack_id = reader.read_string()?;
        let chunk_index = reader.read_u32_le()?;
        Ok(Self { pack_id, chunk_index })
    }
}

// ── MobEffect (S→C, 0x1C) ──
//
// PMMP MobEffectPacket : actor_runtime_id + event(u8) + effect_id(var_i32)
// + amplifier(var_i32) + particles(bool) + duration_ticks(var_i32)
// + tick(var_u64) + ambient(bool).
pub struct MobEffect {
    pub actor_runtime_id: u64,
    pub event_id: u8, // 1 ADD, 2 MODIFY, 3 REMOVE
    pub effect_id: i32,
    pub amplifier: i32,
    pub particles: bool,
    pub duration_ticks: i32,
    pub tick: u64,
    pub ambient: bool,
}

impl MobEffect {
    pub const EVENT_ADD: u8 = 1;
    pub const EVENT_MODIFY: u8 = 2;
    pub const EVENT_REMOVE: u8 = 3;

    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(24);
        w.write_var_u64(self.actor_runtime_id);
        w.write_u8(self.event_id);
        w.write_var_i32(self.effect_id);
        w.write_var_i32(self.amplifier);
        w.write_bool(self.particles);
        w.write_var_i32(self.duration_ticks);
        w.write_var_u64(self.tick);
        w.write_bool(self.ambient);
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
        // gophertunnel 944 SetSpawnPosition : 2 x BlockPos signés (packet/set_spawn_position.go).
        let mut w = ProtoWriter::with_capacity(32);
        w.write_var_i32(self.spawn_type);
        w.write_var_i32(self.position[0]);
        w.write_var_i32(self.position[1]);
        w.write_var_i32(self.position[2]);
        w.write_var_i32(self.dimension);
        w.write_var_i32(self.spawn_position[0]);
        w.write_var_i32(self.spawn_position[1]);
        w.write_var_i32(self.spawn_position[2]);
        w.into_bytes()
    }
}

// ── ContainerOpen (S→C, 0x2E) ──

pub struct ContainerOpen {
    pub window_id: u8,
    pub window_type: u8, // -1 (0xFF) = player inventory
    pub position: [i32; 3],
    pub actor_unique_id: i64,
}

impl ContainerOpen {
    /// Open an entity-backed inventory window, like PocketMine does for the main inventory.
    pub fn entity_inventory(window_id: u8, actor_unique_id: i64) -> Self {
        Self {
            window_id,
            window_type: 0xFF, // WindowTypes::INVENTORY = -1 as u8
            position: [0, 0, 0],
            actor_unique_id,
        }
    }

    /// Open the player's own inventory UI.
    pub fn player_inventory(actor_unique_id: i64) -> Self {
        Self::entity_inventory(1, actor_unique_id)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(16);
        w.write_u8(self.window_id);
        w.write_u8(self.window_type);
        // BlockPos protocol 944 (gophertunnel writer.go:99) : X, Y, Z sont
        // TOUS des Varint32 signés (zigzag). PMMP 924 utilise VarU32 pour Y
        // sur certains paquets ; ici pour ContainerOpen c'est SIGNED pour les 3.
        // Si Y est écrit en VarU32 (par ex. 69 = 0x45, 1 byte) au lieu de
        // VarI32 (69 → zigzag 138 = 0x8A 0x01, 2 bytes) le wire est décalé
        // et le client crash.
        w.write_var_i32(self.position[0]);
        w.write_var_i32(self.position[1]);
        w.write_var_i32(self.position[2]);
        w.write_var_i64(self.actor_unique_id); // ActorUniqueId = VarI64
        w.into_bytes()
    }
}

// ── ContainerClose (S→C / C→S, 0x2F) ──

pub struct ContainerClose {
    pub window_id: u8,
    pub window_type: u8,
    pub server: bool,
}

impl ContainerClose {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(4);
        w.write_u8(self.window_id);
        w.write_u8(self.window_type);
        w.write_bool(self.server);
        w.into_bytes()
    }
}

// ── LevelEvent (S→C, 0x19) ──

pub struct LevelEvent {
    pub event_id: i32,
    pub position: [f32; 3],
    pub event_data: i32,
}

impl LevelEvent {
    /// Block destroy particles.
    pub const PARTICLE_DESTROY: i32 = 2001;
    /// Start block breaking crack animation (data = break speed * 65535).
    pub const BLOCK_START_BREAK: i32 = 3600;
    /// Stop/remove block breaking crack animation.
    pub const BLOCK_STOP_BREAK: i32 = 3601;
    /// Update block break speed mid-break.
    pub const BLOCK_BREAK_SPEED: i32 = 3602;

    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(20);
        w.write_var_i32(self.event_id);
        w.write_f32_le(self.position[0]);
        w.write_f32_le(self.position[1]);
        w.write_f32_le(self.position[2]);
        w.write_var_i32(self.event_data);
        w.into_bytes()
    }
}

// ── LevelSoundEvent (S→C, 0x7B) ──

pub struct LevelSoundEvent {
    pub sound: u32,
    pub position: [f32; 3],
    pub extra_data: i32,
    pub entity_type: String,
    pub is_baby_mob: bool,
    pub disable_relative_volume: bool,
    pub actor_unique_id: i64,
}

impl LevelSoundEvent {
    pub const BREAK: u32 = 5;
    pub const PLACE: u32 = 6;
    pub const HIT: u32 = 1;

    /// Create a non-actor block sound event.
    pub fn block_sound(sound: u32, position: [f32; 3], block_runtime_id: i32) -> Self {
        Self {
            sound,
            position,
            extra_data: block_runtime_id,
            entity_type: ":".to_string(),
            is_baby_mob: false,
            disable_relative_volume: false,
            actor_unique_id: -1,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(32);
        w.write_var_u32(self.sound);
        w.write_f32_le(self.position[0]);
        w.write_f32_le(self.position[1]);
        w.write_f32_le(self.position[2]);
        w.write_var_i32(self.extra_data);
        w.write_string(&self.entity_type);
        w.write_bool(self.is_baby_mob);
        w.write_bool(self.disable_relative_volume);
        w.write_i64_le(self.actor_unique_id);
        w.into_bytes()
    }
}

// ── ItemStackResponse (S→C, 0x94) ──

pub struct ItemStackResponseSlot {
    pub slot: u8,
    pub hotbar_slot: u8,
    pub count: u8,
    pub stack_id: i32,
    pub custom_name: String,
    pub filtered_custom_name: String,
    pub durability_correction: i32,
}

pub struct ItemStackResponseContainer {
    pub container_id: u8,
    pub slots: Vec<ItemStackResponseSlot>,
}

pub struct ItemStackResponseEntry {
    pub result: u8, // 0 = OK, 1+ = error
    pub request_id: i32,
    pub containers: Vec<ItemStackResponseContainer>,
}

pub struct ItemStackResponse {
    pub entries: Vec<ItemStackResponseEntry>,
}

impl ItemStackResponse {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(128);
        w.write_var_u32(self.entries.len() as u32);
        for entry in &self.entries {
            w.write_u8(entry.result);
            w.write_var_i32(entry.request_id);
            if entry.result == 0 {
                w.write_var_u32(entry.containers.len() as u32);
                for container in &entry.containers {
                    // FullContainerName
                    w.write_u8(container.container_id);
                    w.write_bool(false); // no dynamic ID
                    w.write_var_u32(container.slots.len() as u32);
                    for slot in &container.slots {
                        w.write_u8(slot.slot);
                        w.write_u8(slot.hotbar_slot);
                        w.write_u8(slot.count);
                        w.write_var_i32(slot.stack_id);
                        w.write_string(&slot.custom_name);
                        w.write_string(&slot.filtered_custom_name);
                        w.write_var_i32(slot.durability_correction);
                    }
                }
            }
        }
        w.into_bytes()
    }

    /// Create a simple OK response for a request.
    pub fn ok(request_id: i32, containers: Vec<ItemStackResponseContainer>) -> Self {
        Self {
            entries: vec![ItemStackResponseEntry {
                result: 0,
                request_id,
                containers,
            }],
        }
    }

    /// Create an error response for a request.
    pub fn error(request_id: i32) -> Self {
        Self {
            entries: vec![ItemStackResponseEntry {
                result: 2, // generic error
                request_id,
                containers: Vec::new(),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startgame_hex() {
        let sg = StartGame::default_with_id(1, [0.0, 64.0, 0.0]);
        let bytes = sg.encode();
        println!("\n=== StartGame: {} bytes ===", bytes.len());
        for (i, chunk) in bytes.chunks(32).enumerate() {
            print!("{:04X}: ", i * 32);
            for b in chunk {
                print!("{:02X} ", b);
            }
            println!();
        }
        println!("=== END ===\n");
    }

    /// Vérifie byte-par-byte que ContainerOpen match gophertunnel 944 pour
    /// WindowID=0, ContainerType=0xFF (-1), Position=[3, 69, 81], ActorUniqueID=-1.
    /// Expected:
    ///   0x00         WindowID u8 (0)
    ///   0xFF         ContainerType u8 (-1)
    ///   0x06         X VarInt32(3) zigzag=6
    ///   0x8A 0x01    Y VarInt32(69) zigzag=138 → LEB128 : 0x8A, 0x01
    ///   0xA2 0x01    Z VarInt32(81) zigzag=162 → LEB128 : 0xA2, 0x01
    ///   0x01         ActorUniqueID VarInt64(-1) zigzag=1 → 0x01
    #[test]
    fn container_open_bytes_match_gophertunnel_944() {
        let pkt = ContainerOpen {
            window_id: 0,
            window_type: 0xFF,
            position: [3, 69, 81],
            actor_unique_id: -1,
        };
        let bytes = pkt.encode();
        let expected = vec![0x00, 0xFF, 0x06, 0x8A, 0x01, 0xA2, 0x01, 0x01];
        assert_eq!(
            bytes, expected,
            "ContainerOpen bytes diverge : got {:02X?}, expected {:02X?}",
            bytes, expected
        );
    }

    /// Edge case : Y=0 (fond du monde). Avec VarU32 c'était `0x00`, avec VarI32 c'est aussi `0x00`.
    #[test]
    fn container_open_y_zero_same_byte() {
        let pkt = ContainerOpen {
            window_id: 0,
            window_type: 0xFF,
            position: [0, 0, 0],
            actor_unique_id: -1,
        };
        let bytes = pkt.encode();
        // 00 FF 00 00 00 01
        assert_eq!(bytes, vec![0x00, 0xFF, 0x00, 0x00, 0x00, 0x01]);
    }
}
