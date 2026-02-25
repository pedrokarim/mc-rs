//! Per-player connection state management and login flow.

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
use mc_rs_game::combat;
use mc_rs_game::game_world::{GameEvent, GameWorld};
use mc_rs_game::inventory::PlayerInventory;
use mc_rs_proto::batch::{decode_batch, encode_single, BatchConfig};
use mc_rs_proto::codec::{ProtoDecode, ProtoEncode};
use mc_rs_proto::compression::CompressionAlgorithm;
use mc_rs_proto::jwt;
use mc_rs_proto::packets::add_player::default_player_metadata;
use mc_rs_proto::packets::{
    self, ActorAttribute, AddActor, AddPlayer, Animate, AvailableCommands,
    AvailableEntityIdentifiers, BiomeDefinitionList, ChunkRadiusUpdated, ClientToServerHandshake,
    CommandOutput, CommandRequest, Disconnect, EntityEvent, EntityMetadataEntry, InventoryContent,
    InventorySlot, InventoryTransaction, ItemStackRequest, ItemStackResponse, LevelChunk,
    LevelEvent, MetadataValue, MobEffect, MobEquipment, MoveActorAbsolute, MoveMode, MovePlayer,
    NetworkChunkPublisherUpdate, NetworkSettings, PlayStatus, PlayStatusType, PlayerAction,
    PlayerActionType, PlayerAuthInput, PlayerListAdd, PlayerListAddPacket, PlayerListRemove,
    RemoveEntity, RequestChunkRadius, ResourcePackClientResponse, ResourcePackResponseStatus,
    ResourcePackStack, ResourcePacksInfo, Respawn, ServerToClientHandshake, SetEntityMotion,
    SetLocalPlayerAsInitialized, SetPlayerGameType, StartGame, Text, UpdateAbilities,
    UpdateAttributes, UpdateBlock, UseItemAction, UseItemOnEntityAction,
};
use mc_rs_proto::types::{BlockPos, Uuid, VarUInt32, Vec2, Vec3};
use mc_rs_raknet::{RakNetEvent, Reliability, ServerHandle};
use mc_rs_world::block_hash::FlatWorldBlocks;
use mc_rs_world::block_registry::BlockRegistry;
use mc_rs_world::chunk::{ChunkColumn, OVERWORLD_MIN_Y, OVERWORLD_SUB_CHUNK_COUNT};
use mc_rs_world::flat_generator::generate_flat_chunk;
use mc_rs_world::item_registry::ItemRegistry;
use mc_rs_world::physics::{PlayerAabb, MAX_AIRBORNE_TICKS, MAX_FALL_PER_TICK};
use mc_rs_world::serializer::serialize_chunk_column;
use tokio::sync::watch;

use crate::config::ServerConfig;
use crate::permissions::{BanEntry, PermissionManager};

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
    command_registry: CommandRegistry,
    shutdown_tx: Arc<watch::Sender<bool>>,
    /// Cached world chunks — blocks persist across player interactions.
    world_chunks: HashMap<(i32, i32), ChunkColumn>,
    /// Block property registry for all vanilla blocks.
    block_registry: BlockRegistry,
    /// Item registry for all vanilla items.
    item_registry: ItemRegistry,
    /// Permission manager: ops, whitelist, bans.
    permissions: PermissionManager,
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

        let permissions = PermissionManager::load(server_config.permissions.whitelist_enabled);

        Self {
            connections: HashMap::new(),
            server_handle,
            online_mode,
            game_world: GameWorld::new(1),
            server_config,
            flat_world_blocks: FlatWorldBlocks::compute(),
            command_registry,
            shutdown_tx,
            world_chunks: HashMap::new(),
            block_registry: BlockRegistry::new(),
            item_registry: ItemRegistry::new(),
            permissions,
        }
    }

    fn allocate_entity_id(&mut self) -> i64 {
        self.game_world.allocate_entity_id()
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
    }

    /// Drain ECS game events and send the corresponding packets.
    async fn process_game_events(&mut self) {
        let events = self.game_world.drain_events();
        for event in events {
            match event {
                GameEvent::MobSpawned {
                    runtime_id,
                    unique_id,
                    mob_type,
                    position,
                    health,
                    max_health,
                    bb_width,
                    bb_height,
                } => {
                    let pkt = AddActor {
                        entity_unique_id: unique_id,
                        entity_runtime_id: runtime_id,
                        entity_type: mob_type,
                        position: Vec3::new(position.0, position.1, position.2),
                        velocity: Vec3::ZERO,
                        pitch: 0.0,
                        yaw: 0.0,
                        head_yaw: 0.0,
                        body_yaw: 0.0,
                        attributes: vec![ActorAttribute {
                            name: "minecraft:health".to_string(),
                            min: 0.0,
                            max: max_health,
                            current: health,
                            default: max_health,
                        }],
                        metadata: default_mob_metadata(bb_width, bb_height),
                    };
                    self.broadcast_packet(packets::id::ADD_ACTOR, &pkt).await;
                }
                GameEvent::MobMoved {
                    runtime_id,
                    position,
                    pitch,
                    yaw,
                    head_yaw,
                    on_ground,
                } => {
                    let pkt = MoveActorAbsolute::normal(
                        runtime_id,
                        Vec3::new(position.0, position.1, position.2),
                        pitch,
                        yaw,
                        head_yaw,
                        on_ground,
                    );
                    self.broadcast_packet(packets::id::MOVE_ACTOR_ABSOLUTE, &pkt)
                        .await;
                }
                GameEvent::MobHurt {
                    runtime_id,
                    new_health,
                    tick,
                } => {
                    self.broadcast_packet(
                        packets::id::ENTITY_EVENT,
                        &EntityEvent::hurt(runtime_id),
                    )
                    .await;
                    self.broadcast_packet(
                        packets::id::UPDATE_ATTRIBUTES,
                        &UpdateAttributes::health(runtime_id, new_health, tick),
                    )
                    .await;
                }
                GameEvent::MobDied {
                    runtime_id,
                    unique_id,
                } => {
                    self.broadcast_packet(
                        packets::id::ENTITY_EVENT,
                        &EntityEvent::death(runtime_id),
                    )
                    .await;
                    self.broadcast_packet(
                        packets::id::REMOVE_ENTITY,
                        &RemoveEntity {
                            entity_unique_id: unique_id,
                        },
                    )
                    .await;
                }
                GameEvent::EntityRemoved { unique_id } => {
                    self.broadcast_packet(
                        packets::id::REMOVE_ENTITY,
                        &RemoveEntity {
                            entity_unique_id: unique_id,
                        },
                    )
                    .await;
                }
                GameEvent::MobAttackPlayer {
                    mob_runtime_id: _,
                    target_runtime_id,
                    damage: raw_damage,
                    knockback,
                } => {
                    // Find the target player by runtime_id
                    let target_addr = self
                        .connections
                        .iter()
                        .find(|(_, c)| c.entity_runtime_id == target_runtime_id)
                        .map(|(a, _)| *a);
                    if let Some(addr) = target_addr {
                        // Invulnerability check
                        let tick = match self.connections.get(&addr) {
                            Some(c) => c.client_tick,
                            None => continue,
                        };
                        if let Some(last) =
                            self.connections.get(&addr).and_then(|c| c.last_damage_tick)
                        {
                            if tick.saturating_sub(last) < 10 {
                                continue;
                            }
                        }

                        // Apply armor + protection + resistance reduction
                        let (armor_defense, armor_nbt_slots) = {
                            let conn = match self.connections.get(&addr) {
                                Some(c) => c,
                                None => continue,
                            };
                            let defense = combat::total_armor_defense(
                                &self.item_registry,
                                &conn.inventory.armor,
                            );
                            let nbt: Vec<Vec<u8>> = conn
                                .inventory
                                .armor
                                .iter()
                                .map(|i| i.nbt_data.clone())
                                .collect();
                            (defense, nbt)
                        };
                        let armor_refs: Vec<&[u8]> =
                            armor_nbt_slots.iter().map(|v| v.as_slice()).collect();
                        let resistance = self.get_resistance_factor(addr);

                        // Mob attacks have no weapon enchantments or criticals
                        let damage = combat::calculate_damage(&combat::DamageInput {
                            base_damage: raw_damage,
                            weapon_nbt: &[],
                            armor_defense,
                            armor_nbt_slots: &armor_refs,
                            is_critical: false,
                            strength_bonus: 0.0,
                            weakness_penalty: 0.0,
                            resistance_factor: resistance,
                        });

                        // Apply damage
                        let conn = match self.connections.get_mut(&addr) {
                            Some(c) => c,
                            None => continue,
                        };
                        conn.health = (conn.health - damage).max(0.0);
                        conn.last_damage_tick = Some(tick);
                        let new_health = conn.health;
                        let runtime_id = conn.entity_runtime_id;
                        let is_dead = new_health <= 0.0;

                        // Send hurt animation
                        self.broadcast_packet(
                            packets::id::ENTITY_EVENT,
                            &EntityEvent::hurt(runtime_id),
                        )
                        .await;

                        // Send updated health
                        self.broadcast_packet(
                            packets::id::UPDATE_ATTRIBUTES,
                            &UpdateAttributes::health(runtime_id, new_health, tick),
                        )
                        .await;

                        // Knockback
                        self.broadcast_packet(
                            packets::id::SET_ENTITY_MOTION,
                            &SetEntityMotion {
                                entity_runtime_id: runtime_id,
                                motion: Vec3::new(knockback.0, knockback.1, knockback.2),
                            },
                        )
                        .await;

                        if is_dead {
                            // Death flow
                            let conn = self.connections.get_mut(&addr).unwrap();
                            conn.is_dead = true;
                            conn.health = 0.0;

                            self.broadcast_packet(
                                packets::id::ENTITY_EVENT,
                                &EntityEvent::death(runtime_id),
                            )
                            .await;

                            self.send_packet(
                                addr,
                                packets::id::RESPAWN,
                                &Respawn {
                                    position: Vec3::new(0.5, 5.62, 0.5),
                                    state: 0,
                                    runtime_entity_id: runtime_id,
                                },
                            )
                            .await;
                        }
                    }
                }
            }
        }
    }

    fn handle_session_connected(&mut self, addr: SocketAddr, guid: i64) {
        info!("Session connected: {addr} (GUID: {guid})");
        let entity_id = self.allocate_entity_id();
        self.connections.insert(
            addr,
            PlayerConnection {
                state: LoginState::AwaitingNetworkSettings,
                batch_config: BatchConfig::default(),
                login_data: None,
                client_data: None,
                encryption: None,
                pending_encryption: None,
                entity_unique_id: entity_id,
                entity_runtime_id: entity_id as u64,
                position: Vec3::new(0.5, 5.62, 0.5),
                pitch: 0.0,
                yaw: 0.0,
                head_yaw: 0.0,
                on_ground: true,
                client_tick: 0,
                sent_chunks: HashSet::new(),
                chunk_radius: 0,
                gamemode: gamemode_from_str(&self.server_config.server.gamemode),
                breaking_block: None,
                airborne_ticks: 0,
                inventory: PlayerInventory::new(),
                health: 20.0,
                last_damage_tick: None,
                is_dead: false,
                is_sprinting: false,
                effects: Vec::new(),
                last_position_delta_y: 0.0,
                fire_ticks: 0,
            },
        );

        // Spawn ECS mirror entity for this player
        self.game_world
            .spawn_player(entity_id, entity_id as u64, (0.5, 5.62, 0.5), addr);
    }

    async fn handle_session_disconnected(&mut self, addr: SocketAddr) {
        // Collect data before removing from connections
        let (was_in_game, entity_unique_id, uuid, display_name) = match self.connections.get(&addr)
        {
            Some(conn) => {
                let in_game = conn.state == LoginState::InGame;
                let uid = conn.entity_unique_id;
                let (uuid, name) = match &conn.login_data {
                    Some(d) => (
                        Uuid::parse(&d.identity).unwrap_or(Uuid::ZERO),
                        d.display_name.clone(),
                    ),
                    None => (Uuid::ZERO, String::new()),
                };
                (in_game, uid, uuid, name)
            }
            None => return,
        };

        // Remove the connection
        self.connections.remove(&addr);

        // Despawn the ECS mirror entity for this player
        self.game_world.despawn_player(entity_unique_id);

        if was_in_game {
            // Broadcast RemoveEntity to all remaining players
            self.broadcast_packet(
                packets::id::REMOVE_ENTITY,
                &RemoveEntity { entity_unique_id },
            )
            .await;

            // Broadcast PlayerList(Remove) to all remaining players
            self.broadcast_packet(
                packets::id::PLAYER_LIST,
                &PlayerListRemove { uuids: vec![uuid] },
            )
            .await;

            // Broadcast leave message
            let leave_msg = Text::system(format!("{display_name} left the game"));
            self.broadcast_packet(packets::id::TEXT, &leave_msg).await;

            info!("Player {display_name} disconnected ({addr})");
        } else {
            info!("Session disconnected: {addr}");
        }
    }

    async fn handle_packet(&mut self, addr: SocketAddr, payload: Bytes) {
        // Decrypt if encryption is active
        let decrypted = {
            let conn = match self.connections.get_mut(&addr) {
                Some(c) => c,
                None => {
                    warn!("Packet from unknown session {addr}");
                    return;
                }
            };
            if let Some(enc) = conn.encryption.as_mut() {
                match enc.decrypt(&payload) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Decrypt error from {addr}: {e}");
                        return;
                    }
                }
            } else {
                payload
            }
        };

        // Clone batch_config to avoid borrow issues
        let batch_config = match self.connections.get(&addr) {
            Some(conn) => conn.batch_config.clone(),
            None => return,
        };

        let sub_packets = match decode_batch(decrypted, &batch_config) {
            Ok(p) => p,
            Err(e) => {
                warn!("Batch decode error from {addr}: {e}");
                return;
            }
        };

        for sub_packet in sub_packets {
            let mut cursor = Cursor::new(&sub_packet[..]);
            let packet_id = match VarUInt32::proto_decode(&mut cursor) {
                Ok(id) => id.0,
                Err(e) => {
                    warn!("Bad packet ID from {addr}: {e}");
                    continue;
                }
            };

            match packet_id {
                packets::id::REQUEST_NETWORK_SETTINGS => {
                    self.handle_request_network_settings(addr, &mut cursor)
                        .await;
                }
                packets::id::LOGIN => {
                    self.handle_login(addr, &mut cursor).await;
                }
                packets::id::CLIENT_TO_SERVER_HANDSHAKE => {
                    self.handle_client_to_server_handshake(addr, &mut cursor)
                        .await;
                }
                packets::id::RESOURCE_PACK_CLIENT_RESPONSE => {
                    self.handle_resource_pack_client_response(addr, &mut cursor)
                        .await;
                }
                packets::id::REQUEST_CHUNK_RADIUS => {
                    self.handle_request_chunk_radius(addr, &mut cursor).await;
                }
                packets::id::SET_LOCAL_PLAYER_AS_INITIALIZED => {
                    self.handle_set_local_player_as_initialized(addr, &mut cursor)
                        .await;
                }
                packets::id::TEXT => {
                    self.handle_text(addr, &mut cursor).await;
                }
                packets::id::COMMAND_REQUEST => {
                    self.handle_command_request(addr, &mut cursor).await;
                }
                packets::id::MOB_EQUIPMENT => {
                    self.handle_mob_equipment(addr, &mut cursor).await;
                }
                packets::id::INVENTORY_TRANSACTION => {
                    self.handle_inventory_transaction(addr, &mut cursor).await;
                }
                packets::id::PLAYER_ACTION => {
                    self.handle_player_action(addr, &mut cursor).await;
                }
                packets::id::ITEM_STACK_REQUEST => {
                    self.handle_item_stack_request(addr, &mut cursor).await;
                }
                packets::id::PLAYER_AUTH_INPUT => {
                    self.handle_player_auth_input(addr, &mut cursor).await;
                }
                packets::id::ANIMATE => {
                    self.handle_animate(addr, &mut cursor).await;
                }
                packets::id::RESPAWN => {
                    self.handle_respawn(addr, &mut cursor).await;
                }
                other => {
                    debug!(
                        "Game packet 0x{other:02X} from {addr}: {} bytes",
                        sub_packet.len()
                    );
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 3: Network negotiation
    // -----------------------------------------------------------------------

    async fn handle_request_network_settings(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::AwaitingNetworkSettings => {}
            _ => {
                warn!("Unexpected RequestNetworkSettings from {addr}");
                return;
            }
        }

        let request = match packets::RequestNetworkSettings::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad RequestNetworkSettings from {addr}: {e}");
                return;
            }
        };

        info!(
            "RequestNetworkSettings from {addr}: protocol {}",
            request.protocol_version
        );

        if request.protocol_version != packets::PROTOCOL_VERSION {
            let status = if request.protocol_version < packets::PROTOCOL_VERSION {
                PlayStatusType::FailedClient
            } else {
                PlayStatusType::FailedServer
            };
            info!(
                "Protocol mismatch from {addr}: got {}, expected {} -> {:?}",
                request.protocol_version,
                packets::PROTOCOL_VERSION,
                status
            );
            self.send_packet(addr, packets::id::PLAY_STATUS, &PlayStatus { status })
                .await;
            return;
        }

        let settings = NetworkSettings::default();
        self.send_packet(addr, packets::id::NETWORK_SETTINGS, &settings)
            .await;

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.batch_config.compression_enabled = true;
            conn.batch_config.compression =
                CompressionAlgorithm::from_u16(settings.compression_algorithm)
                    .unwrap_or(CompressionAlgorithm::Zlib);
            conn.batch_config.compression_threshold = settings.compression_threshold as usize;
            conn.state = LoginState::AwaitingLogin;
        }

        info!(
            "Sent NetworkSettings to {addr}, compression enabled (online_mode={})",
            self.online_mode
        );
    }

    // -----------------------------------------------------------------------
    // Phase 4: Authentication
    // -----------------------------------------------------------------------

    async fn handle_login(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::AwaitingLogin => {}
            _ => {
                warn!("Unexpected LoginPacket from {addr}");
                return;
            }
        }

        let login = match packets::LoginPacket::proto_decode(buf) {
            Ok(l) => l,
            Err(e) => {
                warn!("Bad LoginPacket from {addr}: {e}");
                self.send_packet(
                    addr,
                    packets::id::DISCONNECT,
                    &Disconnect::with_message("Invalid login data"),
                )
                .await;
                return;
            }
        };

        let login_data = match jwt::extract_login_data(&login.chain_data) {
            Ok(data) => data,
            Err(e) => {
                warn!("JWT extraction failed for {addr}: {e}");
                self.send_packet(
                    addr,
                    packets::id::DISCONNECT,
                    &Disconnect::with_message("Authentication failed"),
                )
                .await;
                return;
            }
        };

        // Parse client data (skin, device info) from the client_data JWT
        let client_data = jwt::extract_client_data(&login.client_data_jwt).unwrap_or_else(|e| {
            warn!("Failed to parse client_data from {addr}: {e}, using defaults");
            jwt::ClientData::default()
        });

        info!(
            "Login from {addr}: {} (XUID: {}, UUID: {})",
            login_data.display_name, login_data.xuid, login_data.identity
        );

        // Check IP ban
        let ip_str = addr.ip().to_string();
        if let Some(ban) = self.permissions.banned_ips.get(&ip_str) {
            info!("Rejected banned IP {ip_str}: {}", ban.reason);
            self.send_packet(
                addr,
                packets::id::DISCONNECT,
                &Disconnect::with_message(format!("You are banned: {}", ban.reason)),
            )
            .await;
            return;
        }

        // Check player ban
        if let Some(ban) = self
            .permissions
            .banned_players
            .get(&login_data.display_name)
        {
            info!(
                "Rejected banned player {}: {}",
                login_data.display_name, ban.reason
            );
            self.send_packet(
                addr,
                packets::id::DISCONNECT,
                &Disconnect::with_message(format!("You are banned: {}", ban.reason)),
            )
            .await;
            return;
        }

        // Check whitelist
        if self.permissions.whitelist_enabled
            && !self
                .permissions
                .whitelist
                .contains(&login_data.display_name)
        {
            info!(
                "Rejected non-whitelisted player: {}",
                login_data.display_name
            );
            self.send_packet(
                addr,
                packets::id::DISCONNECT,
                &Disconnect::with_message("You are not whitelisted on this server."),
            )
            .await;
            return;
        }

        if self.online_mode {
            // Store client_data before encryption handshake
            if let Some(conn) = self.connections.get_mut(&addr) {
                conn.client_data = Some(client_data);
            }
            self.start_encryption_handshake(addr, login_data).await;
        } else {
            if let Some(conn) = self.connections.get_mut(&addr) {
                conn.login_data = Some(login_data);
                conn.client_data = Some(client_data);
                conn.state = LoginState::LoggedIn;
            }

            self.send_packet(
                addr,
                packets::id::PLAY_STATUS,
                &PlayStatus {
                    status: PlayStatusType::LoginSuccess,
                },
            )
            .await;

            info!("Sent PlayStatus(LoginSuccess) to {addr} (offline mode)");

            // Start resource pack exchange
            self.send_resource_packs_info(addr).await;
        }
    }

    async fn start_encryption_handshake(&mut self, addr: SocketAddr, login_data: jwt::LoginData) {
        let client_public = match parse_client_public_key(&login_data.identity_public_key) {
            Ok(pk) => pk,
            Err(e) => {
                warn!("Invalid client public key from {addr}: {e}");
                self.send_packet(
                    addr,
                    packets::id::DISCONNECT,
                    &Disconnect::with_message("Invalid public key"),
                )
                .await;
                return;
            }
        };

        let server_keypair = ServerKeyPair::generate();
        let shared_secret = server_keypair.shared_secret(&client_public);
        let salt: [u8; 16] = rand::random();
        let (aes_key, iv) = derive_key(&salt, &shared_secret);

        let jwt_string = match create_handshake_jwt(&server_keypair, &salt) {
            Ok(jwt) => jwt,
            Err(e) => {
                warn!("Failed to create handshake JWT for {addr}: {e}");
                self.send_packet(
                    addr,
                    packets::id::DISCONNECT,
                    &Disconnect::with_message("Encryption handshake failed"),
                )
                .await;
                return;
            }
        };

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.login_data = Some(login_data);
            conn.pending_encryption = Some(PendingEncryption { aes_key, iv });
            conn.state = LoginState::AwaitingHandshake;
        }

        self.send_packet(
            addr,
            packets::id::SERVER_TO_CLIENT_HANDSHAKE,
            &ServerToClientHandshake { jwt: jwt_string },
        )
        .await;

        info!("Sent ServerToClientHandshake to {addr}, awaiting client confirmation");
    }

    async fn handle_client_to_server_handshake(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::AwaitingHandshake => {}
            _ => {
                warn!("Unexpected ClientToServerHandshake from {addr}");
                return;
            }
        }

        if let Err(e) = ClientToServerHandshake::proto_decode(buf) {
            warn!("Bad ClientToServerHandshake from {addr}: {e}");
            return;
        }

        let activated = if let Some(conn) = self.connections.get_mut(&addr) {
            if let Some(pending) = conn.pending_encryption.take() {
                conn.encryption = Some(PacketEncryption::new(&pending.aes_key, &pending.iv));
                conn.state = LoginState::LoggedIn;
                true
            } else {
                warn!("No pending encryption for {addr}");
                false
            }
        } else {
            false
        };

        if !activated {
            return;
        }

        info!("Encryption activated for {addr}");

        self.send_packet(
            addr,
            packets::id::PLAY_STATUS,
            &PlayStatus {
                status: PlayStatusType::LoginSuccess,
            },
        )
        .await;

        info!("Sent PlayStatus(LoginSuccess) to {addr} (encrypted)");

        // Start resource pack exchange
        self.send_resource_packs_info(addr).await;
    }

    // -----------------------------------------------------------------------
    // Phase 5: Resource Packs
    // -----------------------------------------------------------------------

    async fn send_resource_packs_info(&mut self, addr: SocketAddr) {
        self.send_packet(
            addr,
            packets::id::RESOURCE_PACKS_INFO,
            &ResourcePacksInfo::default(),
        )
        .await;

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.state = LoginState::AwaitingResourcePackResponse;
        }

        info!("Sent ResourcePacksInfo to {addr} (empty packs)");
    }

    async fn handle_resource_pack_client_response(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        let current_state = match self.connections.get(&addr) {
            Some(c) => c.state,
            None => return,
        };

        let response = match ResourcePackClientResponse::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad ResourcePackClientResponse from {addr}: {e}");
                return;
            }
        };

        debug!(
            "ResourcePackClientResponse from {addr}: {:?} (state={:?})",
            response.status, current_state
        );

        match (current_state, response.status) {
            (
                LoginState::AwaitingResourcePackResponse,
                ResourcePackResponseStatus::HaveAllPacks,
            )
            | (LoginState::AwaitingResourcePackResponse, ResourcePackResponseStatus::Completed) => {
                // Client has all packs (or none needed) — send stack
                self.send_packet(
                    addr,
                    packets::id::RESOURCE_PACK_STACK,
                    &ResourcePackStack::default(),
                )
                .await;

                if let Some(conn) = self.connections.get_mut(&addr) {
                    conn.state = LoginState::AwaitingResourcePackComplete;
                }

                info!("Sent ResourcePackStack to {addr}");
            }
            (LoginState::AwaitingResourcePackComplete, ResourcePackResponseStatus::Completed) => {
                // Resource packs done — start game initialization
                info!("Resource pack exchange complete for {addr}");
                self.send_start_game(addr).await;
            }
            _ => {
                warn!(
                    "Unexpected ResourcePackClientResponse {:?} in state {:?} from {addr}",
                    response.status, current_state
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 6: World initialization
    // -----------------------------------------------------------------------

    async fn send_start_game(&mut self, addr: SocketAddr) {
        let (entity_unique_id, entity_runtime_id) = match self.connections.get(&addr) {
            Some(c) => (c.entity_unique_id, c.entity_runtime_id),
            None => return,
        };

        let config = &self.server_config;
        let gamemode = gamemode_from_str(&config.server.gamemode);
        let difficulty = difficulty_from_str(&config.server.difficulty);
        let generator = generator_from_str(&config.world.generator);

        let start_game = StartGame {
            entity_unique_id,
            entity_runtime_id,
            player_gamemode: gamemode,
            player_position: Vec3::new(0.5, 5.62, 0.5),
            rotation: Vec2::ZERO,
            seed: config.world.seed as u64,
            dimension: 0,
            generator,
            world_gamemode: gamemode,
            difficulty,
            spawn_position: BlockPos::new(0, 4, 0),
            level_id: "level".into(),
            world_name: config.world.name.clone(),
            game_version: "1.21.50".into(),
            item_table: self
                .item_registry
                .item_table_entries()
                .into_iter()
                .map(|e| mc_rs_proto::packets::start_game::ItemTableEntry {
                    string_id: e.string_id,
                    numeric_id: e.numeric_id,
                    is_component_based: e.is_component_based,
                })
                .collect(),
            ..StartGame::default()
        };

        self.send_packet(addr, packets::id::START_GAME, &start_game)
            .await;
        info!("Sent StartGame to {addr}");

        // Send creative content (items available in creative menu)
        let creative_items = mc_rs_proto::packets::creative_content::default_creative_items();
        let creative_content =
            mc_rs_proto::packets::creative_content::build_creative_content(&creative_items);
        self.send_packet(addr, packets::id::CREATIVE_CONTENT, &creative_content)
            .await;

        self.send_packet(
            addr,
            packets::id::BIOME_DEFINITION_LIST,
            &BiomeDefinitionList::canonical(),
        )
        .await;

        self.send_packet(
            addr,
            packets::id::AVAILABLE_ENTITY_IDENTIFIERS,
            &AvailableEntityIdentifiers::canonical(),
        )
        .await;

        self.send_packet(addr, packets::id::AVAILABLE_COMMANDS, &AvailableCommands)
            .await;

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.state = LoginState::Spawning;
        }

        info!("Sent world initialization packets to {addr}");
    }

    async fn handle_request_chunk_radius(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        let state = match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::Spawning || c.state == LoginState::InGame => c.state,
            _ => {
                debug!("RequestChunkRadius from {addr} in unexpected state");
                return;
            }
        };

        let request = match RequestChunkRadius::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad RequestChunkRadius from {addr}: {e}");
                return;
            }
        };

        // Clamp to a reasonable server max (8 chunks)
        let accepted_radius = request.chunk_radius.clamp(1, 8);

        self.send_packet(
            addr,
            packets::id::CHUNK_RADIUS_UPDATED,
            &ChunkRadiusUpdated {
                chunk_radius: accepted_radius,
            },
        )
        .await;

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.chunk_radius = accepted_radius;
        }

        if state == LoginState::Spawning {
            // Spawn flow: send initial chunks around spawn
            self.send_packet(
                addr,
                packets::id::NETWORK_CHUNK_PUBLISHER_UPDATE,
                &NetworkChunkPublisherUpdate {
                    position: BlockPos::new(0, 4, 0),
                    radius: (accepted_radius * 16) as u32,
                },
            )
            .await;

            self.send_spawn_chunks(addr, accepted_radius).await;

            // Tell the client it can spawn the player
            self.send_packet(
                addr,
                packets::id::PLAY_STATUS,
                &PlayStatus {
                    status: PlayStatusType::PlayerSpawn,
                },
            )
            .await;

            // Send initial inventory contents
            self.send_inventory(addr).await;

            info!(
                "Sent ChunkRadiusUpdated({accepted_radius}) + {} chunks + PlayStatus(PlayerSpawn) + inventory to {addr}",
                (accepted_radius * 2 + 1) * (accepted_radius * 2 + 1)
            );
        } else {
            // In-game render distance change: send new chunks around current position
            self.send_new_chunks(addr).await;
            info!("Updated chunk radius to {accepted_radius} for {addr}");
        }
    }

    async fn send_spawn_chunks(&mut self, addr: SocketAddr, radius: i32) {
        // Store chunk_radius on the connection
        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.chunk_radius = radius;
        }

        let mut count = 0u32;
        for cx in -radius..=radius {
            for cz in -radius..=radius {
                // Generate chunk if not already cached
                if !self.world_chunks.contains_key(&(cx, cz)) {
                    let column = generate_flat_chunk(cx, cz, &self.flat_world_blocks);
                    self.world_chunks.insert((cx, cz), column);
                }

                let column = self.world_chunks.get(&(cx, cz)).unwrap();
                let (sub_chunk_count, payload) = serialize_chunk_column(column);

                let level_chunk = LevelChunk {
                    chunk_x: cx,
                    chunk_z: cz,
                    dimension_id: 0,
                    sub_chunk_count,
                    cache_enabled: false,
                    payload: Bytes::from(payload),
                };

                self.send_packet(addr, packets::id::LEVEL_CHUNK, &level_chunk)
                    .await;
                count += 1;

                // Track sent chunk
                if let Some(conn) = self.connections.get_mut(&addr) {
                    conn.sent_chunks.insert((cx, cz));
                }
            }
        }
        debug!("Sent {count} LevelChunk packets to {addr}");
    }

    /// Convert a world coordinate (f32) to chunk coordinate.
    fn chunk_coord(v: f32) -> i32 {
        v.floor() as i32 >> 4
    }

    /// Send new chunks around the player's current position that haven't been sent yet.
    async fn send_new_chunks(&mut self, addr: SocketAddr) {
        let (center_x, center_z, radius) = match self.connections.get(&addr) {
            Some(c) => {
                let cx = Self::chunk_coord(c.position.x);
                let cz = Self::chunk_coord(c.position.z);
                (cx, cz, c.chunk_radius)
            }
            None => return,
        };

        // Send NetworkChunkPublisherUpdate with the player's block position
        let player_block_pos = match self.connections.get(&addr) {
            Some(c) => BlockPos::new(
                c.position.x.floor() as i32,
                c.position.y.floor() as i32,
                c.position.z.floor() as i32,
            ),
            None => return,
        };
        self.send_packet(
            addr,
            packets::id::NETWORK_CHUNK_PUBLISHER_UPDATE,
            &NetworkChunkPublisherUpdate {
                position: player_block_pos,
                radius: (radius * 16) as u32,
            },
        )
        .await;

        // Find new chunks to send
        let mut to_send = Vec::new();
        for cx in (center_x - radius)..=(center_x + radius) {
            for cz in (center_z - radius)..=(center_z + radius) {
                let key = (cx, cz);
                let already_sent = self
                    .connections
                    .get(&addr)
                    .map(|c| c.sent_chunks.contains(&key))
                    .unwrap_or(true);
                if !already_sent {
                    to_send.push(key);
                }
            }
        }

        if to_send.is_empty() {
            return;
        }

        // Generate and send new chunks
        for &(cx, cz) in &to_send {
            if !self.world_chunks.contains_key(&(cx, cz)) {
                let column = generate_flat_chunk(cx, cz, &self.flat_world_blocks);
                self.world_chunks.insert((cx, cz), column);
            }

            let column = self.world_chunks.get(&(cx, cz)).unwrap();
            let (sub_chunk_count, payload) = serialize_chunk_column(column);

            let level_chunk = LevelChunk {
                chunk_x: cx,
                chunk_z: cz,
                dimension_id: 0,
                sub_chunk_count,
                cache_enabled: false,
                payload: Bytes::from(payload),
            };

            self.send_packet(addr, packets::id::LEVEL_CHUNK, &level_chunk)
                .await;
        }

        // Mark as sent
        if let Some(conn) = self.connections.get_mut(&addr) {
            for key in &to_send {
                conn.sent_chunks.insert(*key);
            }
        }

        debug!("Sent {} new LevelChunk packets to {addr}", to_send.len());
    }

    async fn handle_set_local_player_as_initialized(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::Spawning => {}
            _ => {
                debug!("SetLocalPlayerAsInitialized from {addr} in unexpected state");
                return;
            }
        }

        let packet = match SetLocalPlayerAsInitialized::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                warn!("Bad SetLocalPlayerAsInitialized from {addr}: {e}");
                return;
            }
        };

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.state = LoginState::InGame;
        }

        // --- Multi-player: send PlayerList + AddPlayer ---
        // 1. Send PlayerList(Add) with all existing InGame players to the new player
        self.send_existing_players_to(addr).await;
        // 2. Broadcast PlayerList(Add) for the new player to everyone (including self for tab list)
        self.broadcast_new_player_list(addr).await;
        // 3. Send AddPlayer for each existing InGame player to the new player
        self.send_existing_add_players_to(addr).await;
        // 4. Broadcast AddPlayer for the new player to all existing InGame players
        self.broadcast_add_player(addr).await;
        // 5. Send AddActor for all existing mobs to the new player
        self.send_existing_mobs_to(addr).await;

        // 6. Send initial health attribute so the client HUD shows correctly
        let rid = self
            .connections
            .get(&addr)
            .map(|c| c.entity_runtime_id)
            .unwrap_or(0);
        self.send_packet(
            addr,
            packets::id::UPDATE_ATTRIBUTES,
            &UpdateAttributes::health(rid, 20.0, 0),
        )
        .await;

        let name = self
            .connections
            .get(&addr)
            .and_then(|c| c.login_data.as_ref())
            .map(|d| d.display_name.clone())
            .unwrap_or_default();

        // 7. Broadcast join message
        let join_msg = Text::system(format!("{name} joined the game"));
        self.broadcast_packet(packets::id::TEXT, &join_msg).await;

        info!(
            "Player {name} is now in-game ({addr}, runtime_id={})",
            packet.entity_runtime_id
        );
    }

    // -----------------------------------------------------------------------
    // Phase 1.1: Movement
    // -----------------------------------------------------------------------

    /// Maximum horizontal distance (blocks) a player can move per tick.
    /// Sprint = ~0.28 b/t; 1.0 gives generous margin for latency.
    const MAX_MOVE_DISTANCE_PER_TICK: f32 = 1.0;

    /// Minimum allowed Y position (world bottom).
    const MIN_Y_POSITION: f32 = -64.0;

    async fn handle_player_auth_input(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame && !c.is_dead => {}
            _ => return,
        }

        let input = match PlayerAuthInput::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                warn!("Bad PlayerAuthInput from {addr}: {e}");
                return;
            }
        };

        let (prev_position, entity_runtime_id, gamemode) = match self.connections.get(&addr) {
            Some(c) => (c.position, c.entity_runtime_id, c.gamemode),
            None => return,
        };

        // --- Validation ---
        let mut needs_correction = false;

        // 1. Reject NaN/Infinity positions
        if !input.position.x.is_finite()
            || !input.position.y.is_finite()
            || !input.position.z.is_finite()
        {
            warn!("Invalid position (NaN/Inf) from {addr}");
            needs_correction = true;
        }

        // 2. Horizontal speed check
        if !needs_correction {
            let dx = input.position.x - prev_position.x;
            let dz = input.position.z - prev_position.z;
            let horizontal_distance = (dx * dx + dz * dz).sqrt();

            if horizontal_distance > Self::MAX_MOVE_DISTANCE_PER_TICK {
                debug!("Movement too fast from {addr}: {horizontal_distance:.2} blocks/tick");
                needs_correction = true;
            }
        }

        // 3. Y position check (void falling)
        if !needs_correction && input.position.y < Self::MIN_Y_POSITION {
            debug!(
                "Player {addr} below world: {:.2} < {}",
                input.position.y,
                Self::MIN_Y_POSITION
            );
            needs_correction = true;
        }

        // 4. Vertical speed check (terminal velocity)
        if !needs_correction {
            let dy = (input.position.y - prev_position.y).abs();
            if dy > MAX_FALL_PER_TICK {
                debug!("Vertical speed too fast from {addr}: {dy:.2} blocks/tick");
                needs_correction = true;
            }
        }

        // 5. No-clip detection (survival/adventure only)
        // Check that the player's AABB does not overlap any solid block.
        if !needs_correction && gamemode != 1 && gamemode != 3 {
            let aabb =
                PlayerAabb::from_eye_position(input.position.x, input.position.y, input.position.z);
            for (bx, by, bz) in aabb.intersecting_blocks() {
                if let Some(hash) = self.get_block(bx, by, bz) {
                    if self.block_registry.is_solid(hash) {
                        debug!("No-clip detected at ({bx},{by},{bz}) from {addr}");
                        needs_correction = true;
                        break;
                    }
                }
            }
        }

        if needs_correction {
            let conn = match self.connections.get(&addr) {
                Some(c) => c,
                None => return,
            };
            let correction = MovePlayer::reset(
                entity_runtime_id,
                conn.position,
                conn.pitch,
                conn.yaw,
                conn.head_yaw,
                conn.on_ground,
                input.tick,
            );
            self.send_packet(addr, packets::id::MOVE_PLAYER, &correction)
                .await;
            return;
        }

        // --- Accept the movement ---

        // On-ground detection: check block below player's feet.
        // In Bedrock, position.y is the eye position (1.62 above feet).
        const PLAYER_EYE_HEIGHT: f32 = 1.62;
        let feet_y = input.position.y - PLAYER_EYE_HEIGHT;
        let check_y = (feet_y - 0.01).floor() as i32;
        let check_x = input.position.x.floor() as i32;
        let check_z = input.position.z.floor() as i32;
        let on_ground = self
            .get_block(check_x, check_y, check_z)
            .map(|hash| self.block_registry.is_solid(hash))
            .unwrap_or(true); // Default true for unloaded chunks

        // Anti-fly: track consecutive airborne ticks (survival only)
        let anti_fly_correction = if gamemode == 0 && !on_ground {
            let ticks = self
                .connections
                .get(&addr)
                .map(|c| c.airborne_ticks)
                .unwrap_or(0);
            let dy = input.position.y - prev_position.y;
            // If airborne too long AND not falling → fly hack
            ticks + 1 > MAX_AIRBORNE_TICKS && dy >= 0.0
        } else {
            false
        };

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.position = input.position;
            conn.pitch = input.pitch;
            conn.yaw = input.yaw;
            conn.head_yaw = input.head_yaw;
            conn.client_tick = input.tick;
            conn.on_ground = on_ground;
            conn.last_position_delta_y = input.position_delta.y;
            conn.is_sprinting =
                input.has_flag(mc_rs_proto::packets::player_auth_input::input_flags::SPRINTING);
            if on_ground {
                conn.airborne_ticks = 0;
            } else {
                conn.airborne_ticks = conn.airborne_ticks.saturating_add(1);
            }

            // Sync position to ECS mirror entity
            let uid = conn.entity_unique_id;
            self.game_world.update_player_position(
                uid,
                input.position.x,
                input.position.y,
                input.position.z,
            );
        }

        if anti_fly_correction {
            debug!("Anti-fly: {addr} airborne too long without falling (gamemode=survival)");
            let conn = match self.connections.get(&addr) {
                Some(c) => c,
                None => return,
            };
            let correction = MovePlayer::reset(
                entity_runtime_id,
                conn.position,
                conn.pitch,
                conn.yaw,
                conn.head_yaw,
                conn.on_ground,
                input.tick,
            );
            self.send_packet(addr, packets::id::MOVE_PLAYER, &correction)
                .await;
            return;
        }

        // --- Broadcast position to other players ---
        let move_pkt = MovePlayer::normal(
            entity_runtime_id,
            input.position,
            input.pitch,
            input.yaw,
            input.head_yaw,
            on_ground,
            input.tick,
        );
        self.broadcast_packet_except(addr, packets::id::MOVE_PLAYER, &move_pkt)
            .await;

        // --- Dynamic chunk loading ---
        let prev_chunk = (
            Self::chunk_coord(prev_position.x),
            Self::chunk_coord(prev_position.z),
        );
        let curr_chunk = (
            Self::chunk_coord(input.position.x),
            Self::chunk_coord(input.position.z),
        );

        if prev_chunk != curr_chunk {
            self.send_new_chunks(addr).await;
            self.cleanup_sent_chunks(addr);
        }
    }

    /// Remove chunks from `sent_chunks` that are outside the player's view radius.
    /// The client handles visual unloading via `NetworkChunkPublisherUpdate.radius`,
    /// this just prevents the tracking `HashSet` from growing indefinitely.
    fn cleanup_sent_chunks(&mut self, addr: SocketAddr) {
        let (center_x, center_z, radius) = match self.connections.get(&addr) {
            Some(c) => (
                Self::chunk_coord(c.position.x),
                Self::chunk_coord(c.position.z),
                c.chunk_radius,
            ),
            None => return,
        };

        // Keep a margin of 2 chunks beyond the render radius
        let unload_radius = radius + 2;
        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.sent_chunks.retain(|&(cx, cz)| {
                (cx - center_x).abs() <= unload_radius && (cz - center_z).abs() <= unload_radius
            });
        }
    }

    // -----------------------------------------------------------------------
    // Phase 1.4: Chat & Commands
    // -----------------------------------------------------------------------

    async fn handle_text(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame => {}
            _ => return,
        }

        let text = match Text::proto_decode(buf) {
            Ok(t) => t,
            Err(e) => {
                warn!("Bad Text packet from {addr}: {e}");
                return;
            }
        };

        let sender_name = self
            .connections
            .get(&addr)
            .and_then(|c| c.login_data.as_ref())
            .map(|d| d.display_name.as_str())
            .unwrap_or("unknown");

        info!("<{sender_name}> {}", text.message);

        let response = Text::raw(format!("<{sender_name}> {}", text.message));
        self.send_packet(addr, packets::id::TEXT, &response).await;
    }

    async fn handle_command_request(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame => {}
            _ => return,
        }

        let request = match CommandRequest::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad CommandRequest from {addr}: {e}");
                return;
            }
        };

        let sender_name = self
            .connections
            .get(&addr)
            .and_then(|c| c.login_data.as_ref())
            .map(|d| d.display_name.clone())
            .unwrap_or_else(|| "unknown".to_string());

        info!("{sender_name} issued command: {}", request.command);

        // Strip the leading '/'
        let command_str = request
            .command
            .strip_prefix('/')
            .unwrap_or(&request.command);
        let mut parts = command_str.split_whitespace();
        let cmd_name = parts.next().unwrap_or("");
        let raw_args: Vec<String> = parts.map(String::from).collect();

        // Permission check for operator-only commands
        let needs_op = matches!(
            cmd_name,
            "gamemode"
                | "tp"
                | "give"
                | "kill"
                | "kick"
                | "op"
                | "deop"
                | "ban"
                | "ban-ip"
                | "unban"
                | "unban-ip"
                | "whitelist"
                | "summon"
                | "stop"
        );
        if needs_op && !self.permissions.ops.contains(&sender_name) {
            let result = CommandResult::err("You do not have permission to use this command");
            let output = CommandOutput::failure(request.origin, result.messages.join("\n"));
            self.send_packet(addr, packets::id::COMMAND_OUTPUT, &output)
                .await;
            for msg in &result.messages {
                self.send_packet(addr, packets::id::TEXT, &Text::raw(msg))
                    .await;
            }
            return;
        }

        // Try server commands first (need &mut self access)
        let server_result = match cmd_name {
            "gamemode" => Some(self.cmd_gamemode(addr, &sender_name, &raw_args).await),
            "tp" => Some(self.cmd_tp(addr, &sender_name, &raw_args).await),
            "give" => Some(self.cmd_give(addr, &raw_args).await),
            "kill" => Some(self.cmd_kill(addr, &sender_name, &raw_args).await),
            "kick" => Some(self.cmd_kick(addr, &raw_args).await),
            "op" => Some(self.cmd_op(addr, &raw_args).await),
            "deop" => Some(self.cmd_deop(addr, &raw_args).await),
            "ban" => Some(self.cmd_ban(addr, &raw_args).await),
            "ban-ip" => Some(self.cmd_ban_ip(addr, &raw_args).await),
            "unban" => Some(self.cmd_unban(&raw_args)),
            "unban-ip" => Some(self.cmd_unban_ip(&raw_args)),
            "whitelist" => Some(self.cmd_whitelist(&raw_args)),
            "summon" => Some(self.cmd_summon(addr, &raw_args)),
            "effect" => Some(self.cmd_effect(addr, &sender_name, &raw_args).await),
            _ => None,
        };

        let result = if let Some(r) = server_result {
            r
        } else {
            // Fall through to registry (help, list, say, stop)
            let args = match cmd_name {
                "help" => self
                    .command_registry
                    .get_commands()
                    .values()
                    .map(|e| format!("{}:{}", e.name, e.description))
                    .collect(),
                "list" => self
                    .connections
                    .values()
                    .filter(|c| c.state == LoginState::InGame)
                    .filter_map(|c| c.login_data.as_ref())
                    .map(|d| d.display_name.clone())
                    .collect(),
                _ => raw_args,
            };
            let ctx = mc_rs_command::CommandContext {
                sender_name: sender_name.clone(),
                args,
            };
            self.command_registry.execute(cmd_name, &ctx)
        };

        // Send CommandOutput
        let output = if result.success {
            CommandOutput::success(request.origin, result.messages.join("\n"))
        } else {
            CommandOutput::failure(request.origin, result.messages.join("\n"))
        };
        self.send_packet(addr, packets::id::COMMAND_OUTPUT, &output)
            .await;

        // Send result messages as chat text to the sender
        for msg in &result.messages {
            self.send_packet(addr, packets::id::TEXT, &Text::raw(msg))
                .await;
        }

        // Broadcast if requested
        if let Some(broadcast_msg) = &result.broadcast {
            let text = Text::raw(broadcast_msg);
            self.broadcast_packet(packets::id::TEXT, &text).await;
        }

        // Shutdown if requested
        if result.should_stop {
            info!("Server stop requested by {sender_name}");
            let _ = self.shutdown_tx.send(true);
        }
    }

    // -----------------------------------------------------------------------
    // Server commands (need &mut self for connections/state access)
    // -----------------------------------------------------------------------

    /// Find a player's SocketAddr by display name.
    fn find_player_addr(&self, name: &str) -> Option<SocketAddr> {
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
    fn online_player_infos(&self) -> Vec<PlayerInfo> {
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
    fn resolve_target(&self, target: &str, addr: SocketAddr) -> Result<Vec<String>, String> {
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

    /// /gamemode <mode> [player]
    async fn cmd_gamemode(
        &mut self,
        sender_addr: SocketAddr,
        sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /gamemode <mode> [player]");
        }

        let gamemode = match parse_gamemode(&args[0]) {
            Some(gm) => gm,
            None => return CommandResult::err(format!("Unknown gamemode: {}", args[0])),
        };

        let targets = if args.len() >= 2 {
            match self.resolve_target(&args[1], sender_addr) {
                Ok(t) => t,
                Err(e) => return CommandResult::err(e),
            }
        } else {
            vec![sender_name.to_string()]
        };

        let mode_name = gamemode_name(gamemode);
        let mut messages = Vec::new();

        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            // Update server state
            if let Some(conn) = self.connections.get_mut(&target_addr) {
                conn.gamemode = gamemode;
            }

            // Send SetPlayerGameType to the target
            self.send_packet(
                target_addr,
                packets::id::SET_PLAYER_GAME_TYPE,
                &SetPlayerGameType { gamemode },
            )
            .await;

            // Send UpdateAbilities to the target
            let (entity_unique_id, perm, cmd_perm) = {
                let conn = &self.connections[&target_addr];
                let is_op = self.permissions.ops.contains(target_name.as_str());
                (
                    conn.entity_unique_id,
                    if is_op { 2u8 } else { 1u8 },
                    if is_op { 1u8 } else { 0u8 },
                )
            };
            self.send_packet(
                target_addr,
                packets::id::UPDATE_ABILITIES,
                &UpdateAbilities {
                    command_permission_level: cmd_perm,
                    permission_level: perm,
                    entity_unique_id,
                    gamemode,
                },
            )
            .await;

            messages.push(format!("Set {target_name}'s game mode to {mode_name}"));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    /// /tp — three forms:
    /// /tp <x> <y> <z>
    /// /tp <target> <x> <y> <z>
    /// /tp <target> <destination>
    async fn cmd_tp(
        &mut self,
        sender_addr: SocketAddr,
        sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        match args.len() {
            3 => {
                // /tp <x> <y> <z> (self)
                let (x, y, z) = match parse_coords(&args[0], &args[1], &args[2]) {
                    Some(c) => c,
                    None => return CommandResult::err("Invalid coordinates"),
                };
                self.teleport_player(sender_addr, sender_name, x, y, z)
                    .await
            }
            2 => {
                // /tp <target> <destination_player>
                let targets = match self.resolve_target(&args[0], sender_addr) {
                    Ok(t) => t,
                    Err(e) => return CommandResult::err(e),
                };
                let dest_names = match self.resolve_target(&args[1], sender_addr) {
                    Ok(t) => t,
                    Err(e) => return CommandResult::err(e),
                };
                if dest_names.len() != 1 {
                    return CommandResult::err("Destination must be a single player");
                }
                let dest_pos = match self.find_player_addr(&dest_names[0]) {
                    Some(a) => self
                        .connections
                        .get(&a)
                        .map(|c| c.position)
                        .unwrap_or(Vec3::ZERO),
                    None => {
                        return CommandResult::err(format!("Player not found: {}", dest_names[0]))
                    }
                };

                let mut messages = Vec::new();
                for target_name in &targets {
                    let target_addr = match self.find_player_addr(target_name) {
                        Some(a) => a,
                        None => {
                            messages.push(format!("Player not found: {target_name}"));
                            continue;
                        }
                    };
                    self.teleport_player(
                        target_addr,
                        target_name,
                        dest_pos.x,
                        dest_pos.y,
                        dest_pos.z,
                    )
                    .await;
                    messages.push(format!("Teleported {target_name} to {}", dest_names[0]));
                }
                CommandResult {
                    success: true,
                    messages,
                    broadcast: None,
                    should_stop: false,
                }
            }
            4 => {
                // /tp <target> <x> <y> <z>
                let targets = match self.resolve_target(&args[0], sender_addr) {
                    Ok(t) => t,
                    Err(e) => return CommandResult::err(e),
                };
                let (x, y, z) = match parse_coords(&args[1], &args[2], &args[3]) {
                    Some(c) => c,
                    None => return CommandResult::err("Invalid coordinates"),
                };
                let mut messages = Vec::new();
                for target_name in &targets {
                    let target_addr = match self.find_player_addr(target_name) {
                        Some(a) => a,
                        None => {
                            messages.push(format!("Player not found: {target_name}"));
                            continue;
                        }
                    };
                    self.teleport_player(target_addr, target_name, x, y, z)
                        .await;
                    messages.push(format!(
                        "Teleported {target_name} to {x:.1}, {y:.1}, {z:.1}"
                    ));
                }
                CommandResult {
                    success: true,
                    messages,
                    broadcast: None,
                    should_stop: false,
                }
            }
            _ => CommandResult::err(
                "Usage: /tp <x> <y> <z> OR /tp <target> <x> <y> <z> OR /tp <target> <destination>",
            ),
        }
    }

    /// Perform the actual teleport for a single player.
    async fn teleport_player(
        &mut self,
        target_addr: SocketAddr,
        target_name: &str,
        x: f32,
        y: f32,
        z: f32,
    ) -> CommandResult {
        let (runtime_id, tick, uid) = match self.connections.get_mut(&target_addr) {
            Some(conn) => {
                conn.position = Vec3::new(x, y, z);
                conn.on_ground = false;
                (
                    conn.entity_runtime_id,
                    conn.client_tick,
                    conn.entity_unique_id,
                )
            }
            None => return CommandResult::err(format!("Player not found: {target_name}")),
        };

        // Sync position to ECS mirror entity
        self.game_world.update_player_position(uid, x, y, z);

        let pkt = MovePlayer {
            runtime_entity_id: runtime_id,
            position: Vec3::new(x, y, z),
            pitch: 0.0,
            yaw: 0.0,
            head_yaw: 0.0,
            mode: MoveMode::Teleport,
            on_ground: false,
            ridden_entity_runtime_id: 0,
            teleport_cause: Some(0),
            teleport_entity_type: Some(0),
            tick,
        };
        self.send_packet(target_addr, packets::id::MOVE_PLAYER, &pkt)
            .await;

        // Broadcast to other players so they see the teleport
        self.broadcast_packet_except(target_addr, packets::id::MOVE_PLAYER, &pkt)
            .await;

        CommandResult::ok(format!(
            "Teleported {target_name} to {x:.1}, {y:.1}, {z:.1}"
        ))
    }

    /// /give <player> <item> [amount] [metadata]
    async fn cmd_give(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::err("Usage: /give <player> <item> [amount] [metadata]");
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        // Normalize item name: add "minecraft:" prefix if missing
        let item_name = if args[1].contains(':') {
            args[1].clone()
        } else {
            format!("minecraft:{}", args[1])
        };

        let item_info = match self.item_registry.get_by_name(&item_name) {
            Some(info) => info.clone(),
            None => return CommandResult::err(format!("Unknown item: {}", args[1])),
        };

        let amount = if args.len() >= 3 {
            match args[2].parse::<u16>() {
                Ok(a) if (1..=255).contains(&a) => a,
                _ => return CommandResult::err("Amount must be 1-255"),
            }
        } else {
            1
        };

        let metadata = if args.len() >= 4 {
            match args[3].parse::<u16>() {
                Ok(m) => m,
                _ => return CommandResult::err("Invalid metadata value"),
            }
        } else {
            0
        };

        let mut messages = Vec::new();

        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            // Find first empty slot in main inventory
            let slot = match self.connections.get(&target_addr) {
                Some(c) => c.inventory.main.iter().position(|s| s.is_empty()),
                None => continue,
            };

            let slot = match slot {
                Some(s) => s as u8,
                None => {
                    messages.push(format!("{target_name}'s inventory is full"));
                    continue;
                }
            };

            // Create the item
            let stack_id = match self.connections.get(&target_addr) {
                Some(c) => c.inventory.next_stack_network_id(),
                None => continue,
            };

            let item = mc_rs_proto::item_stack::ItemStack::new_with_meta(
                item_info.numeric_id as i32,
                amount,
                metadata,
                stack_id,
            );

            // Set in server inventory
            if let Some(conn) = self.connections.get_mut(&target_addr) {
                conn.inventory.set_slot(0, slot, item.clone());
            }

            // Send InventorySlot to client
            self.send_packet(
                target_addr,
                packets::id::INVENTORY_SLOT,
                &InventorySlot {
                    window_id: 0,
                    slot: slot as u32,
                    item,
                },
            )
            .await;

            messages.push(format!("Gave {amount} {} to {target_name}", item_info.name));
        }

        CommandResult {
            success: !messages.is_empty(),
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    /// /kill [player] (default = self)
    async fn cmd_kill(
        &mut self,
        sender_addr: SocketAddr,
        sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        let targets = if args.is_empty() {
            vec![sender_name.to_string()]
        } else {
            match self.resolve_target(&args[0], sender_addr) {
                Ok(t) => t,
                Err(e) => return CommandResult::err(e),
            }
        };

        let mut messages = Vec::new();

        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            let runtime_id = match self.connections.get(&target_addr) {
                Some(conn) => conn.entity_runtime_id,
                None => continue,
            };

            // Set health to 0 and mark as dead
            if let Some(conn) = self.connections.get_mut(&target_addr) {
                conn.health = 0.0;
                conn.is_dead = true;
            }

            // Send health=0 to the victim
            let tick = self
                .connections
                .get(&target_addr)
                .map(|c| c.client_tick)
                .unwrap_or(0);
            self.send_packet(
                target_addr,
                packets::id::UPDATE_ATTRIBUTES,
                &UpdateAttributes::health(runtime_id, 0.0, tick),
            )
            .await;

            // Broadcast death event
            self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::death(runtime_id))
                .await;

            // Send Respawn(searching) to trigger death screen
            let spawn_pos = Vec3::new(0.5, 5.62, 0.5);
            self.send_packet(
                target_addr,
                packets::id::RESPAWN,
                &Respawn {
                    position: spawn_pos,
                    state: 0, // searching — shows death screen
                    runtime_entity_id: runtime_id,
                },
            )
            .await;

            messages.push(format!("Killed {target_name}"));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    /// /kick <player> [reason]
    async fn cmd_kick(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /kick <player> [reason]");
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let reason = if args.len() >= 2 {
            args[1..].join(" ")
        } else {
            "Kicked by an operator".to_string()
        };

        let mut messages = Vec::new();

        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            // Send Disconnect packet — cleanup happens via handle_session_disconnected
            self.send_packet(
                target_addr,
                packets::id::DISCONNECT,
                &Disconnect::with_message(&reason),
            )
            .await;

            messages.push(format!("Kicked {target_name}: {reason}"));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    /// /op <player>
    async fn cmd_op(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /op <player>");
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let mut messages = Vec::new();

        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            self.permissions.ops.insert(target_name.clone());
            self.permissions.save_ops();

            let (entity_unique_id, gamemode) = {
                let conn = &self.connections[&target_addr];
                (conn.entity_unique_id, conn.gamemode)
            };
            self.send_packet(
                target_addr,
                packets::id::UPDATE_ABILITIES,
                &UpdateAbilities {
                    command_permission_level: 1,
                    permission_level: 2,
                    entity_unique_id,
                    gamemode,
                },
            )
            .await;

            messages.push(format!("Opped {target_name}"));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    /// /deop <player>
    async fn cmd_deop(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /deop <player>");
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let mut messages = Vec::new();

        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            self.permissions.ops.remove(target_name.as_str());
            self.permissions.save_ops();

            let (entity_unique_id, gamemode) = {
                let conn = &self.connections[&target_addr];
                (conn.entity_unique_id, conn.gamemode)
            };
            self.send_packet(
                target_addr,
                packets::id::UPDATE_ABILITIES,
                &UpdateAbilities {
                    command_permission_level: 0,
                    permission_level: 1,
                    entity_unique_id,
                    gamemode,
                },
            )
            .await;

            messages.push(format!("De-opped {target_name}"));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    async fn cmd_ban(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /ban <player> [reason]");
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let reason = if args.len() > 1 {
            args[1..].join(" ")
        } else {
            "Banned by an operator".to_string()
        };

        let mut messages = Vec::new();

        for target_name in &targets {
            self.permissions.banned_players.insert(
                target_name.clone(),
                BanEntry {
                    reason: reason.clone(),
                },
            );

            // Kick the player if online
            if let Some(target_addr) = self.find_player_addr(target_name) {
                self.send_packet(
                    target_addr,
                    packets::id::DISCONNECT,
                    &Disconnect::with_message(format!("You are banned: {reason}")),
                )
                .await;
            }

            messages.push(format!("Banned {target_name}: {reason}"));
        }

        self.permissions.save_banned_players();

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    async fn cmd_ban_ip(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /ban-ip <ip> [reason]");
        }

        let ip = &args[0];
        let reason = if args.len() > 1 {
            args[1..].join(" ")
        } else {
            "Banned by an operator".to_string()
        };

        self.permissions.banned_ips.insert(
            ip.clone(),
            BanEntry {
                reason: reason.clone(),
            },
        );
        self.permissions.save_banned_ips();

        // Kick all players connected from this IP
        let addrs_to_kick: Vec<SocketAddr> = self
            .connections
            .iter()
            .filter(|(a, c)| a.ip().to_string() == *ip && c.state == LoginState::InGame)
            .map(|(&a, _)| a)
            .collect();

        for kick_addr in &addrs_to_kick {
            self.send_packet(
                *kick_addr,
                packets::id::DISCONNECT,
                &Disconnect::with_message(format!("You are banned: {reason}")),
            )
            .await;
        }

        let _ = sender_addr; // used for consistency with other commands
        CommandResult::ok(format!(
            "Banned IP {ip}: {reason} ({} player(s) kicked)",
            addrs_to_kick.len()
        ))
    }

    fn cmd_unban(&mut self, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /unban <player>");
        }

        let name = &args[0];
        if self.permissions.banned_players.remove(name).is_some() {
            self.permissions.save_banned_players();
            CommandResult::ok(format!("Unbanned {name}"))
        } else {
            CommandResult::err(format!("{name} is not banned"))
        }
    }

    fn cmd_unban_ip(&mut self, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /unban-ip <ip>");
        }

        let ip = &args[0];
        if self.permissions.banned_ips.remove(ip.as_str()).is_some() {
            self.permissions.save_banned_ips();
            CommandResult::ok(format!("Unbanned IP {ip}"))
        } else {
            CommandResult::err(format!("IP {ip} is not banned"))
        }
    }

    fn cmd_whitelist(&mut self, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /whitelist <add|remove|list|on|off> [player]");
        }

        match args[0].as_str() {
            "add" => {
                let name = match args.get(1) {
                    Some(n) => n,
                    None => return CommandResult::err("Usage: /whitelist add <player>"),
                };
                self.permissions.whitelist.insert(name.clone());
                self.permissions.save_whitelist();
                CommandResult::ok(format!("Added {name} to the whitelist"))
            }
            "remove" => {
                let name = match args.get(1) {
                    Some(n) => n,
                    None => return CommandResult::err("Usage: /whitelist remove <player>"),
                };
                if self.permissions.whitelist.remove(name.as_str()) {
                    self.permissions.save_whitelist();
                    CommandResult::ok(format!("Removed {name} from the whitelist"))
                } else {
                    CommandResult::err(format!("{name} is not on the whitelist"))
                }
            }
            "list" => {
                let mut names: Vec<&String> = self.permissions.whitelist.iter().collect();
                names.sort();
                if names.is_empty() {
                    CommandResult::ok("Whitelist is empty")
                } else {
                    CommandResult::ok(format!(
                        "Whitelisted players ({}): {}",
                        names.len(),
                        names
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                }
            }
            "on" => {
                self.permissions.whitelist_enabled = true;
                CommandResult::ok("Whitelist enabled")
            }
            "off" => {
                self.permissions.whitelist_enabled = false;
                CommandResult::ok("Whitelist disabled")
            }
            other => CommandResult::err(format!(
                "Unknown whitelist action: {other}. Use add, remove, list, on, or off."
            )),
        }
    }

    /// /summon <entity_type> [x y z]
    fn cmd_summon(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            let known: Vec<&str> = self
                .game_world
                .mob_registry
                .all()
                .iter()
                .map(|m| m.type_id)
                .collect();
            return CommandResult::err(format!(
                "Usage: /summon <type> [x y z]. Available: {}",
                known.join(", ")
            ));
        }

        let entity_type = &args[0];
        let full_type = if entity_type.contains(':') {
            entity_type.clone()
        } else {
            format!("minecraft:{entity_type}")
        };

        if self.game_world.mob_registry.get(&full_type).is_none() {
            let known: Vec<&str> = self
                .game_world
                .mob_registry
                .all()
                .iter()
                .map(|m| m.type_id)
                .collect();
            return CommandResult::err(format!(
                "Unknown entity type: {full_type}. Available: {}",
                known.join(", ")
            ));
        }

        let (x, y, z) = if args.len() >= 4 {
            match parse_coords(&args[1], &args[2], &args[3]) {
                Some(c) => c,
                None => return CommandResult::err("Invalid coordinates"),
            }
        } else {
            match self.connections.get(&sender_addr) {
                Some(c) => (c.position.x, c.position.y, c.position.z),
                None => return CommandResult::err("Sender not found"),
            }
        };

        match self.game_world.spawn_mob(&full_type, x, y, z) {
            Some(_) => {
                CommandResult::ok(format!("Summoned {full_type} at ({x:.1}, {y:.1}, {z:.1})"))
            }
            None => CommandResult::err(format!("Failed to summon {full_type}")),
        }
    }

    /// /effect <target> <effect> [amplifier] [duration_seconds]
    /// /effect <target> clear
    async fn cmd_effect(
        &mut self,
        sender_addr: SocketAddr,
        _sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::err(
                "Usage: /effect <target> <effect> [amplifier] [duration] or /effect <target> clear",
            );
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        // /effect <target> clear
        if args[1] == "clear" {
            let mut messages = Vec::new();
            for target_name in &targets {
                let target_addr = match self.find_player_addr(target_name) {
                    Some(a) => a,
                    None => {
                        messages.push(format!("Player not found: {target_name}"));
                        continue;
                    }
                };
                self.clear_effects(target_addr).await;
                messages.push(format!("Cleared effects for {target_name}"));
            }
            return CommandResult {
                success: true,
                messages,
                broadcast: None,
                should_stop: false,
            };
        }

        // Parse effect name
        let effect_id = match effect_name_to_id(&args[1]) {
            Some(id) => id,
            None => {
                return CommandResult::err(format!(
                "Unknown effect: {}. Available: speed, slowness, strength, weakness, resistance, \
                     haste, mining_fatigue, jump_boost, nausea, regeneration, fire_resistance, \
                     water_breathing, invisibility, blindness, night_vision, hunger, poison, \
                     wither, absorption",
                args[1]
            ))
            }
        };

        let amplifier = if args.len() >= 3 {
            match args[2].parse::<i32>() {
                Ok(a) => a.clamp(0, 255),
                Err(_) => return CommandResult::err("Invalid amplifier (must be 0-255)"),
            }
        } else {
            0
        };

        let duration_secs = if args.len() >= 4 {
            match args[3].parse::<i32>() {
                Ok(d) if d > 0 => d,
                _ => return CommandResult::err("Invalid duration (must be > 0)"),
            }
        } else {
            30
        };
        let duration_ticks = duration_secs * 20;

        let mut messages = Vec::new();
        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            self.apply_effect(target_addr, effect_id, amplifier, duration_ticks)
                .await;
            messages.push(format!(
                "Applied {} {} to {target_name} for {duration_secs}s",
                args[1],
                amplifier + 1
            ));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    // -----------------------------------------------------------------------
    // PvP combat
    // -----------------------------------------------------------------------

    /// Handle Animate packet (arm swing broadcast).
    async fn handle_animate(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame && !c.is_dead => {}
            _ => return,
        }

        let pkt = match Animate::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                warn!("Bad Animate from {addr}: {e}");
                return;
            }
        };

        if pkt.action_type != mc_rs_proto::packets::animate::ACTION_SWING_ARM {
            return;
        }

        let runtime_id = match self.connections.get(&addr) {
            Some(c) => c.entity_runtime_id,
            None => return,
        };

        let broadcast = Animate {
            action_type: mc_rs_proto::packets::animate::ACTION_SWING_ARM,
            entity_runtime_id: runtime_id,
        };
        self.broadcast_packet_except(addr, packets::id::ANIMATE, &broadcast)
            .await;
    }

    /// Find a player's address by their entity runtime ID.
    fn find_addr_by_runtime_id(&self, runtime_id: u64) -> Option<SocketAddr> {
        self.connections.iter().find_map(|(&a, conn)| {
            if conn.entity_runtime_id == runtime_id && conn.state == LoginState::InGame {
                Some(a)
            } else {
                None
            }
        })
    }

    /// Handle a player attacking another entity (PvP or PvE).
    async fn handle_attack(&mut self, attacker_addr: SocketAddr, victim_runtime_id: u64) {
        let (
            attacker_gamemode,
            attacker_pos,
            held_item_rid,
            is_sprinting,
            on_ground,
            delta_y,
            weapon_nbt,
        ) = match self.connections.get(&attacker_addr) {
            Some(c) => {
                let item = c.inventory.held_item();
                (
                    c.gamemode,
                    c.position,
                    item.runtime_id,
                    c.is_sprinting,
                    c.on_ground,
                    c.last_position_delta_y,
                    item.nbt_data.clone(),
                )
            }
            None => return,
        };

        // Creative/Spectator cannot attack
        if attacker_gamemode == 1 || attacker_gamemode == 3 {
            return;
        }

        let base_damage = base_attack_damage(&self.item_registry, held_item_rid);
        let is_critical = combat::is_critical_hit(on_ground, delta_y);
        let (strength_bonus, weakness_penalty) = self.get_attacker_bonuses(attacker_addr);

        // Check if target is a mob (PvE)
        if self.game_world.is_mob(victim_runtime_id) {
            let mob_pos = match self.game_world.mob_position(victim_runtime_id) {
                Some(p) => Vec3::new(p.0, p.1, p.2),
                None => return,
            };

            if attacker_pos.distance(&mob_pos) > 6.0 {
                return;
            }

            // PvE: no armor/protection/resistance on mobs (simplified)
            let damage = combat::calculate_damage(&combat::DamageInput {
                base_damage,
                weapon_nbt: &weapon_nbt,
                armor_defense: 0.0,
                armor_nbt_slots: &[],
                is_critical,
                strength_bonus,
                weakness_penalty,
                resistance_factor: 0.0,
            });

            let attacker_tick = self
                .connections
                .get(&attacker_addr)
                .map(|c| c.client_tick)
                .unwrap_or(0);

            if self
                .game_world
                .damage_mob(victim_runtime_id, damage, attacker_tick)
                .is_none()
            {
                return; // invulnerable
            }

            // Broadcast critical hit animation
            if is_critical {
                let attacker_rid = self
                    .connections
                    .get(&attacker_addr)
                    .map(|c| c.entity_runtime_id)
                    .unwrap_or(0);
                self.broadcast_packet(
                    packets::id::ANIMATE,
                    &Animate {
                        action_type: 4, // critical hit particles
                        entity_runtime_id: attacker_rid,
                    },
                )
                .await;
            }

            // Knockback (with enchantment bonus)
            let kb_enchant = combat::knockback_bonus(&weapon_nbt);
            let dx = mob_pos.x - attacker_pos.x;
            let dz = mob_pos.z - attacker_pos.z;
            let horizontal_len = (dx * dx + dz * dz).sqrt();
            if horizontal_len > 0.001 {
                let norm_x = dx / horizontal_len;
                let norm_z = dz / horizontal_len;
                let kb = 0.4 + kb_enchant as f32 * 0.3;
                let sprint_mult = if is_sprinting { 1.5 } else { 1.0 };
                let vx = norm_x * kb * sprint_mult;
                let vy = 0.4;
                let vz = norm_z * kb * sprint_mult;

                self.game_world
                    .apply_knockback(victim_runtime_id, vx, vy, vz);

                self.broadcast_packet(
                    packets::id::SET_ENTITY_MOTION,
                    &SetEntityMotion {
                        entity_runtime_id: victim_runtime_id,
                        motion: Vec3::new(vx, vy, vz),
                    },
                )
                .await;
            }

            return;
        }

        // --- PvP: find victim player by runtime_id ---
        let victim_addr = match self.find_addr_by_runtime_id(victim_runtime_id) {
            Some(a) => a,
            None => return,
        };

        let (victim_gamemode, victim_pos) = match self.connections.get(&victim_addr) {
            Some(c) if c.state == LoginState::InGame && !c.is_dead => (c.gamemode, c.position),
            _ => return,
        };

        if victim_gamemode == 1 || victim_gamemode == 3 {
            return;
        }

        let distance = attacker_pos.distance(&victim_pos);
        if distance > 6.0 {
            debug!("Attack rejected: distance {distance:.2} > 6.0 from {attacker_addr}");
            return;
        }

        // Invulnerability check (10 ticks = 500ms)
        let current_tick = self
            .connections
            .get(&attacker_addr)
            .map(|c| c.client_tick)
            .unwrap_or(0);
        if let Some(last_tick) = self
            .connections
            .get(&victim_addr)
            .and_then(|c| c.last_damage_tick)
        {
            if current_tick.saturating_sub(last_tick) < 10 {
                return;
            }
        }

        // Gather victim armor data for damage calculation
        let (armor_defense, armor_nbt_slots) = {
            let victim_conn = match self.connections.get(&victim_addr) {
                Some(c) => c,
                None => return,
            };
            let defense =
                combat::total_armor_defense(&self.item_registry, &victim_conn.inventory.armor);
            let nbt_slots: Vec<Vec<u8>> = victim_conn
                .inventory
                .armor
                .iter()
                .map(|item| item.nbt_data.clone())
                .collect();
            (defense, nbt_slots)
        };
        let armor_nbt_refs: Vec<&[u8]> = armor_nbt_slots.iter().map(|v| v.as_slice()).collect();
        let resistance_factor = self.get_resistance_factor(victim_addr);

        // Full damage pipeline
        let damage = combat::calculate_damage(&combat::DamageInput {
            base_damage,
            weapon_nbt: &weapon_nbt,
            armor_defense,
            armor_nbt_slots: &armor_nbt_refs,
            is_critical,
            strength_bonus,
            weakness_penalty,
            resistance_factor,
        });

        // Apply damage
        let (new_health, victim_rid, victim_name) = {
            let conn = match self.connections.get_mut(&victim_addr) {
                Some(c) => c,
                None => return,
            };
            conn.health = (conn.health - damage).max(0.0);
            conn.last_damage_tick = Some(current_tick);
            let name = conn
                .login_data
                .as_ref()
                .map(|d| d.display_name.clone())
                .unwrap_or_default();
            (conn.health, conn.entity_runtime_id, name)
        };

        // Send UpdateAttributes (health) to victim
        self.send_packet(
            victim_addr,
            packets::id::UPDATE_ATTRIBUTES,
            &UpdateAttributes::health(victim_rid, new_health, current_tick),
        )
        .await;

        // Broadcast EntityEvent(hurt) to all
        self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::hurt(victim_rid))
            .await;

        // Broadcast critical hit animation
        if is_critical {
            let attacker_rid = self
                .connections
                .get(&attacker_addr)
                .map(|c| c.entity_runtime_id)
                .unwrap_or(0);
            self.broadcast_packet(
                packets::id::ANIMATE,
                &Animate {
                    action_type: 4, // critical hit particles
                    entity_runtime_id: attacker_rid,
                },
            )
            .await;
        }

        // Knockback (with enchantment bonus)
        let kb_enchant = combat::knockback_bonus(&weapon_nbt);
        let dx = victim_pos.x - attacker_pos.x;
        let dz = victim_pos.z - attacker_pos.z;
        let horizontal_len = (dx * dx + dz * dz).sqrt();

        if horizontal_len > 0.001 {
            let norm_x = dx / horizontal_len;
            let norm_z = dz / horizontal_len;
            let kb_horizontal = 0.4 + kb_enchant as f32 * 0.3;
            let kb_vertical = 0.4;
            let sprint_mult = if is_sprinting { 1.5 } else { 1.0 };

            let motion = Vec3::new(
                norm_x * kb_horizontal * sprint_mult,
                kb_vertical,
                norm_z * kb_horizontal * sprint_mult,
            );

            self.send_packet(
                victim_addr,
                packets::id::SET_ENTITY_MOTION,
                &SetEntityMotion {
                    entity_runtime_id: victim_rid,
                    motion,
                },
            )
            .await;
        }

        // Fire Aspect: set victim on fire
        let fire_level = combat::fire_aspect_level(&weapon_nbt);
        if fire_level > 0 {
            if let Some(conn) = self.connections.get_mut(&victim_addr) {
                conn.fire_ticks = fire_level as i32 * 80; // 4 seconds per level
            }
        }

        // Check for death
        if new_health <= 0.0 {
            self.handle_player_death(victim_addr, &victim_name, attacker_addr)
                .await;
        }
    }

    /// Handle a player dying.
    async fn handle_player_death(
        &mut self,
        victim_addr: SocketAddr,
        victim_name: &str,
        killer_addr: SocketAddr,
    ) {
        let victim_rid = match self.connections.get(&victim_addr) {
            Some(c) => c.entity_runtime_id,
            None => return,
        };

        // Mark as dead
        if let Some(conn) = self.connections.get_mut(&victim_addr) {
            conn.is_dead = true;
        }

        // Broadcast death event to all
        self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::death(victim_rid))
            .await;

        // Send Respawn(searching) to dead player — triggers death screen
        let spawn_pos = Vec3::new(0.5, 5.62, 0.5);
        self.send_packet(
            victim_addr,
            packets::id::RESPAWN,
            &Respawn {
                position: spawn_pos,
                state: 0, // searching
                runtime_entity_id: victim_rid,
            },
        )
        .await;

        // Broadcast death message
        let killer_name = self
            .connections
            .get(&killer_addr)
            .and_then(|c| c.login_data.as_ref())
            .map(|d| d.display_name.clone())
            .unwrap_or_else(|| "???".to_string());

        let death_msg = Text::system(format!("{victim_name} was slain by {killer_name}"));
        self.broadcast_packet(packets::id::TEXT, &death_msg).await;
    }

    /// Handle a player death not caused by another player (fire, environment, etc.).
    async fn handle_player_death_generic(&mut self, victim_addr: SocketAddr, victim_name: &str) {
        let victim_rid = match self.connections.get(&victim_addr) {
            Some(c) => c.entity_runtime_id,
            None => return,
        };

        if let Some(conn) = self.connections.get_mut(&victim_addr) {
            conn.is_dead = true;
            conn.fire_ticks = 0;
            conn.effects.clear();
        }

        self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::death(victim_rid))
            .await;

        let spawn_pos = Vec3::new(0.5, 5.62, 0.5);
        self.send_packet(
            victim_addr,
            packets::id::RESPAWN,
            &Respawn {
                position: spawn_pos,
                state: 0,
                runtime_entity_id: victim_rid,
            },
        )
        .await;

        let death_msg = Text::system(format!("{victim_name} died"));
        self.broadcast_packet(packets::id::TEXT, &death_msg).await;
    }

    /// Handle Respawn packet from client (state=2, client clicked "Respawn").
    async fn handle_respawn(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame && c.is_dead => {}
            _ => return,
        }

        let pkt = match Respawn::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                warn!("Bad Respawn from {addr}: {e}");
                return;
            }
        };

        // Client sends state=2 (client_ready) when "Respawn" is clicked
        if pkt.state != 2 {
            return;
        }

        let spawn_pos = Vec3::new(0.5, 5.62, 0.5);

        let runtime_id = match self.connections.get_mut(&addr) {
            Some(conn) => {
                conn.health = 20.0;
                conn.is_dead = false;
                conn.last_damage_tick = None;
                conn.position = spawn_pos;
                conn.effects.clear();
                conn.fire_ticks = 0;
                conn.entity_runtime_id
            }
            None => return,
        };

        // Send Respawn(server_ready)
        self.send_packet(
            addr,
            packets::id::RESPAWN,
            &Respawn {
                position: spawn_pos,
                state: 1, // server_ready
                runtime_entity_id: runtime_id,
            },
        )
        .await;

        // Send full health
        let tick = self
            .connections
            .get(&addr)
            .map(|c| c.client_tick)
            .unwrap_or(0);
        self.send_packet(
            addr,
            packets::id::UPDATE_ATTRIBUTES,
            &UpdateAttributes::health(runtime_id, 20.0, tick),
        )
        .await;

        // Broadcast position reset
        let move_pkt = MovePlayer::reset(runtime_id, spawn_pos, 0.0, 0.0, 0.0, true, tick);
        self.broadcast_packet(packets::id::MOVE_PLAYER, &move_pkt)
            .await;
    }

    // -----------------------------------------------------------------------
    // Inventory handlers
    // -----------------------------------------------------------------------

    /// Send the full inventory contents to a player.
    async fn send_inventory(&mut self, addr: SocketAddr) {
        let items = match self.connections.get(&addr) {
            Some(c) => c.inventory.main.clone(),
            None => return,
        };
        self.send_packet(
            addr,
            packets::id::INVENTORY_CONTENT,
            &InventoryContent {
                window_id: 0,
                items,
            },
        )
        .await;

        let armor = match self.connections.get(&addr) {
            Some(c) => c.inventory.armor.clone(),
            None => return,
        };
        self.send_packet(
            addr,
            packets::id::INVENTORY_CONTENT,
            &InventoryContent {
                window_id: 119,
                items: armor,
            },
        )
        .await;
    }

    async fn handle_mob_equipment(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        let state = match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame => c.state,
            _ => return,
        };
        let _ = state;

        let equipment = match MobEquipment::proto_decode(buf) {
            Ok(e) => e,
            Err(e) => {
                debug!("Bad MobEquipment from {addr}: {e}");
                return;
            }
        };

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.inventory.held_slot = equipment.hotbar_slot;
        }

        // Broadcast to other players
        let entity_runtime_id = match self.connections.get(&addr) {
            Some(c) => c.entity_runtime_id,
            None => return,
        };

        let broadcast_pkt = MobEquipment {
            entity_runtime_id,
            item: equipment.item,
            inventory_slot: equipment.inventory_slot,
            hotbar_slot: equipment.hotbar_slot,
            window_id: equipment.window_id,
        };
        self.broadcast_packet_except(addr, packets::id::MOB_EQUIPMENT, &broadcast_pkt)
            .await;
    }

    async fn handle_item_stack_request(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        let state = match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame => c.state,
            _ => return,
        };
        let _ = state;

        let request = match ItemStackRequest::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                debug!("Bad ItemStackRequest from {addr}: {e}");
                return;
            }
        };

        let mut responses = Vec::new();
        for req in &request.requests {
            // Need to borrow inventory mutably but also need item_registry immutably.
            // Extract response using a helper that takes both.
            let response = match self.connections.get_mut(&addr) {
                Some(conn) => conn.inventory.process_request(req, &self.item_registry),
                None => return,
            };
            responses.push(response);
        }

        self.send_packet(
            addr,
            packets::id::ITEM_STACK_RESPONSE,
            &ItemStackResponse { responses },
        )
        .await;
    }

    async fn broadcast_packet(&mut self, packet_id: u32, packet: &impl ProtoEncode) {
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

    async fn broadcast_packet_except(
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
    fn get_block(&self, x: i32, y: i32, z: i32) -> Option<u32> {
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
    fn set_block(&mut self, x: i32, y: i32, z: i32, runtime_id: u32) -> bool {
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
        true
    }

    /// Compute the target position when placing a block on a face.
    fn face_offset(pos: BlockPos, face: i32) -> BlockPos {
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

    async fn handle_player_action(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame => {}
            _ => return,
        }

        let action = match PlayerAction::proto_decode(buf) {
            Ok(a) => a,
            Err(e) => {
                warn!("Bad PlayerAction from {addr}: {e}");
                return;
            }
        };

        match action.action {
            PlayerActionType::StartBreak => {
                if let Some(conn) = self.connections.get_mut(&addr) {
                    conn.breaking_block = Some((action.block_position, Instant::now()));
                }
                debug!("StartBreak at {} by {addr}", action.block_position);
            }
            PlayerActionType::AbortBreak => {
                if let Some(conn) = self.connections.get_mut(&addr) {
                    conn.breaking_block = None;
                }
                debug!("AbortBreak by {addr}");
            }
            other => {
                debug!("PlayerAction {:?} from {addr}", other);
            }
        }
    }

    async fn handle_inventory_transaction(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame && !c.is_dead => {}
            _ => return,
        }

        let transaction = match InventoryTransaction::proto_decode(buf) {
            Ok(t) => t,
            Err(e) => {
                warn!("Bad InventoryTransaction from {addr}: {e}");
                return;
            }
        };

        // Handle UseItemOnEntity (attack/interact) first
        if let Some(entity_data) = transaction.use_item_on_entity {
            if entity_data.action == UseItemOnEntityAction::Attack {
                self.handle_attack(addr, entity_data.entity_runtime_id)
                    .await;
            }
            return;
        }

        let use_item = match transaction.use_item {
            Some(data) => data,
            None => return,
        };

        match use_item.action {
            UseItemAction::BreakBlock => {
                let pos = use_item.block_position;

                // Check bounds
                if pos.y < OVERWORLD_MIN_Y || pos.y > Self::MAX_Y {
                    return;
                }

                // Get old block before replacing
                let old_runtime_id = match self.get_block(pos.x, pos.y, pos.z) {
                    Some(id) => id,
                    None => return,
                };

                let air_hash = self.flat_world_blocks.air;

                // Skip if already air
                if old_runtime_id == air_hash {
                    return;
                }

                // Don't allow breaking unbreakable blocks (bedrock, barriers, etc.)
                if let Some(hardness) = self.block_registry.hardness(old_runtime_id) {
                    if hardness < 0.0 {
                        debug!("Rejected break of unbreakable block at {pos} by {addr}");
                        return;
                    }
                }

                // Mining time validation for survival mode
                let gamemode = self.connections.get(&addr).map(|c| c.gamemode).unwrap_or(0);

                if gamemode == 0 {
                    // Survival mode: validate mining time
                    let breaking_info = self.connections.get(&addr).and_then(|c| c.breaking_block);

                    if let Some(expected_secs) =
                        self.block_registry.expected_mining_secs(old_runtime_id)
                    {
                        if expected_secs > 0.0 {
                            match breaking_info {
                                Some((break_pos, start_time)) if break_pos == pos => {
                                    let elapsed = start_time.elapsed().as_secs_f32();
                                    let min_allowed = expected_secs * 0.8;
                                    if elapsed < min_allowed {
                                        debug!(
                                            "Mining too fast at {pos} by {addr}: {elapsed:.2}s < {min_allowed:.2}s (expected {expected_secs:.2}s)"
                                        );
                                        return;
                                    }
                                }
                                _ => {
                                    // No StartBreak recorded for this position
                                    debug!("No StartBreak for {pos} from {addr}, rejecting break");
                                    return;
                                }
                            }
                        }
                    }

                    // Clear breaking state
                    if let Some(conn) = self.connections.get_mut(&addr) {
                        conn.breaking_block = None;
                    }
                }

                // Set to air
                if !self.set_block(pos.x, pos.y, pos.z, air_hash) {
                    return;
                }

                // Send UpdateBlock to all players
                let update = UpdateBlock::new(pos, air_hash);
                self.broadcast_packet(packets::id::UPDATE_BLOCK, &update)
                    .await;

                // Send LevelEvent (destroy particles) to all players
                let event = LevelEvent::destroy_block(pos.x, pos.y, pos.z, old_runtime_id);
                self.broadcast_packet(packets::id::LEVEL_EVENT, &event)
                    .await;

                debug!("Block broken at {pos} by {addr}");
            }
            UseItemAction::ClickBlock => {
                let target = Self::face_offset(use_item.block_position, use_item.face);

                // Check bounds
                if target.y < OVERWORLD_MIN_Y || target.y > Self::MAX_Y {
                    return;
                }

                // Get the block runtime ID from the held item
                let block_runtime_id = use_item.held_item_block_runtime_id;
                if block_runtime_id <= 0 {
                    return;
                }
                let block_runtime_id = block_runtime_id as u32;

                // Don't place air
                let air_hash = self.flat_world_blocks.air;
                if block_runtime_id == air_hash {
                    return;
                }

                // Check that target is in a loaded chunk
                let cx = target.x >> 4;
                let cz = target.z >> 4;
                if !self.world_chunks.contains_key(&(cx, cz)) {
                    return;
                }

                // Set the block
                if !self.set_block(target.x, target.y, target.z, block_runtime_id) {
                    return;
                }

                // Send UpdateBlock to all players
                let update = UpdateBlock::new(target, block_runtime_id);
                self.broadcast_packet(packets::id::UPDATE_BLOCK, &update)
                    .await;

                debug!("Block placed at {target} by {addr}");
            }
            UseItemAction::ClickAir => {
                // Nothing to do for creative mode
            }
        }
    }

    // -----------------------------------------------------------------------
    // Packet sending
    // -----------------------------------------------------------------------

    async fn send_packet(&mut self, addr: SocketAddr, packet_id: u32, packet: &impl ProtoEncode) {
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

    // -----------------------------------------------------------------------
    // Phase 2.1: Multi-player (PlayerList, AddPlayer, RemoveEntity)
    // -----------------------------------------------------------------------

    /// Send PlayerList(Add) with all existing InGame players to a newly joined player.
    async fn send_existing_players_to(&mut self, new_addr: SocketAddr) {
        let entries: Vec<PlayerListAdd> = self
            .connections
            .iter()
            .filter(|(&a, c)| a != new_addr && c.state == LoginState::InGame)
            .filter_map(|(_, conn)| {
                let login = conn.login_data.as_ref()?;
                let uuid = Uuid::parse(&login.identity).unwrap_or(Uuid::ZERO);
                let client_data = conn.client_data.clone().unwrap_or_default();
                Some(PlayerListAdd {
                    uuid,
                    entity_unique_id: conn.entity_unique_id,
                    username: login.display_name.clone(),
                    xuid: login.xuid.clone(),
                    platform_chat_id: String::new(),
                    device_os: client_data.device_os,
                    skin_data: client_data,
                    is_teacher: false,
                    is_host: false,
                    is_sub_client: false,
                })
            })
            .collect();

        if !entries.is_empty() {
            self.send_packet(
                new_addr,
                packets::id::PLAYER_LIST,
                &PlayerListAddPacket { entries },
            )
            .await;
        }
    }

    /// Broadcast PlayerList(Add) for the new player to all InGame players (including self).
    async fn broadcast_new_player_list(&mut self, new_addr: SocketAddr) {
        let entry = {
            let conn = match self.connections.get(&new_addr) {
                Some(c) => c,
                None => return,
            };
            let login = match &conn.login_data {
                Some(d) => d,
                None => return,
            };
            let uuid = Uuid::parse(&login.identity).unwrap_or(Uuid::ZERO);
            let client_data = conn.client_data.clone().unwrap_or_default();
            PlayerListAdd {
                uuid,
                entity_unique_id: conn.entity_unique_id,
                username: login.display_name.clone(),
                xuid: login.xuid.clone(),
                platform_chat_id: String::new(),
                device_os: client_data.device_os,
                skin_data: client_data,
                is_teacher: false,
                is_host: false,
                is_sub_client: false,
            }
        };
        let packet = PlayerListAddPacket {
            entries: vec![entry],
        };
        self.broadcast_packet(packets::id::PLAYER_LIST, &packet)
            .await;
    }

    /// Send AddPlayer for each existing InGame player to a newly joined player.
    async fn send_existing_add_players_to(&mut self, new_addr: SocketAddr) {
        let ops = &self.permissions.ops;
        let players: Vec<AddPlayer> = self
            .connections
            .iter()
            .filter(|(&a, c)| a != new_addr && c.state == LoginState::InGame)
            .filter_map(|(_, conn)| {
                let login = conn.login_data.as_ref()?;
                let uuid = Uuid::parse(&login.identity).unwrap_or(Uuid::ZERO);
                let client_data = conn.client_data.clone().unwrap_or_default();
                let held_item = conn
                    .inventory
                    .get_slot(0, conn.inventory.held_slot)
                    .cloned()
                    .unwrap_or_else(mc_rs_proto::item_stack::ItemStack::empty);
                let is_op = ops.contains(&login.display_name);
                Some(AddPlayer {
                    uuid,
                    username: login.display_name.clone(),
                    entity_runtime_id: conn.entity_runtime_id,
                    platform_chat_id: String::new(),
                    position: conn.position,
                    velocity: Vec3::ZERO,
                    pitch: conn.pitch,
                    yaw: conn.yaw,
                    head_yaw: conn.head_yaw,
                    held_item,
                    gamemode: conn.gamemode,
                    metadata: default_player_metadata(&login.display_name),
                    entity_unique_id: conn.entity_unique_id,
                    permission_level: if is_op { 2 } else { 1 },
                    command_permission_level: if is_op { 1 } else { 0 },
                    device_id: client_data.device_id,
                    device_os: client_data.device_os,
                })
            })
            .collect();

        for player in &players {
            self.send_packet(new_addr, packets::id::ADD_PLAYER, player)
                .await;
        }
    }

    /// Send AddActor for all existing mobs to a newly joined player.
    async fn send_existing_mobs_to(&mut self, addr: SocketAddr) {
        let mobs = self.game_world.all_mobs();
        for mob in mobs {
            let pkt = AddActor {
                entity_unique_id: mob.unique_id,
                entity_runtime_id: mob.runtime_id,
                entity_type: mob.mob_type,
                position: Vec3::new(mob.position.0, mob.position.1, mob.position.2),
                velocity: Vec3::ZERO,
                pitch: mob.pitch,
                yaw: mob.yaw,
                head_yaw: mob.head_yaw,
                body_yaw: mob.yaw,
                attributes: vec![ActorAttribute {
                    name: "minecraft:health".to_string(),
                    min: 0.0,
                    max: mob.max_health,
                    current: mob.health,
                    default: mob.max_health,
                }],
                metadata: default_mob_metadata(mob.bb_width, mob.bb_height),
            };
            self.send_packet(addr, packets::id::ADD_ACTOR, &pkt).await;
        }
    }

    /// Broadcast AddPlayer for the new player to all existing InGame players.
    async fn broadcast_add_player(&mut self, new_addr: SocketAddr) {
        let packet = {
            let conn = match self.connections.get(&new_addr) {
                Some(c) => c,
                None => return,
            };
            let login = match &conn.login_data {
                Some(d) => d,
                None => return,
            };
            let uuid = Uuid::parse(&login.identity).unwrap_or(Uuid::ZERO);
            let client_data = conn.client_data.clone().unwrap_or_default();
            let held_item = conn
                .inventory
                .get_slot(0, conn.inventory.held_slot)
                .cloned()
                .unwrap_or_else(mc_rs_proto::item_stack::ItemStack::empty);
            let is_op = self.permissions.ops.contains(&login.display_name);
            AddPlayer {
                uuid,
                username: login.display_name.clone(),
                entity_runtime_id: conn.entity_runtime_id,
                platform_chat_id: String::new(),
                position: conn.position,
                velocity: Vec3::ZERO,
                pitch: conn.pitch,
                yaw: conn.yaw,
                head_yaw: conn.head_yaw,
                held_item,
                gamemode: conn.gamemode,
                metadata: default_player_metadata(&login.display_name),
                entity_unique_id: conn.entity_unique_id,
                permission_level: if is_op { 2 } else { 1 },
                command_permission_level: if is_op { 1 } else { 0 },
                device_id: client_data.device_id,
                device_os: client_data.device_os,
            }
        };
        self.broadcast_packet_except(new_addr, packets::id::ADD_PLAYER, &packet)
            .await;
    }

    // ------------------------------------------------------------------
    // Status effect management
    // ------------------------------------------------------------------

    /// Apply a status effect to a player, sending the MobEffect packet.
    async fn apply_effect(
        &mut self,
        addr: SocketAddr,
        effect_id: i32,
        amplifier: i32,
        duration_ticks: i32,
    ) {
        let runtime_id = match self.connections.get_mut(&addr) {
            Some(conn) => {
                // Remove existing effect of same type
                conn.effects.retain(|e| e.effect_id != effect_id);
                conn.effects.push(ActiveEffect {
                    effect_id,
                    amplifier,
                    remaining_ticks: duration_ticks,
                });
                conn.entity_runtime_id
            }
            None => return,
        };

        self.send_packet(
            addr,
            packets::id::MOB_EFFECT,
            &MobEffect::add(runtime_id, effect_id, amplifier, duration_ticks, true),
        )
        .await;
    }

    /// Remove a status effect from a player, sending the MobEffect(remove) packet.
    #[allow(dead_code)]
    async fn remove_effect(&mut self, addr: SocketAddr, effect_id: i32) {
        let runtime_id = match self.connections.get_mut(&addr) {
            Some(conn) => {
                conn.effects.retain(|e| e.effect_id != effect_id);
                conn.entity_runtime_id
            }
            None => return,
        };

        self.send_packet(
            addr,
            packets::id::MOB_EFFECT,
            &MobEffect::remove(runtime_id, effect_id),
        )
        .await;
    }

    /// Remove all status effects from a player.
    async fn clear_effects(&mut self, addr: SocketAddr) {
        let (runtime_id, effect_ids) = match self.connections.get_mut(&addr) {
            Some(conn) => {
                let ids: Vec<i32> = conn.effects.iter().map(|e| e.effect_id).collect();
                conn.effects.clear();
                (conn.entity_runtime_id, ids)
            }
            None => return,
        };

        for eid in effect_ids {
            self.send_packet(
                addr,
                packets::id::MOB_EFFECT,
                &MobEffect::remove(runtime_id, eid),
            )
            .await;
        }
    }

    /// Tick all active effects for all players. Called once per game tick (50ms).
    async fn tick_effects(&mut self) {
        // Collect addresses of all in-game players
        let addrs: Vec<SocketAddr> = self
            .connections
            .iter()
            .filter(|(_, c)| c.state == LoginState::InGame && !c.is_dead)
            .map(|(a, _)| *a)
            .collect();

        for addr in addrs {
            let conn = match self.connections.get_mut(&addr) {
                Some(c) => c,
                None => continue,
            };

            // Tick fire damage
            if conn.fire_ticks > 0 {
                conn.fire_ticks -= 1;
                if conn.fire_ticks % 20 == 0 && conn.fire_ticks >= 0 {
                    // Deal 1 fire damage every second (20 ticks)
                    conn.health = (conn.health - 1.0).max(0.0);
                    let rid = conn.entity_runtime_id;
                    let hp = conn.health;
                    let tick = conn.client_tick;
                    self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::hurt(rid))
                        .await;
                    self.send_packet(
                        addr,
                        packets::id::UPDATE_ATTRIBUTES,
                        &UpdateAttributes::health(rid, hp, tick),
                    )
                    .await;
                    if hp <= 0.0 {
                        let name = self
                            .connections
                            .get(&addr)
                            .and_then(|c| c.login_data.as_ref())
                            .map(|d| d.display_name.clone())
                            .unwrap_or_default();
                        self.handle_player_death_generic(addr, &name).await;
                        continue;
                    }
                }
            }

            // Tick effect durations
            let mut expired = Vec::new();
            if let Some(conn) = self.connections.get_mut(&addr) {
                for effect in &mut conn.effects {
                    effect.remaining_ticks -= 1;
                    if effect.remaining_ticks <= 0 {
                        expired.push(effect.effect_id);
                    }
                }
                conn.effects.retain(|e| e.remaining_ticks > 0);
            }

            // Send remove packets for expired effects
            if let Some(conn) = self.connections.get(&addr) {
                let rid = conn.entity_runtime_id;
                for eid in expired {
                    self.send_packet(addr, packets::id::MOB_EFFECT, &MobEffect::remove(rid, eid))
                        .await;
                }
            }
        }
    }

    /// Get combat bonuses from active effects for an attacker.
    /// Returns (strength_bonus, weakness_penalty).
    fn get_attacker_bonuses(&self, addr: SocketAddr) -> (f32, f32) {
        use mc_rs_proto::packets::mob_effect::effect_id as eid;

        let conn = match self.connections.get(&addr) {
            Some(c) => c,
            None => return (0.0, 0.0),
        };

        let mut strength = 0.0_f32;
        let mut weakness = 0.0_f32;

        for effect in &conn.effects {
            match effect.effect_id {
                eid::STRENGTH => strength += 3.0 * (effect.amplifier as f32 + 1.0),
                eid::WEAKNESS => weakness += 4.0,
                _ => {}
            }
        }

        (strength, weakness)
    }

    /// Get damage resistance factor from victim's effects and armor.
    /// Returns the resistance factor (0.0 to 1.0).
    fn get_resistance_factor(&self, addr: SocketAddr) -> f32 {
        use mc_rs_proto::packets::mob_effect::effect_id as eid;

        let conn = match self.connections.get(&addr) {
            Some(c) => c,
            None => return 0.0,
        };

        let mut factor = 0.0_f32;
        for effect in &conn.effects {
            if effect.effect_id == eid::RESISTANCE {
                factor += 0.2 * (effect.amplifier as f32 + 1.0);
            }
        }

        factor.min(1.0)
    }
}

/// Map an effect name to its Bedrock protocol ID.
fn effect_name_to_id(name: &str) -> Option<i32> {
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
fn encode_sub_packet(packet_id: u32, packet: &impl ProtoEncode) -> Bytes {
    let mut buf = BytesMut::new();
    VarUInt32(packet_id).proto_encode(&mut buf);
    packet.proto_encode(&mut buf);
    buf.freeze()
}

fn gamemode_from_str(s: &str) -> i32 {
    match s.to_lowercase().as_str() {
        "survival" => 0,
        "creative" => 1,
        "adventure" => 2,
        "spectator" => 3,
        _ => 0,
    }
}

fn difficulty_from_str(s: &str) -> i32 {
    match s.to_lowercase().as_str() {
        "peaceful" => 0,
        "easy" => 1,
        "normal" => 2,
        "hard" => 3,
        _ => 2,
    }
}

fn generator_from_str(s: &str) -> i32 {
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

fn parse_gamemode(s: &str) -> Option<i32> {
    match s.to_lowercase().as_str() {
        "0" | "survival" | "s" => Some(0),
        "1" | "creative" | "c" => Some(1),
        "2" | "adventure" | "a" => Some(2),
        "3" | "spectator" | "sp" => Some(3),
        _ => None,
    }
}

fn gamemode_name(gm: i32) -> &'static str {
    match gm {
        0 => "Survival",
        1 => "Creative",
        2 => "Adventure",
        3 => "Spectator",
        _ => "Unknown",
    }
}

fn parse_coords(xs: &str, ys: &str, zs: &str) -> Option<(f32, f32, f32)> {
    let x = xs.parse::<f32>().ok()?;
    let y = ys.parse::<f32>().ok()?;
    let z = zs.parse::<f32>().ok()?;
    Some((x, y, z))
}

/// Return base attack damage for a held item, looked up by runtime ID.
fn base_attack_damage(registry: &ItemRegistry, runtime_id: i32) -> f32 {
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
fn default_mob_metadata(bb_width: f32, bb_height: f32) -> Vec<EntityMetadataEntry> {
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
