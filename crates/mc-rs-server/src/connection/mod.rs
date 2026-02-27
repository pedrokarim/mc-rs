//! Per-player connection state management and login flow.

mod combat;
mod commands;
mod inventory;
mod login;
mod movement;
mod plugins;
mod spawn;
mod survival;
mod world_tick;

use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use bytes::{BufMut, Bytes, BytesMut};
use tracing::{debug, info, warn};

use mc_rs_command::selector::PlayerInfo;
use mc_rs_command::{CommandRegistry, CommandResult};
use mc_rs_crypto::{
    create_handshake_jwt, derive_key, parse_client_public_key, PacketEncryption, ServerKeyPair,
};
use mc_rs_game::combat as game_combat;
use mc_rs_game::game_world::{GameEvent, GameWorld};
use mc_rs_game::inventory::PlayerInventory;
use mc_rs_game::recipe::RecipeRegistry;
use mc_rs_game::xp;
use mc_rs_proto::batch::{decode_batch, encode_single, BatchConfig};
use mc_rs_proto::codec::{ProtoDecode, ProtoEncode};
use mc_rs_proto::compression::CompressionAlgorithm;
use mc_rs_proto::jwt;
use mc_rs_proto::packets::add_player::default_player_metadata;
use mc_rs_proto::packets::{
    self, ActorAttribute, AddActor, AddPlayer, Animate, AvailableCommands,
    AvailableEntityIdentifiers, BiomeDefinitionList, BlockActorData, ChunkRadiusUpdated,
    ClientToServerHandshake, CommandOutput, CommandRequest, ContainerClose, ContainerOpen,
    ContainerSetData, Disconnect, EntityEvent, EntityMetadataEntry, GameRule, GameRuleValue,
    GameRulesChanged, InventoryContent, InventorySlot, InventoryTransaction, ItemStackRequest,
    ItemStackResponse, LevelChunk, LevelEvent, MetadataValue, MobEffect, MobEquipment,
    MoveActorAbsolute, MoveMode, MovePlayer, NetworkChunkPublisherUpdate, NetworkSettings,
    PlayStatus, PlayStatusType, PlayerAction, PlayerActionType, PlayerAuthInput, PlayerListAdd,
    PlayerListAddPacket, PlayerListRemove, RemoveEntity, RequestChunkRadius,
    ResourcePackClientResponse, ResourcePackResponseStatus, ResourcePackStack, ResourcePacksInfo,
    Respawn, ServerToClientHandshake, SetEntityMotion, SetLocalPlayerAsInitialized,
    SetPlayerGameType, SetTime, StartGame, Text, UpdateAbilities, UpdateAttributes, UpdateBlock,
    UseItemAction, UseItemOnEntityAction,
};
use mc_rs_proto::types::{BlockPos, Uuid, VarUInt32, Vec2, Vec3};
use mc_rs_raknet::{RakNetEvent, Reliability, ServerHandle};
use rand::prelude::*;

use mc_rs_game::block_entity::{self, BlockEntityData};
use mc_rs_game::smelting::SmeltingRegistry;
use mc_rs_world::block_hash::{BlockEntityHashes, FlatWorldBlocks, TickBlocks};
use mc_rs_world::block_registry::BlockRegistry;
use mc_rs_world::block_tick::{process_random_tick, process_scheduled_tick, TickScheduler};
use mc_rs_world::chunk::{ChunkColumn, OVERWORLD_MIN_Y, OVERWORLD_SUB_CHUNK_COUNT};
use mc_rs_world::flat_generator::generate_flat_chunk;
use mc_rs_world::fluid;
use mc_rs_world::gravity;
use mc_rs_world::item_registry::ItemRegistry;
use mc_rs_world::overworld_generator::OverworldGenerator;
use mc_rs_world::physics::{PlayerAabb, MAX_AIRBORNE_TICKS, MAX_FALL_PER_TICK};
use mc_rs_world::redstone;
use mc_rs_world::serializer::serialize_chunk_column;
use mc_rs_world::storage::{block_entity_key, LevelDbProvider};
use tokio::sync::watch;

use mc_rs_behavior_pack::loader::LoadedBehaviorPack;
use mc_rs_behavior_pack::loot_table::LootTableFile;
use mc_rs_plugin_api::{DamageCause, EventResult, PluginBlockPos, PluginEvent, PluginPlayer};

use crate::config::ServerConfig;
use crate::permissions::{BanEntry, PermissionManager};
use crate::persistence::{LevelDat, PlayerData};
use crate::plugin_manager::{PendingAction, PluginManager, ServerSnapshot};

/// Login state machine states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginState {
    /// Waiting for RequestNetworkSettings (0xC1).
    AwaitingNetworkSettings,
    /// NetworkSettings sent, waiting for LoginPacket (0x01).
    AwaitingLogin,
    /// ServerToClientHandshake sent, waiting for ClientToServerHandshake (0x04).
    /// Only used when online_mode = true.
    AwaitingHandshake,
    /// Login complete, PlayStatus(LoginSuccess) sent.
    LoggedIn,
    /// ResourcePacksInfo sent, waiting for ResourcePackClientResponse.
    AwaitingResourcePackResponse,
    /// ResourcePackStack sent, waiting for ResourcePackClientResponse(Completed).
    AwaitingResourcePackComplete,
    /// StartGame + metadata sent, waiting for chunks.
    Spawning,
    /// Player fully spawned and in the world.
    InGame,
}

/// Encryption key material waiting for handshake confirmation.
struct PendingEncryption {
    aes_key: [u8; 32],
    iv: [u8; 16],
}

/// Per-player connection state.
pub struct PlayerConnection {
    pub state: LoginState,
    pub batch_config: BatchConfig,
    pub login_data: Option<jwt::LoginData>,
    /// Client data (skin, device info) extracted from the client_data JWT.
    pub client_data: Option<jwt::ClientData>,
    /// Active packet encryption (set after handshake completes).
    pub encryption: Option<PacketEncryption>,
    /// Key material waiting for ClientToServerHandshake confirmation.
    pending_encryption: Option<PendingEncryption>,
    /// Unique entity ID assigned to this player.
    pub entity_unique_id: i64,
    /// Runtime entity ID assigned to this player.
    pub entity_runtime_id: u64,
    /// Server-accepted player position.
    pub position: Vec3,
    /// Player rotation (pitch, yaw, head_yaw).
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    /// Whether the player is on the ground.
    pub on_ground: bool,
    /// Last client tick received from PlayerAuthInput.
    pub client_tick: u64,
    /// Chunks already sent to this player.
    pub sent_chunks: HashSet<(i32, i32)>,
    /// Accepted chunk radius for this player.
    pub chunk_radius: i32,
    /// Player gamemode: 0=survival, 1=creative, 2=adventure, 3=spectator.
    pub gamemode: i32,
    /// Active block-breaking state: (position, start_time).
    pub breaking_block: Option<(BlockPos, Instant)>,
    /// Consecutive ticks spent airborne (for anti-fly detection).
    pub airborne_ticks: u32,
    /// Player inventory.
    pub inventory: PlayerInventory,
    /// Player health (0.0 - 20.0).
    pub health: f32,
    /// Tick when player last took damage (invulnerability frames).
    pub last_damage_tick: Option<u64>,
    /// Whether the player is dead (waiting for respawn).
    pub is_dead: bool,
    /// Whether the player is sprinting (from PlayerAuthInput flags).
    pub is_sprinting: bool,
    /// Active status effects on this player.
    pub effects: Vec<ActiveEffect>,
    /// Last vertical velocity (position_delta.y) for critical hit detection.
    pub last_position_delta_y: f32,
    /// Remaining fire ticks (1 damage per 20 ticks). 0 = not on fire.
    pub fire_ticks: i32,
    /// Food level (0-20). 20 = full.
    pub food: i32,
    /// Saturation level (0.0-20.0). Consumed before food.
    pub saturation: f32,
    /// Exhaustion accumulator (0.0-4.0+). Drains saturation/food at 4.0.
    pub exhaustion: f32,
    /// Accumulated fall distance in blocks.
    pub fall_distance: f32,
    /// Air supply ticks (300 = full). Decrements when head is in water.
    pub air_ticks: i32,
    /// Whether the player is swimming (from PlayerAuthInput flags).
    pub is_swimming: bool,
    /// XP level (0+).
    pub xp_level: i32,
    /// Total accumulated XP.
    pub xp_total: i32,
    /// Pending forms awaiting response: form_id -> form_type ("simple"/"modal"/"custom").
    pub pending_forms: HashMap<u32, String>,
    /// Currently open container (chest, etc.).
    pub open_container: Option<OpenContainer>,
    /// Next window ID to assign when opening a container.
    pub next_window_id: u8,
    /// Enchantment seed (used for deterministic option generation).
    pub enchant_seed: i32,
    /// Pending enchantment options offered to the player.
    pub pending_enchant_options: Vec<mc_rs_game::enchanting::EnchantOption>,
}

/// State for a currently open container window.
#[derive(Debug, Clone)]
pub struct OpenContainer {
    /// Window ID for this container session.
    pub window_id: u8,
    /// Container type (0=container/chest).
    #[allow(dead_code)]
    pub container_type: u8,
    /// World position of the container block.
    pub position: BlockPos,
}

/// An active status effect on a player.
#[derive(Debug, Clone)]
pub struct ActiveEffect {
    /// Effect ID (see `mc_rs_proto::packets::mob_effect::effect_id`).
    pub effect_id: i32,
    /// Amplifier (0 = level I, 1 = level II, etc.).
    pub amplifier: i32,
    /// Remaining duration in ticks.
    pub remaining_ticks: i32,
}

/// Manages all player connections and their login state machines.
pub struct ConnectionHandler {
    connections: HashMap<SocketAddr, PlayerConnection>,
    server_handle: ServerHandle,
    online_mode: bool,
    game_world: GameWorld,
    server_config: Arc<ServerConfig>,
    flat_world_blocks: FlatWorldBlocks,
    /// Overworld generator (None if using flat world).
    overworld_generator: Option<OverworldGenerator>,
    /// Pre-computed spawn position (eye position).
    spawn_position: Vec3,
    /// Pre-computed spawn block position (feet).
    spawn_block: BlockPos,
    command_registry: CommandRegistry,
    shutdown_tx: Arc<watch::Sender<bool>>,
    /// Cached world chunks — blocks persist across player interactions.
    world_chunks: HashMap<(i32, i32), ChunkColumn>,
    /// Block property registry for all vanilla blocks.
    block_registry: BlockRegistry,
    /// Item registry for all vanilla items.
    item_registry: ItemRegistry,
    /// Recipe registry for crafting.
    recipe_registry: RecipeRegistry,
    /// Permission manager: ops, whitelist, bans.
    permissions: PermissionManager,
    /// LevelDB chunk storage provider.
    chunk_storage: LevelDbProvider,
    /// World metadata (level.dat).
    level_dat: LevelDat,
    /// Path to the world directory on disk.
    world_dir: std::path::PathBuf,
    /// Tick counter for auto-save scheduling.
    save_tick_counter: u64,
    /// Auto-save interval in ticks (0 = disabled).
    auto_save_interval_ticks: u64,
    /// Pre-computed block hashes for tick processing.
    tick_blocks: TickBlocks,
    /// Scheduled block tick queue.
    tick_scheduler: TickScheduler,
    /// Plugin manager: dispatches events to loaded plugins.
    plugin_manager: PluginManager,
    /// World time in ticks (0-24000 cycle).
    world_time: i64,
    /// Whether the daylight cycle is active.
    do_daylight_cycle: bool,
    /// Whether the weather cycle is active.
    do_weather_cycle: bool,
    /// Current rain intensity (0.0-1.0).
    rain_level: f32,
    /// Current lightning intensity (0.0-1.0).
    lightning_level: f32,
    /// Smooth transition target for rain.
    rain_target: f32,
    /// Smooth transition target for lightning.
    lightning_target: f32,
    /// Ticks until next weather change.
    weather_duration: i32,
    /// Whether it is currently raining.
    is_raining: bool,
    /// Whether it is currently thundering.
    is_thundering: bool,
    /// Whether the ServerStarted plugin event has been dispatched.
    plugin_started: bool,
    /// Loaded behavior packs (for resource pack transfer to clients).
    behavior_packs: Vec<LoadedBehaviorPack>,
    /// Merged loot tables from all loaded behavior packs.
    #[allow(dead_code)]
    loot_tables: HashMap<String, LootTableFile>,
    /// Block entities (signs, chests, furnaces) keyed by world position (x, y, z).
    block_entities: HashMap<(i32, i32, i32), BlockEntityData>,
    /// Pre-computed block entity hashes for detection.
    block_entity_hashes: BlockEntityHashes,
    /// Smelting recipes and fuel data.
    smelting_registry: SmeltingRegistry,
}

impl ConnectionHandler {
    pub fn new(
        server_handle: ServerHandle,
        online_mode: bool,
        server_config: Arc<ServerConfig>,
        shutdown_tx: Arc<watch::Sender<bool>>,
    ) -> Self {
        let mut command_registry = CommandRegistry::new();
        command_registry.register_stub("gamemode", "Set a player's game mode");
        command_registry.register_stub("tp", "Teleport a player");
        command_registry.register_stub("give", "Give items to a player");
        command_registry.register_stub("kill", "Kill a player");
        command_registry.register_stub("kick", "Kick a player from the server");
        command_registry.register_stub("op", "Grant operator status");
        command_registry.register_stub("deop", "Revoke operator status");
        command_registry.register_stub("ban", "Ban a player");
        command_registry.register_stub("ban-ip", "Ban an IP address");
        command_registry.register_stub("unban", "Unban a player");
        command_registry.register_stub("unban-ip", "Unban an IP address");
        command_registry.register_stub("whitelist", "Manage the whitelist");
        command_registry.register_stub("summon", "Summon an entity");
        command_registry.register_stub("enchant", "Enchant the held item");
        command_registry.register_stub("time", "Set or query the world time");
        command_registry.register_stub("weather", "Set the weather");
        command_registry.register_stub("gamerule", "Set or query a game rule value");

        let permissions = PermissionManager::load(server_config.permissions.whitelist_enabled);

        // Initialize world generator based on config
        let gen_name = server_config.world.generator.to_lowercase();
        let overworld_generator = if gen_name == "default" || gen_name == "overworld" {
            Some(OverworldGenerator::new(server_config.world.seed as u64))
        } else {
            None
        };

        // Compute spawn position
        let (spawn_position, spawn_block) = if let Some(ref gen) = overworld_generator {
            let feet_y = gen.find_spawn_y();
            let eye_y = feet_y as f32 + 1.62;
            (Vec3::new(8.5, eye_y, 8.5), BlockPos::new(8, feet_y, 8))
        } else {
            (Vec3::new(0.5, 5.62, 0.5), BlockPos::new(0, 4, 0))
        };

        // Initialize world storage
        let world_dir = std::path::PathBuf::from(format!("worlds/{}", server_config.world.name));
        std::fs::create_dir_all(world_dir.join("db")).expect("Failed to create world db directory");
        std::fs::create_dir_all(world_dir.join("players"))
            .expect("Failed to create players directory");

        let chunk_storage =
            LevelDbProvider::open(&world_dir.join("db")).expect("Failed to open LevelDB");

        // Load or create level.dat
        let level_dat_path = world_dir.join("level.dat");
        let level_dat = if level_dat_path.exists() {
            LevelDat::load(&level_dat_path).unwrap_or_else(|e| {
                warn!("Failed to load level.dat: {e}, creating new");
                LevelDat::new(
                    &server_config.world.name,
                    server_config.world.seed,
                    &server_config.world.generator,
                    (spawn_block.x, spawn_block.y, spawn_block.z),
                )
            })
        } else {
            let dat = LevelDat::new(
                &server_config.world.name,
                server_config.world.seed,
                &server_config.world.generator,
                (spawn_block.x, spawn_block.y, spawn_block.z),
            );
            if let Err(e) = dat.save(&level_dat_path) {
                warn!("Failed to save initial level.dat: {e}");
            }
            dat
        };

        // Write levelname.txt
        std::fs::write(world_dir.join("levelname.txt"), &server_config.world.name).ok();

        let auto_save_interval_ticks = server_config.world.auto_save_interval * 20;

        // Extract weather state from level_dat before moving it
        let initial_world_time = level_dat.time;
        let initial_rain_level = level_dat.rain_level;
        let initial_lightning_level = level_dat.lightning_level;
        let initial_weather_duration = level_dat.rain_time.max(level_dat.lightning_time);

        info!(
            "World directory: {} (auto-save every {}s)",
            world_dir.display(),
            server_config.world.auto_save_interval
        );

        // Load behavior packs
        let packs_dir = std::path::PathBuf::from(&server_config.packs.directory);
        std::fs::create_dir_all(&packs_dir).ok();
        let behavior_packs = mc_rs_behavior_pack::load_all_packs(&packs_dir);

        // Build registries and register behavior pack content
        let mut block_registry = BlockRegistry::new();
        let mut item_registry = ItemRegistry::new();
        let mut recipe_registry = RecipeRegistry::new();
        let mut game_world = GameWorld::new(1);
        let mut loot_tables: HashMap<String, LootTableFile> = HashMap::new();

        for pack in &behavior_packs {
            // Register custom entities into mob registry
            for entity in &pack.entities {
                game_world
                    .mob_registry
                    .register_mob(mc_rs_game::mob_registry::MobDefinition {
                        type_id: entity.identifier.clone(),
                        display_name: entity
                            .identifier
                            .split(':')
                            .next_back()
                            .unwrap_or(&entity.identifier)
                            .to_string(),
                        category: if entity.attack_damage > 0.0 {
                            mc_rs_game::mob_registry::MobCategory::Hostile
                        } else {
                            mc_rs_game::mob_registry::MobCategory::Passive
                        },
                        max_health: entity.max_health,
                        attack_damage: entity.attack_damage,
                        movement_speed: entity.movement_speed,
                        bb_width: entity.bb_width,
                        bb_height: entity.bb_height,
                    });
            }

            // Register custom items
            for item in &pack.items {
                item_registry.register_item(
                    item.identifier.clone(),
                    item.max_stack_size,
                    item.is_component_based,
                );
            }

            // Register custom blocks
            for block in &pack.blocks {
                block_registry.register_block(
                    block.identifier.clone(),
                    block.hardness,
                    block.is_solid,
                );
            }

            // Register custom recipes
            for recipe_file in &pack.recipes {
                if let Some(ref shaped) = recipe_file.shaped {
                    let (width, height, flat_inputs) = shaped.flatten();
                    let inputs: Vec<mc_rs_game::recipe::RecipeInput> = flat_inputs
                        .iter()
                        .map(|fi| mc_rs_game::recipe::RecipeInput {
                            item_name: fi.item_name.clone(),
                            count: fi.count,
                            metadata: fi.metadata,
                        })
                        .collect();
                    let output = mc_rs_game::recipe::RecipeOutput {
                        item_name: shaped.result.item.clone(),
                        count: shaped.result.count,
                        metadata: shaped.result.data,
                    };
                    recipe_registry.register_shaped(mc_rs_game::recipe::ShapedRecipe {
                        id: shaped.description.identifier.clone(),
                        network_id: 0,
                        width,
                        height,
                        input: inputs,
                        output: vec![output],
                        tag: shaped
                            .tags
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "crafting_table".into()),
                    });
                }
                if let Some(ref shapeless) = recipe_file.shapeless {
                    let inputs: Vec<mc_rs_game::recipe::RecipeInput> = shapeless
                        .ingredients
                        .iter()
                        .map(|ing| mc_rs_game::recipe::RecipeInput {
                            item_name: ing.item.clone(),
                            count: ing.count,
                            metadata: ing.data,
                        })
                        .collect();
                    let output = mc_rs_game::recipe::RecipeOutput {
                        item_name: shapeless.result.item.clone(),
                        count: shapeless.result.count,
                        metadata: shapeless.result.data,
                    };
                    recipe_registry.register_shapeless(mc_rs_game::recipe::ShapelessRecipe {
                        id: shapeless.description.identifier.clone(),
                        network_id: 0,
                        inputs,
                        output: vec![output],
                        tag: shapeless
                            .tags
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "crafting_table".into()),
                    });
                }
            }

            // Merge loot tables
            for (key, table) in &pack.loot_tables {
                loot_tables.insert(key.clone(), table.clone());
            }
        }

        Self {
            connections: HashMap::new(),
            server_handle,
            online_mode,
            game_world,
            server_config,
            flat_world_blocks: FlatWorldBlocks::compute(),
            overworld_generator,
            spawn_position,
            spawn_block,
            command_registry,
            shutdown_tx,
            world_chunks: HashMap::new(),
            block_registry,
            item_registry,
            recipe_registry,
            permissions,
            chunk_storage,
            level_dat,
            world_dir,
            save_tick_counter: 0,
            auto_save_interval_ticks,
            tick_blocks: TickBlocks::compute(),
            tick_scheduler: TickScheduler::new(),
            world_time: initial_world_time,
            do_daylight_cycle: true,
            do_weather_cycle: true,
            rain_level: initial_rain_level,
            lightning_level: initial_lightning_level,
            rain_target: initial_rain_level,
            lightning_target: initial_lightning_level,
            weather_duration: initial_weather_duration,
            is_raining: initial_rain_level > 0.0,
            is_thundering: initial_lightning_level > 0.0,
            plugin_manager: {
                let mut mgr = PluginManager::new();
                // Load WASM plugins from plugins/ directory
                let plugins_dir = std::path::PathBuf::from("plugins");
                std::fs::create_dir_all(&plugins_dir).ok();
                let engine = mc_rs_plugin_wasm::create_engine();
                let wasm_plugins = mc_rs_plugin_wasm::load_wasm_plugins(&plugins_dir, &engine);
                for plugin in wasm_plugins {
                    mgr.register(plugin);
                }
                // Load Lua plugins from plugins/ directory
                let lua_plugins = mc_rs_plugin_lua::load_lua_plugins(&plugins_dir);
                for plugin in lua_plugins {
                    mgr.register(plugin);
                }
                mgr
            },
            plugin_started: false,
            behavior_packs,
            loot_tables,
            block_entities: HashMap::new(),
            block_entity_hashes: BlockEntityHashes::compute(),
            smelting_registry: SmeltingRegistry::new(),
        }
    }

    pub(super) fn allocate_entity_id(&mut self) -> i64 {
        self.game_world.allocate_entity_id()
    }

    /// Generate a chunk using the appropriate generator (overworld or flat).
    pub(super) fn generate_chunk(&self, cx: i32, cz: i32) -> ChunkColumn {
        if let Some(ref gen) = self.overworld_generator {
            gen.generate_chunk(cx, cz)
        } else {
            generate_flat_chunk(cx, cz, &self.flat_world_blocks)
        }
    }

    // ─── Plugin helpers ────────────────────────────────────────────────────

    /// Build a PluginPlayer from a PlayerConnection.
    pub(super) fn make_plugin_player(conn: &PlayerConnection) -> PluginPlayer {
        let name = conn
            .login_data
            .as_ref()
            .map(|d| d.display_name.clone())
            .unwrap_or_default();
        let uuid = conn
            .login_data
            .as_ref()
            .map(|d| d.identity.clone())
            .unwrap_or_default();
        PluginPlayer {
            name,
            uuid,
            runtime_id: conn.entity_runtime_id,
            position: (conn.position.x, conn.position.y, conn.position.z),
            gamemode: conn.gamemode,
            health: conn.health,
        }
    }

    /// Build a read-only snapshot of server state for plugin API.
    pub(super) fn build_snapshot(&mut self) -> ServerSnapshot {
        let players: Vec<PluginPlayer> = self
            .connections
            .values()
            .filter(|c| c.state == LoginState::InGame)
            .map(Self::make_plugin_player)
            .collect();
        ServerSnapshot {
            players,
            world_time: self.world_time,
            current_tick: self.game_world.current_tick(),
            is_raining: self.is_raining,
        }
    }

    /// Process a RakNet event.
    pub async fn handle_event(&mut self, event: RakNetEvent) {
        match event {
            RakNetEvent::SessionConnected { addr, guid } => {
                self.handle_session_connected(addr, guid);
            }
            RakNetEvent::SessionDisconnected { addr } => {
                self.handle_session_disconnected(addr).await;
            }
            RakNetEvent::Packet { addr, payload } => {
                self.handle_packet(addr, payload).await;
            }
        }
        // Immediately process any game events generated during packet handling
        // (e.g. mob spawns, damage) so clients see results without waiting for next tick.
        self.process_game_events().await;
    }

    /// Run one ECS game tick (called every 50ms from main loop) and process outgoing events.
    pub async fn game_tick(&mut self) {
        self.game_world.tick();
        self.process_game_events().await;
        self.tick_effects().await;
        self.tick_survival().await;
        self.tick_block_updates().await;
        self.tick_furnaces().await;
        self.tick_time_and_weather().await;

        // Plugin: dispatch ServerStarted on first tick
        if !self.plugin_started {
            self.plugin_started = true;
            let snapshot = self.build_snapshot();
            let (_, actions) = self
                .plugin_manager
                .dispatch(&PluginEvent::ServerStarted, &snapshot);
            self.apply_plugin_actions(actions).await;
        }

        // Plugin scheduler tick
        let plugin_actions = {
            let snapshot = self.build_snapshot();
            self.plugin_manager.tick_scheduler(&snapshot)
        };
        self.apply_plugin_actions(plugin_actions).await;

        // Auto-save
        if self.auto_save_interval_ticks > 0 {
            self.save_tick_counter += 1;
            if self.save_tick_counter >= self.auto_save_interval_ticks {
                self.save_tick_counter = 0;
                self.save_all();
            }
        }
    }

    // -----------------------------------------------------------------------
    // Utility helpers used by sub-modules
    // -----------------------------------------------------------------------

    /// Find a player's SocketAddr by display name.
    pub(super) fn find_player_addr(&self, name: &str) -> Option<SocketAddr> {
        self.connections.iter().find_map(|(&addr, conn)| {
            if conn.state == LoginState::InGame {
                if let Some(ref data) = conn.login_data {
                    if data.display_name == name {
                        return Some(addr);
                    }
                }
            }
            None
        })
    }

    /// Collect PlayerInfo for all online (InGame) players.
    pub(super) fn online_player_infos(&self) -> Vec<PlayerInfo> {
        self.connections
            .values()
            .filter(|c| c.state == LoginState::InGame)
            .filter_map(|c| {
                let name = c.login_data.as_ref()?.display_name.clone();
                Some(PlayerInfo {
                    name,
                    x: c.position.x,
                    y: c.position.y,
                    z: c.position.z,
                })
            })
            .collect()
    }

    /// Resolve a target argument (selector or player name).
    pub(super) fn resolve_target(
        &self,
        target: &str,
        addr: SocketAddr,
    ) -> Result<Vec<String>, String> {
        let conn = self
            .connections
            .get(&addr)
            .ok_or_else(|| "Sender not found".to_string())?;
        let sender_name = conn
            .login_data
            .as_ref()
            .map(|d| d.display_name.as_str())
            .unwrap_or("unknown");
        let sender_pos = (conn.position.x, conn.position.y, conn.position.z);
        let players = self.online_player_infos();
        mc_rs_command::selector::resolve_target(target, sender_name, sender_pos, &players)
    }

    /// Find a player's address by their entity runtime ID.
    pub(super) fn find_addr_by_runtime_id(&self, runtime_id: u64) -> Option<SocketAddr> {
        self.connections.iter().find_map(|(&a, conn)| {
            if conn.entity_runtime_id == runtime_id && conn.state == LoginState::InGame {
                Some(a)
            } else {
                None
            }
        })
    }

    pub(super) async fn broadcast_packet(&mut self, packet_id: u32, packet: &impl ProtoEncode) {
        let addrs: Vec<SocketAddr> = self
            .connections
            .iter()
            .filter(|(_, c)| c.state == LoginState::InGame)
            .map(|(&a, _)| a)
            .collect();
        for addr in addrs {
            self.send_packet(addr, packet_id, packet).await;
        }
    }

    pub(super) async fn broadcast_packet_except(
        &mut self,
        except: SocketAddr,
        packet_id: u32,
        packet: &impl ProtoEncode,
    ) {
        let addrs: Vec<SocketAddr> = self
            .connections
            .iter()
            .filter(|(&a, c)| a != except && c.state == LoginState::InGame)
            .map(|(&a, _)| a)
            .collect();
        for addr in addrs {
            self.send_packet(addr, packet_id, packet).await;
        }
    }

    // -----------------------------------------------------------------------
    // Phase 1.2: Block breaking & placing
    // -----------------------------------------------------------------------

    /// Maximum Y coordinate in the Overworld.
    const MAX_Y: i32 = OVERWORLD_MIN_Y + (OVERWORLD_SUB_CHUNK_COUNT as i32) * 16 - 1; // 319

    /// Get the block runtime ID at a world position.
    pub(super) fn get_block(&self, x: i32, y: i32, z: i32) -> Option<u32> {
        let cx = x >> 4;
        let cz = z >> 4;
        let column = self.world_chunks.get(&(cx, cz))?;

        let sub_index = (y - OVERWORLD_MIN_Y) / 16;
        if sub_index < 0 || sub_index >= OVERWORLD_SUB_CHUNK_COUNT as i32 {
            return None;
        }
        let local_x = (x & 15) as usize;
        let local_y = ((y - OVERWORLD_MIN_Y) % 16) as usize;
        let local_z = (z & 15) as usize;

        Some(column.sub_chunks[sub_index as usize].get_block(local_x, local_y, local_z))
    }

    /// Set a block at a world position. Returns false if the chunk is not loaded.
    pub(super) fn set_block(&mut self, x: i32, y: i32, z: i32, runtime_id: u32) -> bool {
        let cx = x >> 4;
        let cz = z >> 4;
        let column = match self.world_chunks.get_mut(&(cx, cz)) {
            Some(c) => c,
            None => return false,
        };

        let sub_index = (y - OVERWORLD_MIN_Y) / 16;
        if sub_index < 0 || sub_index >= OVERWORLD_SUB_CHUNK_COUNT as i32 {
            return false;
        }
        let local_x = (x & 15) as usize;
        let local_y = ((y - OVERWORLD_MIN_Y) % 16) as usize;
        let local_z = (z & 15) as usize;

        column.sub_chunks[sub_index as usize].set_block(local_x, local_y, local_z, runtime_id);
        column.dirty = true;
        true
    }

    /// Compute the target position when placing a block on a face.
    pub(super) fn face_offset(pos: BlockPos, face: i32) -> BlockPos {
        match face {
            0 => BlockPos::new(pos.x, pos.y - 1, pos.z), // Down
            1 => BlockPos::new(pos.x, pos.y + 1, pos.z), // Up
            2 => BlockPos::new(pos.x, pos.y, pos.z - 1), // North
            3 => BlockPos::new(pos.x, pos.y, pos.z + 1), // South
            4 => BlockPos::new(pos.x - 1, pos.y, pos.z), // West
            5 => BlockPos::new(pos.x + 1, pos.y, pos.z), // East
            _ => pos,
        }
    }

    // -----------------------------------------------------------------------
    // Packet sending
    // -----------------------------------------------------------------------

    pub(super) async fn send_packet(
        &mut self,
        addr: SocketAddr,
        packet_id: u32,
        packet: &impl ProtoEncode,
    ) {
        let (batch_config, has_encryption) = match self.connections.get(&addr) {
            Some(c) => (c.batch_config.clone(), c.encryption.is_some()),
            None => return,
        };

        let sub_packet = encode_sub_packet(packet_id, packet);
        let batch = match encode_single(sub_packet, &batch_config) {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to encode packet 0x{packet_id:02X} for {addr}: {e}");
                return;
            }
        };

        let final_payload = if has_encryption {
            match self.connections.get_mut(&addr) {
                Some(conn) => match conn.encryption.as_mut() {
                    Some(enc) => enc.encrypt(&batch),
                    None => batch,
                },
                None => return,
            }
        } else {
            batch
        };

        let mut out = BytesMut::with_capacity(1 + final_payload.len());
        out.put_u8(0xFE);
        out.put_slice(&final_payload);

        self.server_handle
            .send_to(addr, out.freeze(), Reliability::ReliableOrdered, 0)
            .await;
    }

    /// Save all dirty chunks, online player data, and level.dat to disk.
    pub fn save_all(&mut self) {
        // Plugin event: ServerStopping (synchronous, actions not applied)
        {
            let snapshot = self.build_snapshot();
            let (_, _actions) = self
                .plugin_manager
                .dispatch(&PluginEvent::ServerStopping, &snapshot);
        }
        self.plugin_manager.disable_all();

        // Save dirty chunks
        let dirty_keys: Vec<(i32, i32)> = self
            .world_chunks
            .iter()
            .filter(|(_, col)| col.dirty)
            .map(|(&k, _)| k)
            .collect();

        let chunk_count = dirty_keys.len();
        for key in &dirty_keys {
            if let Some(col) = self.world_chunks.get(key) {
                if let Err(e) = self.chunk_storage.save_chunk(col) {
                    warn!("Failed to save chunk ({},{}): {e}", key.0, key.1);
                }
            }
        }
        // Mark saved chunks as clean
        for key in &dirty_keys {
            if let Some(col) = self.world_chunks.get_mut(key) {
                col.dirty = false;
            }
        }

        // Save block entities grouped by chunk
        let mut be_by_chunk: HashMap<(i32, i32), Vec<u8>> = HashMap::new();
        for (&(bx, by, bz), be) in &self.block_entities {
            let cx = bx >> 4;
            let cz = bz >> 4;
            be_by_chunk
                .entry((cx, cz))
                .or_default()
                .extend_from_slice(&be.to_le_nbt(bx, by, bz));
        }
        for ((cx, cz), data) in &be_by_chunk {
            let key = block_entity_key(*cx, *cz);
            if let Err(e) = self.chunk_storage.put_raw(&key, data) {
                warn!(
                    "Failed to save block entities for chunk ({},{}): {e}",
                    cx, cz
                );
            }
        }

        if let Err(e) = self.chunk_storage.flush() {
            warn!("Failed to flush LevelDB: {e}");
        }

        // Save all online players
        let mut player_count = 0u32;
        let player_entries: Vec<(SocketAddr, String)> = self
            .connections
            .iter()
            .filter(|(_, c)| c.state == LoginState::InGame)
            .filter_map(|(&addr, c)| c.login_data.as_ref().map(|d| (addr, d.identity.clone())))
            .collect();

        for (addr, uuid) in &player_entries {
            if let Some(conn) = self.connections.get(addr) {
                let data = PlayerData::from_connection(conn);
                if let Err(e) = data.save(&self.world_dir, uuid) {
                    warn!("Failed to save player {uuid}: {e}");
                } else {
                    player_count += 1;
                }
            }
        }

        // Persist weather + time state
        self.level_dat.time = self.world_time;
        self.level_dat.rain_level = self.rain_level;
        self.level_dat.lightning_level = self.lightning_level;
        self.level_dat.rain_time = self.weather_duration;
        self.level_dat.lightning_time = self.weather_duration;

        // Update and save level.dat
        let tick = self.game_world.current_tick();
        self.level_dat.update_on_save(tick);
        if let Err(e) = self.level_dat.save(&self.world_dir.join("level.dat")) {
            warn!("Failed to save level.dat: {e}");
        }

        info!("World saved: {chunk_count} chunks, {player_count} players");
    }
}

/// Map an effect name to its Bedrock protocol ID.
pub(super) fn effect_name_to_id(name: &str) -> Option<i32> {
    use mc_rs_proto::packets::mob_effect::effect_id;
    match name.to_lowercase().as_str() {
        "speed" => Some(effect_id::SPEED),
        "slowness" => Some(effect_id::SLOWNESS),
        "haste" => Some(effect_id::HASTE),
        "mining_fatigue" => Some(effect_id::MINING_FATIGUE),
        "strength" => Some(effect_id::STRENGTH),
        "instant_health" => Some(effect_id::INSTANT_HEALTH),
        "instant_damage" => Some(effect_id::INSTANT_DAMAGE),
        "jump_boost" => Some(effect_id::JUMP_BOOST),
        "nausea" => Some(effect_id::NAUSEA),
        "regeneration" => Some(effect_id::REGENERATION),
        "resistance" => Some(effect_id::RESISTANCE),
        "fire_resistance" => Some(effect_id::FIRE_RESISTANCE),
        "water_breathing" => Some(effect_id::WATER_BREATHING),
        "invisibility" => Some(effect_id::INVISIBILITY),
        "blindness" => Some(effect_id::BLINDNESS),
        "night_vision" => Some(effect_id::NIGHT_VISION),
        "hunger" => Some(effect_id::HUNGER),
        "weakness" => Some(effect_id::WEAKNESS),
        "poison" => Some(effect_id::POISON),
        "wither" => Some(effect_id::WITHER),
        "absorption" => Some(effect_id::ABSORPTION),
        _ => None,
    }
}

/// Encode a packet struct into a sub-packet: `VarUInt32(id) + proto_encoded fields`.
pub(super) fn encode_sub_packet(packet_id: u32, packet: &impl ProtoEncode) -> Bytes {
    let mut buf = BytesMut::new();
    VarUInt32(packet_id).proto_encode(&mut buf);
    packet.proto_encode(&mut buf);
    buf.freeze()
}

pub(super) fn gamemode_from_str(s: &str) -> i32 {
    match s.to_lowercase().as_str() {
        "survival" => 0,
        "creative" => 1,
        "adventure" => 2,
        "spectator" => 3,
        _ => 0,
    }
}

pub(super) fn difficulty_from_str(s: &str) -> i32 {
    match s.to_lowercase().as_str() {
        "peaceful" => 0,
        "easy" => 1,
        "normal" => 2,
        "hard" => 3,
        _ => 2,
    }
}

pub(super) fn generator_from_str(s: &str) -> i32 {
    match s.to_lowercase().as_str() {
        "legacy" => 0,
        "overworld" | "default" => 1,
        "flat" => 2,
        "nether" => 3,
        "end" => 4,
        "void" => 5,
        _ => 2,
    }
}

pub(super) fn parse_gamemode(s: &str) -> Option<i32> {
    match s.to_lowercase().as_str() {
        "0" | "survival" | "s" => Some(0),
        "1" | "creative" | "c" => Some(1),
        "2" | "adventure" | "a" => Some(2),
        "3" | "spectator" | "sp" => Some(3),
        _ => None,
    }
}

pub(super) fn gamemode_name(gm: i32) -> &'static str {
    match gm {
        0 => "Survival",
        1 => "Creative",
        2 => "Adventure",
        3 => "Spectator",
        _ => "Unknown",
    }
}

pub(super) fn parse_coords(xs: &str, ys: &str, zs: &str) -> Option<(f32, f32, f32)> {
    let x = xs.parse::<f32>().ok()?;
    let y = ys.parse::<f32>().ok()?;
    let z = zs.parse::<f32>().ok()?;
    Some((x, y, z))
}

/// Return base attack damage for a held item, looked up by runtime ID.
pub(super) fn base_attack_damage(registry: &ItemRegistry, runtime_id: i32) -> f32 {
    if runtime_id <= 0 {
        return 1.0; // fist / air
    }
    let name = match registry.get_by_id(runtime_id as i16) {
        Some(info) => info.name.as_str(),
        None => return 1.0,
    };
    match name {
        // Swords
        "minecraft:wooden_sword" => 5.0,
        "minecraft:stone_sword" => 6.0,
        "minecraft:iron_sword" => 7.0,
        "minecraft:golden_sword" => 5.0,
        "minecraft:diamond_sword" => 8.0,
        "minecraft:netherite_sword" => 9.0,
        // Axes
        "minecraft:wooden_axe" => 4.0,
        "minecraft:stone_axe" => 5.0,
        "minecraft:iron_axe" => 6.0,
        "minecraft:golden_axe" => 4.0,
        "minecraft:diamond_axe" => 7.0,
        "minecraft:netherite_axe" => 8.0,
        // Pickaxes
        "minecraft:wooden_pickaxe" => 3.0,
        "minecraft:stone_pickaxe" => 4.0,
        "minecraft:iron_pickaxe" => 5.0,
        "minecraft:golden_pickaxe" => 3.0,
        "minecraft:diamond_pickaxe" => 6.0,
        "minecraft:netherite_pickaxe" => 7.0,
        // Shovels
        "minecraft:wooden_shovel" => 2.0,
        "minecraft:stone_shovel" => 3.0,
        "minecraft:iron_shovel" => 4.0,
        "minecraft:golden_shovel" => 2.0,
        "minecraft:diamond_shovel" => 5.0,
        "minecraft:netherite_shovel" => 6.0,
        // Trident
        "minecraft:trident" => 9.0,
        // Everything else (hoes, misc items, blocks)
        _ => 1.0,
    }
}

/// Build default entity metadata for a mob (FLAGS, SCALE, bounding box).
pub(super) fn default_mob_metadata(bb_width: f32, bb_height: f32) -> Vec<EntityMetadataEntry> {
    vec![
        EntityMetadataEntry {
            key: 0,
            data_type: 7,
            value: MetadataValue::Long(0), // FLAGS
        },
        EntityMetadataEntry {
            key: 23,
            data_type: 3,
            value: MetadataValue::Float(1.0), // SCALE
        },
        EntityMetadataEntry {
            key: 38,
            data_type: 3,
            value: MetadataValue::Float(bb_width), // BB_WIDTH
        },
        EntityMetadataEntry {
            key: 39,
            data_type: 3,
            value: MetadataValue::Float(bb_height), // BB_HEIGHT
        },
    ]
}

/// Build metadata for a baby mob (half scale, BABY flag bit).
pub(super) fn baby_mob_metadata(bb_width: f32, bb_height: f32) -> Vec<EntityMetadataEntry> {
    vec![
        EntityMetadataEntry {
            key: 0,
            data_type: 7,
            value: MetadataValue::Long(1 << 8), // FLAGS: BABY bit
        },
        EntityMetadataEntry {
            key: 23,
            data_type: 3,
            value: MetadataValue::Float(0.5), // SCALE: half size
        },
        EntityMetadataEntry {
            key: 38,
            data_type: 3,
            value: MetadataValue::Float(bb_width), // BB_WIDTH
        },
        EntityMetadataEntry {
            key: 39,
            data_type: 3,
            value: MetadataValue::Float(bb_height), // BB_HEIGHT
        },
    ]
}
