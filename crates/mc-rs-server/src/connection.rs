//! Per-player connection state management and login flow.

use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use bytes::{BufMut, Bytes, BytesMut};
use tracing::{debug, info, warn};

use mc_rs_command::CommandRegistry;
use mc_rs_crypto::{
    create_handshake_jwt, derive_key, parse_client_public_key, PacketEncryption, ServerKeyPair,
};
use mc_rs_proto::batch::{decode_batch, encode_single, BatchConfig};
use mc_rs_proto::codec::{ProtoDecode, ProtoEncode};
use mc_rs_proto::compression::CompressionAlgorithm;
use mc_rs_proto::jwt;
use mc_rs_proto::packets::{
    self, AvailableCommands, AvailableEntityIdentifiers, BiomeDefinitionList, ChunkRadiusUpdated,
    ClientToServerHandshake, CommandOutput, CommandRequest, CreativeContent, Disconnect,
    InventoryTransaction, LevelChunk, LevelEvent, MovePlayer, NetworkChunkPublisherUpdate,
    NetworkSettings, PlayStatus, PlayStatusType, PlayerAction, PlayerActionType, PlayerAuthInput,
    RequestChunkRadius, ResourcePackClientResponse, ResourcePackResponseStatus, ResourcePackStack,
    ResourcePacksInfo, ServerToClientHandshake, SetLocalPlayerAsInitialized, StartGame, Text,
    UpdateBlock, UseItemAction,
};
use mc_rs_proto::types::{BlockPos, VarUInt32, Vec2, Vec3};
use mc_rs_raknet::{RakNetEvent, Reliability, ServerHandle};
use mc_rs_world::block_hash::FlatWorldBlocks;
use mc_rs_world::block_registry::BlockRegistry;
use mc_rs_world::chunk::{ChunkColumn, OVERWORLD_MIN_Y, OVERWORLD_SUB_CHUNK_COUNT};
use mc_rs_world::flat_generator::generate_flat_chunk;
use mc_rs_world::serializer::serialize_chunk_column;
use tokio::sync::watch;

use crate::config::ServerConfig;

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
}

/// Manages all player connections and their login state machines.
pub struct ConnectionHandler {
    connections: HashMap<SocketAddr, PlayerConnection>,
    server_handle: ServerHandle,
    online_mode: bool,
    next_entity_id: i64,
    server_config: Arc<ServerConfig>,
    flat_world_blocks: FlatWorldBlocks,
    command_registry: CommandRegistry,
    shutdown_tx: Arc<watch::Sender<bool>>,
    /// Cached world chunks — blocks persist across player interactions.
    world_chunks: HashMap<(i32, i32), ChunkColumn>,
    /// Block property registry for all vanilla blocks.
    block_registry: BlockRegistry,
}

impl ConnectionHandler {
    pub fn new(
        server_handle: ServerHandle,
        online_mode: bool,
        server_config: Arc<ServerConfig>,
        shutdown_tx: Arc<watch::Sender<bool>>,
    ) -> Self {
        Self {
            connections: HashMap::new(),
            server_handle,
            online_mode,
            next_entity_id: 1,
            server_config,
            flat_world_blocks: FlatWorldBlocks::compute(),
            command_registry: CommandRegistry::new(),
            shutdown_tx,
            world_chunks: HashMap::new(),
            block_registry: BlockRegistry::new(),
        }
    }

    fn allocate_entity_id(&mut self) -> i64 {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }

    /// Process a RakNet event.
    pub async fn handle_event(&mut self, event: RakNetEvent) {
        match event {
            RakNetEvent::SessionConnected { addr, guid } => {
                self.handle_session_connected(addr, guid);
            }
            RakNetEvent::SessionDisconnected { addr } => {
                self.handle_session_disconnected(addr);
            }
            RakNetEvent::Packet { addr, payload } => {
                self.handle_packet(addr, payload).await;
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
            },
        );
    }

    fn handle_session_disconnected(&mut self, addr: SocketAddr) {
        if let Some(conn) = self.connections.remove(&addr) {
            if let Some(data) = &conn.login_data {
                info!("Player {} disconnected ({addr})", data.display_name);
            } else {
                info!("Session disconnected: {addr}");
            }
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
                packets::id::INVENTORY_TRANSACTION => {
                    self.handle_inventory_transaction(addr, &mut cursor).await;
                }
                packets::id::PLAYER_ACTION => {
                    self.handle_player_action(addr, &mut cursor).await;
                }
                packets::id::PLAYER_AUTH_INPUT => {
                    self.handle_player_auth_input(addr, &mut cursor).await;
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

        info!(
            "Login from {addr}: {} (XUID: {}, UUID: {})",
            login_data.display_name, login_data.xuid, login_data.identity
        );

        if self.online_mode {
            self.start_encryption_handshake(addr, login_data).await;
        } else {
            if let Some(conn) = self.connections.get_mut(&addr) {
                conn.login_data = Some(login_data);
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
            ..StartGame::default()
        };

        self.send_packet(addr, packets::id::START_GAME, &start_game)
            .await;
        info!("Sent StartGame to {addr}");

        // Send metadata packets
        self.send_packet(
            addr,
            packets::id::CREATIVE_CONTENT,
            &CreativeContent::default(),
        )
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

            info!(
                "Sent ChunkRadiusUpdated({accepted_radius}) + {} chunks + PlayStatus(PlayerSpawn) to {addr}",
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

        let name = self
            .connections
            .get(&addr)
            .and_then(|c| c.login_data.as_ref())
            .map(|d| d.display_name.as_str())
            .unwrap_or("unknown");

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
            Some(c) if c.state == LoginState::InGame => {}
            _ => return,
        }

        let input = match PlayerAuthInput::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                warn!("Bad PlayerAuthInput from {addr}: {e}");
                return;
            }
        };

        let (prev_position, entity_runtime_id) = match self.connections.get(&addr) {
            Some(c) => (c.position, c.entity_runtime_id),
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

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.position = input.position;
            conn.pitch = input.pitch;
            conn.yaw = input.yaw;
            conn.head_yaw = input.head_yaw;
            conn.client_tick = input.tick;
            conn.on_ground = on_ground;
        }

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

        // Prepare context — inject special args for certain commands
        let args = match cmd_name {
            "help" => {
                // Inject command list as "name:description" pairs
                self.command_registry
                    .get_commands()
                    .values()
                    .map(|e| format!("{}:{}", e.name, e.description))
                    .collect()
            }
            "list" => {
                // Inject online player names
                self.connections
                    .values()
                    .filter(|c| c.state == LoginState::InGame)
                    .filter_map(|c| c.login_data.as_ref())
                    .map(|d| d.display_name.clone())
                    .collect()
            }
            _ => raw_args,
        };

        let ctx = mc_rs_command::CommandContext {
            sender_name: sender_name.clone(),
            args,
        };

        let result = self.command_registry.execute(cmd_name, &ctx);

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

    async fn broadcast_packet(&mut self, packet_id: u32, packet: &impl ProtoEncode) {
        let addrs: Vec<SocketAddr> = self.connections.keys().copied().collect();
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
            Some(c) if c.state == LoginState::InGame => {}
            _ => return,
        }

        let transaction = match InventoryTransaction::proto_decode(buf) {
            Ok(t) => t,
            Err(e) => {
                warn!("Bad InventoryTransaction from {addr}: {e}");
                return;
            }
        };

        let use_item = match transaction.use_item {
            Some(data) => data,
            None => return, // Not a UseItem transaction
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
