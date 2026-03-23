use std::collections::{HashSet, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use base64::Engine;
use tracing::{debug, info, warn};

use mc_rs_crypto::ecdh::{self, ServerKeyPair};
use mc_rs_crypto::encrypt::EncryptionContext;
use mc_rs_crypto::jwt;
use mc_rs_proto::batch::{self, CompressionAlgorithm};
use mc_rs_proto::codec;
use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::chunks::*;
use mc_rs_proto::packets::login::*;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::*;
use mc_rs_proto::packets::world::*;

use crate::config::ConnectionConfig;
use crate::inventory::PlayerInventory;
use crate::player_data;
use crate::player_registry;
use crate::world::chunk_cache::ChunkCache;
use crate::world::flat_generator;
use crate::world::terrain_generator;

/// Connection state for a single player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ConnectionState {
    SessionStart,
    Login,
    Handshake,
    ResourcePacks,
    PreSpawn,
    SpawnResponse,
    InGame,
}

/// Manages a single client connection's protocol state machine.
pub struct Connection {
    pub addr: SocketAddr,
    pub state: ConnectionState,
    pub encryption: Option<EncryptionContext>,
    /// Encryption key waiting to be activated AFTER current batch is sent.
    pending_encryption_key: Option<[u8; 32]>,
    pub compression_algo: CompressionAlgorithm,

    // Player identity (set after login)
    pub display_name: Option<String>,
    pub uuid: Option<uuid::Uuid>,
    pub xuid: Option<String>,
    pub client_pub_key_b64: Option<String>,

    // Player state
    pub position: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub entity_runtime_id: u64,
    pub tick: u64,
    pub gamemode: i32, // 0=survival, 1=creative, 2=adventure, 3=spectator

    // Chunk tracking
    pub sent_chunks: HashSet<(i32, i32)>,
    pub view_distance: i32,
    pub last_chunk_x: i32,
    pub last_chunk_z: i32,
    /// Queue of chunks to send, ordered by distance (nearest first).
    pub chunk_load_queue: VecDeque<(i32, i32)>,
    /// Countdown ticks until chunk reorder. 0 = reorder now. u32::MAX = idle.
    pub chunk_order_countdown: u32,

    // Packets to broadcast to ALL other players
    pub broadcasts: Vec<Vec<u8>>,

    // Server-side actions from commands (read by main.rs)
    pub pending_actions: Vec<mc_rs_command::CommandAction>,

    // Server keypair (shared across connections)
    server_keypair: std::sync::Arc<ServerKeyPair>,

    // Player inventory
    pub inventory: PlayerInventory,

    // Shared chunk cache (world persistence)
    chunk_cache: Arc<Mutex<ChunkCache>>,

    // Server config subset for this connection
    config: Arc<ConnectionConfig>,
}

impl Connection {
    pub fn new(
        addr: SocketAddr,
        server_keypair: std::sync::Arc<ServerKeyPair>,
        chunk_cache: Arc<Mutex<ChunkCache>>,
        config: Arc<ConnectionConfig>,
    ) -> Self {
        Self {
            addr,
            state: ConnectionState::SessionStart,
            encryption: None,
            pending_encryption_key: None,
            compression_algo: CompressionAlgorithm::Zlib,
            display_name: None,
            uuid: None,
            xuid: None,
            client_pub_key_b64: None,
            position: {
                // PMMP-style safe spawn: find highest solid block, then 2 air blocks above
                let seed = config.world_seed;
                let surface_y = terrain_generator::get_surface_height(0, 0, seed);
                // Place feet 1 block above surface, eyes at feet + 1.621
                let feet_y = (surface_y + 1) as f32;
                let eye_y = feet_y + 1.621;
                [0.5, eye_y, 0.5]
            },
            pitch: 0.0,
            yaw: 0.0,
            head_yaw: 0.0,
            entity_runtime_id: player_registry::next_entity_id() as u64,
            tick: 0,
            gamemode: config.default_gamemode,
            sent_chunks: HashSet::new(),
            view_distance: config.max_view_distance,
            last_chunk_x: 0,
            last_chunk_z: 0,
            chunk_load_queue: VecDeque::new(),
            chunk_order_countdown: 5, // reorder shortly after spawn (like PMMP)
            broadcasts: Vec::new(),
            pending_actions: Vec::new(),
            inventory: PlayerInventory::new(),
            server_keypair,
            chunk_cache,
            config,
        }
    }

    /// Reorder the chunk load queue: spiral from player position, unload distant chunks.
    /// Called when the player changes chunk or periodically.
    pub fn order_chunks(&mut self) {
        self.chunk_load_queue.clear();
        let cx = self.last_chunk_x;
        let cz = self.last_chunk_z;
        let r = self.view_distance;
        let r_sq = r * r;

        // Collect all chunks in circular view distance, sorted by distance
        let mut candidates: Vec<(i32, i32, i32)> = Vec::new();
        for dx in -r..=r {
            for dz in -r..=r {
                let dist_sq = dx * dx + dz * dz;
                if dist_sq <= r_sq {
                    let chunk = (cx + dx, cz + dz);
                    if !self.sent_chunks.contains(&chunk) {
                        candidates.push((cx + dx, cz + dz, dist_sq));
                    }
                }
            }
        }

        // Sort by distance (nearest first = spiral-like)
        candidates.sort_by_key(|&(_, _, d)| d);

        for (x, z, _) in candidates {
            self.chunk_load_queue.push_back((x, z));
        }

        // Unload chunks outside view distance (+2 margin)
        let unload_r_sq = (r + 2) * (r + 2);
        let old: Vec<(i32, i32)> = self
            .sent_chunks
            .iter()
            .filter(|&&(sx, sz)| {
                let dx = sx - cx;
                let dz = sz - cz;
                dx * dx + dz * dz > unload_r_sq
            })
            .copied()
            .collect();
        for chunk in old {
            self.sent_chunks.remove(&chunk);
        }
    }

    /// Send up to CHUNKS_PER_TICK chunks from the queue.
    /// Called from the main tick loop, not from packet handlers.
    /// Returns response packets to send to this player.
    pub fn send_queued_chunks(&mut self) -> Vec<Vec<u8>> {
        const CHUNKS_PER_TICK: usize = 8;

        // PMMP-style countdown: doChunkRequests() { if(nextChunkOrderRun-- <= 0) { orderChunks(); } }
        if self.chunk_order_countdown != u32::MAX {
            if self.chunk_order_countdown == 0 {
                self.order_chunks();
                self.chunk_order_countdown = u32::MAX; // idle until next trigger

                // Send NetworkChunkPublisherUpdate when there are chunks to load/unload
                if !self.chunk_load_queue.is_empty() {
                    let ncpu = NetworkChunkPublisherUpdate {
                        position: [
                            self.position[0] as i32,
                            self.position[1] as i32,
                            self.position[2] as i32,
                        ],
                        radius: (self.view_distance * 16) as u32,
                    };
                    let mut responses = vec![self.encode_compressed_packet(
                        packet_id::NETWORK_CHUNK_PUBLISHER_UPDATE,
                        &ncpu.encode(),
                    )];
                    // Send chunks in same batch
                    responses.extend(self.send_chunk_batch());
                    return responses;
                }
            } else {
                self.chunk_order_countdown -= 1;
            }
        }

        // Still send queued chunks even if no reorder happened
        self.send_chunk_batch()
    }

    /// Send up to 8 chunks from the load queue.
    fn send_chunk_batch(&mut self) -> Vec<Vec<u8>> {
        const CHUNKS_PER_TICK: usize = 8;
        let mut responses = Vec::new();
        let mut sent = 0;

        while sent < CHUNKS_PER_TICK {
            let Some((cx, cz)) = self.chunk_load_queue.pop_front() else {
                break;
            };
            if self.sent_chunks.contains(&(cx, cz)) {
                continue;
            }

            let (sub_count, payload) = {
                let mut cache = self.chunk_cache.lock().unwrap();
                let col = cache.get_chunk_mut(cx, cz);
                (col.sub_chunk_count, col.get_network_payload().to_vec())
            };

            let chunk_pkt = LevelChunk {
                chunk_x: cx,
                chunk_z: cz,
                dimension_id: 0,
                sub_chunk_count: sub_count,
                cache_enabled: false,
                payload,
            };
            responses
                .push(self.encode_compressed_packet(packet_id::LEVEL_CHUNK, &chunk_pkt.encode()));
            self.sent_chunks.insert((cx, cz));
            sent += 1;
        }

        responses
    }

    /// Handle a raw game packet (0xFE batch) from RakNet.
    /// Returns a list of response batches to send back.
    pub fn handle_raw_batch(&mut self, raw: &[u8]) -> Vec<Vec<u8>> {
        if raw.is_empty() || raw[0] != 0xFE {
            warn!("[{}] Invalid batch: missing 0xFE header", self.addr);
            return Vec::new();
        }

        let payload = &raw[1..];

        // Decrypt if encryption is active
        let decrypted = if let Some(ref mut ctx) = self.encryption {
            match ctx.decrypt(payload) {
                Ok(data) => data,
                Err(e) => {
                    warn!("[{}] Decryption failed: {}", self.addr, e);
                    return Vec::new();
                }
            }
        } else {
            payload.to_vec()
        };

        // Determine compression
        let (algo_byte, compressed_data) = if decrypted.is_empty() {
            return Vec::new();
        } else if self.state == ConnectionState::SessionStart {
            // First packet (RequestNetworkSettings) has no algo byte
            // It's sent raw (no compression) before NetworkSettings response
            (CompressionAlgorithm::None, &decrypted[..])
        } else {
            let algo =
                CompressionAlgorithm::from_u8(decrypted[0]).unwrap_or(CompressionAlgorithm::None);
            (algo, &decrypted[1..])
        };

        // Decode batch
        let packets = match batch::decode_batch(compressed_data, algo_byte) {
            Ok(p) => p,
            Err(e) => {
                warn!("[{}] Batch decode failed: {}", self.addr, e);
                return Vec::new();
            }
        };

        // Process each packet
        let mut responses = Vec::new();
        for pkt_data in packets {
            let mut reader = ProtoReader::new(&pkt_data);
            let Ok((pkt_id, _, _)) = codec::decode_packet_header(&mut reader) else {
                continue;
            };

            let response_packets = self.handle_packet(pkt_id, &mut reader);
            for resp in response_packets {
                responses.push(resp);
            }
        }

        responses
    }

    /// Handle a single decoded packet. Returns response packets to send.
    fn handle_packet(&mut self, pkt_id: u32, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        match (self.state, pkt_id) {
            // ── SessionStart ──
            (ConnectionState::SessionStart, packet_id::REQUEST_NETWORK_SETTINGS) => {
                self.handle_request_network_settings(reader)
            }

            // ── Login ──
            (ConnectionState::Login, packet_id::LOGIN) => self.handle_login(reader),

            // ── Handshake ──
            (ConnectionState::Handshake, packet_id::CLIENT_TO_SERVER_HANDSHAKE) => {
                self.handle_client_to_server_handshake(reader)
            }

            // ── ResourcePacks ──
            (ConnectionState::ResourcePacks, packet_id::RESOURCE_PACK_CLIENT_RESPONSE) => {
                self.handle_resource_pack_client_response(reader)
            }

            // ── PreSpawn ──
            (ConnectionState::PreSpawn, packet_id::REQUEST_CHUNK_RADIUS) => {
                self.handle_request_chunk_radius(reader)
            }

            // Silently ignore these in PreSpawn
            (ConnectionState::PreSpawn, packet_id::PLAYER_AUTH_INPUT)
            | (ConnectionState::PreSpawn, packet_id::SERVERBOUND_LOADING_SCREEN) => Vec::new(),

            // ── SpawnResponse ──
            (ConnectionState::SpawnResponse, packet_id::SET_LOCAL_PLAYER_AS_INITIALIZED) => {
                self.handle_set_local_player_as_initialized()
            }

            // ── Silently ignored packets ──
            // ── InGame ──
            (ConnectionState::InGame, packet_id::PLAYER_AUTH_INPUT) => {
                self.handle_player_auth_input(reader)
            }
            (ConnectionState::InGame, packet_id::INTERACT) => self.handle_interact(reader),
            (ConnectionState::InGame, packet_id::CONTAINER_CLOSE) => {
                self.handle_container_close(reader)
            }
            (ConnectionState::InGame, packet_id::MOB_EQUIPMENT) => {
                self.handle_mob_equipment(reader)
            }
            (ConnectionState::InGame, packet_id::TEXT) => self.handle_text(reader),
            (ConnectionState::InGame, packet_id::COMMAND_REQUEST) => {
                self.handle_command_request(reader)
            }

            // ── Silently ignored ──
            (_, packet_id::EMOTE_LIST)
            | (_, packet_id::SERVERBOUND_LOADING_SCREEN)
            | (ConnectionState::SpawnResponse, packet_id::PLAYER_AUTH_INPUT)
            | (_, 0x081) => Vec::new(),

            _ => {
                debug!(
                    "[{}] Unhandled packet 0x{:03X} in state {:?}",
                    self.addr, pkt_id, self.state
                );
                Vec::new()
            }
        }
    }

    // ── Packet handlers ──

    fn handle_request_network_settings(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(pkt) = RequestNetworkSettings::decode(reader) else {
            return Vec::new();
        };

        info!(
            "[{}] RequestNetworkSettings: protocol={}",
            self.addr, pkt.protocol_version
        );

        if pkt.protocol_version != 924 {
            warn!(
                "[{}] Incompatible protocol: {} (expected 924)",
                self.addr, pkt.protocol_version
            );
            let disconnect = Disconnect {
                reason: DisconnectReason::Unknown,
                message: Some("Incompatible protocol version".to_string()),
            };
            return vec![self.encode_raw_packet(packet_id::DISCONNECT, &disconnect.encode())];
        }

        let settings = NetworkSettings::default_settings();
        let response = self.encode_raw_packet(packet_id::NETWORK_SETTINGS, &settings.encode());

        self.state = ConnectionState::Login;
        debug!("[{}] → Login state", self.addr);

        vec![response]
    }

    fn handle_login(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(pkt) = Login::decode(reader) else {
            warn!("[{}] Failed to decode Login packet", self.addr);
            return Vec::new();
        };

        info!("[{}] Login: protocol={}", self.addr, pkt.protocol_version);

        // Parse authInfoJson to extract identity and public key
        match jwt::extract_login_identity(&pkt.chain_data) {
            Ok(identity) => {
                self.client_pub_key_b64 = Some(identity.public_key_b64);
                self.display_name = Some(identity.display_name);
                self.xuid = if identity.xuid.is_empty() {
                    None
                } else {
                    Some(identity.xuid)
                };
                if !identity.uuid_str.is_empty() {
                    self.uuid = uuid::Uuid::parse_str(&identity.uuid_str).ok();
                }
                // Load saved player data if exists
                if let Some(ref xuid) = self.xuid {
                    if let Some(save) = player_data::load_player(xuid) {
                        self.position = [
                            save.position[0] as f32,
                            save.position[1] as f32,
                            save.position[2] as f32,
                        ];
                        self.yaw = save.rotation[0];
                        self.pitch = save.rotation[1];
                        self.gamemode = save.gamemode;
                        info!(
                            "[{}] Restored position: {:.1}, {:.1}, {:.1} (gamemode={})",
                            self.addr,
                            self.position[0],
                            self.position[1],
                            self.position[2],
                            self.gamemode
                        );
                    }
                }

                info!(
                    "[{}] Player: {} (xuid={}, auth={})",
                    self.addr,
                    self.display_name.as_deref().unwrap_or("?"),
                    self.xuid.as_deref().unwrap_or("none"),
                    if identity.authenticated {
                        "xbox"
                    } else {
                        "offline"
                    },
                );
            }
            Err(e) => {
                warn!("[{}] Login identity parse failed: {}", self.addr, e);
                // Fallback: try to get the key from the client data JWT header
                if !pkt.client_data_jwt.is_empty() {
                    if let Ok(decoded) = jwt::decode_jwt(&pkt.client_data_jwt) {
                        if let Some(key) = decoded.header.get("x5u").and_then(|v| v.as_str()) {
                            debug!("[{}] Got client key from client_data JWT x5u", self.addr);
                            self.client_pub_key_b64 = Some(key.to_string());
                        }
                    }
                }
            }
        }

        // Set up encryption
        let Some(ref client_pub_b64) = self.client_pub_key_b64 else {
            warn!("[{}] No client public key found", self.addr);
            return Vec::new();
        };

        let Ok(client_pub_key) = ecdh::parse_client_public_key(client_pub_b64) else {
            warn!("[{}] Failed to parse client public key", self.addr);
            return Vec::new();
        };

        // Generate salt
        let mut salt = [0u8; 16];
        rand::Rng::fill(&mut rand::thread_rng(), &mut salt);

        // Derive AES key
        let aes_key = self.server_keypair.derive_aes_key(&client_pub_key, &salt);

        // Create handshake JWT
        let server_pub_b64 = self.server_keypair.public_key_base64();
        let salt_b64 = base64::engine::general_purpose::STANDARD.encode(salt);
        let keypair = self.server_keypair.clone();
        let handshake_jwt =
            jwt::create_handshake_jwt(&server_pub_b64, &salt_b64, |data| keypair.sign(data));

        let handshake_pkt = ServerToClientHandshake { jwt: handshake_jwt };
        let response = self.encode_compressed_packet(
            packet_id::SERVER_TO_CLIENT_HANDSHAKE,
            &handshake_pkt.encode(),
        );

        // DON'T enable encryption yet — the ServerToClientHandshake must be sent unencrypted.
        // Store the key to activate AFTER this packet is sent.
        self.pending_encryption_key = Some(aes_key);

        self.state = ConnectionState::Handshake;
        debug!("[{}] → Handshake state (encryption pending)", self.addr);

        vec![response]
    }

    fn handle_client_to_server_handshake(&mut self, _reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        info!(
            "[{}] ClientToServerHandshake received — encryption verified",
            self.addr
        );

        // Send PlayStatus(LOGIN_SUCCESS)
        let play_status = PlayStatus {
            status: PlayStatusType::LoginSuccess,
        };
        let response = self.encode_compressed_packet(packet_id::PLAY_STATUS, &play_status.encode());

        self.state = ConnectionState::ResourcePacks;
        debug!("[{}] → ResourcePacks state", self.addr);

        // Also send ResourcePacksInfo immediately
        let mut responses = vec![response];
        responses.extend(self.send_resource_packs_info());
        responses
    }

    fn handle_resource_pack_client_response(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(status) = reader.read_u8() else {
            return Vec::new();
        };

        debug!(
            "[{}] ResourcePackClientResponse: status={}",
            self.addr, status
        );

        match status {
            3 => {
                // HAVE_ALL_PACKS → send ResourcePackStack
                self.send_resource_pack_stack()
            }
            4 => {
                // COMPLETED → transition to PreSpawn
                info!("[{}] Resource packs completed", self.addr);
                self.state = ConnectionState::PreSpawn;
                debug!("[{}] → PreSpawn state", self.addr);
                self.send_pre_spawn_packets()
            }
            _ => {
                debug!(
                    "[{}] Unexpected resource pack status: {}",
                    self.addr, status
                );
                Vec::new()
            }
        }
    }

    fn handle_request_chunk_radius(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let radius = reader.read_var_i32().unwrap_or(4);
        let clamped = radius.clamp(2, self.config.max_view_distance);
        self.view_distance = clamped;
        info!(
            "[{}] RequestChunkRadius: {} (responding with {})",
            self.addr, radius, clamped
        );

        let mut responses = Vec::new();

        // ChunkRadiusUpdated
        let radius_pkt = ChunkRadiusUpdated { radius: clamped };
        responses.push(
            self.encode_compressed_packet(packet_id::CHUNK_RADIUS_UPDATED, &radius_pkt.encode()),
        );

        // NetworkChunkPublisherUpdate
        let spawn_x = self.position[0] as i32;
        let spawn_y = self.position[1] as i32;
        let spawn_z = self.position[2] as i32;
        let publisher = NetworkChunkPublisherUpdate {
            position: [spawn_x, spawn_y, spawn_z],
            radius: (clamped * 16) as u32,
        };
        responses.push(self.encode_compressed_packet(
            packet_id::NETWORK_CHUNK_PUBLISHER_UPDATE,
            &publisher.encode(),
        ));

        // Send initial chunks around spawn using spiral order (nearest first)
        let spawn_chunk_x = spawn_x >> 4;
        let spawn_chunk_z = spawn_z >> 4;
        self.last_chunk_x = spawn_chunk_x;
        self.last_chunk_z = spawn_chunk_z;

        // Build spiral-ordered list of spawn chunks
        let mut spawn_chunks: Vec<(i32, i32, i32)> = Vec::new();
        for dx in -clamped..=clamped {
            for dz in -clamped..=clamped {
                let dist_sq = dx * dx + dz * dz;
                if dist_sq <= clamped * clamped {
                    spawn_chunks.push((spawn_chunk_x + dx, spawn_chunk_z + dz, dist_sq));
                }
            }
        }
        spawn_chunks.sort_by_key(|&(_, _, d)| d);

        for (cx, cz, _) in &spawn_chunks {
            let (sub_chunk_count, chunk_payload) = {
                let mut cache = self.chunk_cache.lock().unwrap();
                let col = cache.get_chunk_mut(*cx, *cz);
                (col.sub_chunk_count, col.get_network_payload().to_vec())
            };
            let chunk = LevelChunk {
                chunk_x: *cx,
                chunk_z: *cz,
                dimension_id: 0,
                sub_chunk_count,
                cache_enabled: false,
                payload: chunk_payload,
            };
            responses.push(self.encode_compressed_packet(packet_id::LEVEL_CHUNK, &chunk.encode()));
            self.sent_chunks.insert((*cx, *cz));
        }
        info!(
            "[{}] Sent {} chunks (radius={})",
            self.addr,
            self.sent_chunks.len(),
            clamped
        );

        // PLAYER_SPAWN — send after chunks
        let spawn_status = PlayStatus {
            status: PlayStatusType::PlayerSpawn,
        };
        responses
            .push(self.encode_compressed_packet(packet_id::PLAY_STATUS, &spawn_status.encode()));
        self.state = ConnectionState::SpawnResponse;
        debug!("[{}] → SpawnResponse state", self.addr);

        responses
    }

    fn send_player_spawn(&mut self) -> Vec<Vec<u8>> {
        if self.state != ConnectionState::PreSpawn {
            return Vec::new();
        }
        info!("[{}] Sending PlayStatus(PLAYER_SPAWN)", self.addr);
        let spawn_status = PlayStatus {
            status: PlayStatusType::PlayerSpawn,
        };
        let response =
            self.encode_compressed_packet(packet_id::PLAY_STATUS, &spawn_status.encode());
        self.state = ConnectionState::SpawnResponse;
        debug!("[{}] → SpawnResponse state", self.addr);
        vec![response]
    }

    fn handle_set_local_player_as_initialized(&mut self) -> Vec<Vec<u8>> {
        info!(
            "[{}] {} is now in-game!",
            self.addr,
            self.display_name.as_deref().unwrap_or("Player")
        );
        self.state = ConnectionState::InGame;

        // Gravity works from PreSpawn (correct bit 49). No extra packets needed here.
        // Sending a second SetActorData after init breaks skin rendering.
        vec![]
    }

    // ── InGame handlers ──

    fn handle_player_auth_input(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(pkt) = mc_rs_proto::packets::player::PlayerAuthInput::decode(reader) else {
            return Vec::new();
        };

        // Validate position (anti-cheat basics)
        if !pkt.position[0].is_finite()
            || !pkt.position[1].is_finite()
            || !pkt.position[2].is_finite()
        {
            return Vec::new(); // ignore invalid position
        }

        // Void kill check
        if pkt.position[1] < -128.0 {
            let surface = terrain_generator::get_surface_height(0, 0, 42) as f32;
            self.position = [0.5, surface + 2.621, 0.5];
            let reset = mc_rs_proto::packets::player::MovePlayer {
                runtime_entity_id: self.entity_runtime_id,
                position: self.position,
                pitch: 0.0,
                yaw: 0.0,
                head_yaw: 0.0,
                mode: 1, // reset
                on_ground: true,
                riding_runtime_id: 0,
                tick: self.tick,
            };
            return vec![self.encode_compressed_packet(packet_id::MOVE_PLAYER, &reset.encode())];
        }

        // Anti-fly: if player is moving UP by more than 1.5 blocks per tick,
        // clamp their Y to prevent fly hacking
        let dy = pkt.position[1] - self.position[1];
        if dy > 1.5 {
            // Player is rising too fast — clamp Y to max jump height
            let clamped_y = self.position[1] + 1.5;
            self.position = [pkt.position[0], clamped_y, pkt.position[2]];
            // Send correction
            let reset = mc_rs_proto::packets::player::MovePlayer {
                runtime_entity_id: self.entity_runtime_id,
                position: self.position,
                pitch: pkt.pitch,
                yaw: pkt.yaw,
                head_yaw: pkt.head_yaw,
                mode: 1, // reset
                on_ground: false,
                riding_runtime_id: 0,
                tick: self.tick,
            };
            return vec![self.encode_compressed_packet(packet_id::MOVE_PLAYER, &reset.encode())];
        }

        // Update player position
        self.position = pkt.position;
        self.pitch = pkt.pitch;
        self.yaw = pkt.yaw;
        self.head_yaw = pkt.head_yaw;
        self.tick += 1;

        // Broadcast MovePlayer to all other players
        let move_pkt = mc_rs_proto::packets::player::MovePlayer {
            runtime_entity_id: self.entity_runtime_id,
            position: self.position,
            pitch: self.pitch,
            yaw: self.yaw,
            head_yaw: self.head_yaw,
            mode: 0, // normal
            on_ground: true,
            riding_runtime_id: 0,
            tick: self.tick,
        };
        self.broadcasts
            .push(self.encode_compressed_packet(packet_id::MOVE_PLAYER, &move_pkt.encode()));

        // Check if player moved to a new chunk — queue chunks for tick-based sending
        let mut responses = Vec::new();
        let chunk_x = (self.position[0] as i32) >> 4;
        let chunk_z = (self.position[2] as i32) >> 4;

        if chunk_x != self.last_chunk_x || chunk_z != self.last_chunk_z {
            self.last_chunk_x = chunk_x;
            self.last_chunk_z = chunk_z;

            // PMMP: nextChunkOrderRun = 0 on chunk change
            self.chunk_order_countdown = 0;
        } else {
            // PMMP: nextChunkOrderRun = min(current, 20) on normal movement
            if self.chunk_order_countdown > 20 {
                self.chunk_order_countdown = 20;
            }
        }

        // Handle block actions (breaking/placing)
        for action in &pkt.block_actions {
            let bx = action.position[0];
            let by = action.position[1];
            let bz = action.position[2];
            let block_center = [bx as f32 + 0.5, by as f32 + 0.5, bz as f32 + 0.5];

            match action.action_type {
                // START_BREAK (0) or CONTINUE_DESTROY_BLOCK (27)
                0 | 27 => {
                    // Calculate break speed — simplified: 1.0 / (break_time_seconds * 20)
                    // Default hardness for most blocks: ~1.5s with hand = 30 ticks
                    let break_speed: f32 = {
                        let block_id = if let Ok(mut cache) = self.chunk_cache.lock() {
                            cache.get_block(bx, by, bz)
                        } else {
                            0
                        };
                        // Simple break speed based on block type
                        match block_id {
                            13079 => 0.0,         // bedrock — unbreakable
                            12421 | 11669 => 1.0, // short grass/tall grass — instant
                            _ => 1.0 / 30.0,      // default ~1.5s with hand
                        }
                    };

                    let event = LevelEvent {
                        event_id: LevelEvent::BLOCK_START_BREAK,
                        position: block_center,
                        event_data: (65535.0 * break_speed) as i32,
                    };
                    let event_bytes =
                        self.encode_compressed_packet(packet_id::LEVEL_EVENT, &event.encode());
                    responses.push(event_bytes.clone());
                    self.broadcasts.push(event_bytes);
                }

                // ABORT_BREAK (1) or STOP_BREAK (2)
                1 | 2 => {
                    let event = LevelEvent {
                        event_id: LevelEvent::BLOCK_STOP_BREAK,
                        position: block_center,
                        event_data: 0,
                    };
                    let event_bytes =
                        self.encode_compressed_packet(packet_id::LEVEL_EVENT, &event.encode());
                    responses.push(event_bytes.clone());
                    self.broadcasts.push(event_bytes);
                }

                // PREDICT_DESTROY_BLOCK (26)
                26 => {
                    let air_id = flat_generator::block_ids::AIR;

                    // Send BLOCK_STOP_BREAK to clear crack animation
                    let stop_event = LevelEvent {
                        event_id: LevelEvent::BLOCK_STOP_BREAK,
                        position: block_center,
                        event_data: 0,
                    };
                    let stop_bytes =
                        self.encode_compressed_packet(packet_id::LEVEL_EVENT, &stop_event.encode());
                    responses.push(stop_bytes.clone());
                    self.broadcasts.push(stop_bytes);

                    // Get the old block ID and set to air
                    let old_block_id = if let Ok(mut cache) = self.chunk_cache.lock() {
                        let old = cache.get_block(bx, by, bz);
                        cache.set_block(bx, by, bz, air_id);
                        old
                    } else {
                        air_id
                    };

                    // Send UpdateBlock
                    let update = UpdateBlock {
                        position: action.position,
                        runtime_id: air_id,
                        flags: 3, // FLAG_NEIGHBORS | FLAG_NETWORK
                        layer: 0,
                    };
                    let update_bytes =
                        self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &update.encode());
                    responses.push(update_bytes.clone());
                    self.broadcasts.push(update_bytes);

                    // Send block destroy particles + sound
                    if old_block_id != air_id {
                        let level_event = LevelEvent {
                            event_id: LevelEvent::PARTICLE_DESTROY,
                            position: block_center,
                            event_data: old_block_id as i32,
                        };
                        let event_bytes = self.encode_compressed_packet(
                            packet_id::LEVEL_EVENT,
                            &level_event.encode(),
                        );
                        responses.push(event_bytes.clone());
                        self.broadcasts.push(event_bytes);

                        let sound = LevelSoundEvent::block_sound(
                            LevelSoundEvent::BREAK,
                            block_center,
                            old_block_id as i32,
                        );
                        let sound_bytes = self.encode_compressed_packet(
                            packet_id::LEVEL_SOUND_EVENT,
                            &sound.encode(),
                        );
                        responses.push(sound_bytes.clone());
                        self.broadcasts.push(sound_bytes);
                    }

                    // Add block drop to inventory
                    if old_block_id != air_id {
                        if let Some(drop_item) = crate::inventory::block_drop(old_block_id) {
                            if let Some(slot) = self.inventory.add_item(drop_item) {
                                let slot_pkt = InventorySlot::encode(
                                    0,
                                    slot as u32,
                                    &self.inventory.slots[slot],
                                    0,
                                );
                                responses.push(self.encode_compressed_packet(
                                    packet_id::INVENTORY_SLOT,
                                    &slot_pkt,
                                ));
                            }
                        }
                    }

                    info!(
                        "[{}] Block broken at ({}, {}, {}) old_id={}",
                        self.addr, bx, by, bz, old_block_id
                    );
                }

                _ => {}
            }
        }

        // Handle block placement (item interaction with ACTION_CLICK_BLOCK)
        if let Some(ref interaction) = pkt.item_interaction {
            if interaction.action_type == 0 {
                // ACTION_CLICK_BLOCK
                self.handle_block_place(interaction, &mut responses);
            }
        }

        // Handle inventory stack requests (slot movements)
        if let Some(ref request) = pkt.item_stack_request {
            self.handle_item_stack_request(request, &mut responses);
        }

        responses
    }

    fn handle_block_place(
        &mut self,
        interaction: &mc_rs_proto::packets::player::ItemInteractionData,
        responses: &mut Vec<Vec<u8>>,
    ) {
        let held = &self.inventory.slots[self.inventory.held_slot as usize];
        if held.item.is_air() {
            return;
        }

        // Check if held item is a placeable block
        let block_runtime_id = match crate::inventory::item_to_block(held.item.id) {
            Some(id) => id,
            None => return, // Not a block item
        };

        // Calculate target position from face offset
        let (dx, dy, dz) = match interaction.face {
            0 => (0, -1, 0), // down
            1 => (0, 1, 0),  // up
            2 => (0, 0, -1), // north
            3 => (0, 0, 1),  // south
            4 => (-1, 0, 0), // west
            5 => (1, 0, 0),  // east
            _ => return,
        };

        let tx = interaction.block_position[0] + dx;
        let ty = interaction.block_position[1] + dy;
        let tz = interaction.block_position[2] + dz;

        // Set the block
        if let Ok(mut cache) = self.chunk_cache.lock() {
            let existing = cache.get_block(tx, ty, tz);
            if existing != flat_generator::block_ids::AIR {
                return; // Can't place on a non-air block
            }
            cache.set_block(tx, ty, tz, block_runtime_id);
        }

        // Send UpdateBlock
        let update = UpdateBlock {
            position: [tx, ty, tz],
            runtime_id: block_runtime_id,
            flags: 3,
            layer: 0,
        };
        let update_bytes = self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &update.encode());
        responses.push(update_bytes.clone());
        self.broadcasts.push(update_bytes);

        // Send place sound
        let block_center = [tx as f32 + 0.5, ty as f32 + 0.5, tz as f32 + 0.5];
        let sound = LevelSoundEvent::block_sound(
            LevelSoundEvent::PLACE,
            block_center,
            block_runtime_id as i32,
        );
        let sound_bytes =
            self.encode_compressed_packet(packet_id::LEVEL_SOUND_EVENT, &sound.encode());
        responses.push(sound_bytes.clone());
        self.broadcasts.push(sound_bytes);

        // Decrement item count in inventory
        let slot = self.inventory.held_slot as usize;
        if self.inventory.slots[slot].item.count > 1 {
            self.inventory.slots[slot].item.count -= 1;
        } else {
            self.inventory.slots[slot] = ItemStackWrapper::air();
        }

        // Send inventory slot update
        let slot_pkt = InventorySlot::encode(0, slot as u32, &self.inventory.slots[slot], 0);
        responses.push(self.encode_compressed_packet(packet_id::INVENTORY_SLOT, &slot_pkt));

        info!(
            "[{}] Block placed at ({}, {}, {}) block_id={}",
            self.addr, tx, ty, tz, block_runtime_id
        );
    }

    fn handle_item_stack_request(
        &mut self,
        request: &mc_rs_proto::packets::player::ItemStackRequest,
        responses: &mut Vec<Vec<u8>>,
    ) {
        use mc_rs_proto::packets::player::StackRequestAction;
        use mc_rs_proto::packets::world::{ItemStackResponse, ItemStackResponseContainer};

        let mut changed_containers: Vec<ItemStackResponseContainer> = Vec::new();

        for action in &request.actions {
            match action {
                StackRequestAction::Take {
                    count,
                    source,
                    destination,
                }
                | StackRequestAction::Place {
                    count,
                    source,
                    destination,
                } => {
                    let src_slot = self.resolve_slot(source.container_id, source.slot_id);
                    let dst_slot = self.resolve_slot(destination.container_id, destination.slot_id);

                    if let (Some(src_idx), Some(dst_idx)) = (src_slot, dst_slot) {
                        let take_count = *count;

                        // Take from source
                        let src_item = self.inventory.slots[src_idx].item.clone();
                        if src_item.is_air() || src_item.count < take_count as u16 {
                            continue;
                        }

                        // Place to destination
                        let dst_item = &self.inventory.slots[dst_idx].item;
                        if dst_item.is_air() {
                            // Move to empty slot
                            let mut new_item = src_item.clone();
                            new_item.count = take_count as u16;
                            let stack_id = self.inventory.next_stack_id();
                            self.inventory.slots[dst_idx] =
                                ItemStackWrapper::new(new_item, stack_id);
                        } else if dst_item.id == src_item.id && dst_item.meta == src_item.meta {
                            // Stack on same item
                            self.inventory.slots[dst_idx].item.count += take_count as u16;
                        } else {
                            continue; // Can't place here
                        }

                        // Reduce source
                        if self.inventory.slots[src_idx].item.count <= take_count as u16 {
                            self.inventory.slots[src_idx] = ItemStackWrapper::air();
                        } else {
                            self.inventory.slots[src_idx].item.count -= take_count as u16;
                        }

                        // Track changes for response
                        self.add_slot_to_response(
                            &mut changed_containers,
                            source.container_id,
                            source.slot_id,
                            src_idx,
                        );
                        self.add_slot_to_response(
                            &mut changed_containers,
                            destination.container_id,
                            destination.slot_id,
                            dst_idx,
                        );
                    }
                }
                StackRequestAction::Swap {
                    source,
                    destination,
                    ..
                } => {
                    let src_slot = self.resolve_slot(source.container_id, source.slot_id);
                    let dst_slot = self.resolve_slot(destination.container_id, destination.slot_id);

                    if let (Some(src_idx), Some(dst_idx)) = (src_slot, dst_slot) {
                        self.inventory.slots.swap(src_idx, dst_idx);

                        self.add_slot_to_response(
                            &mut changed_containers,
                            source.container_id,
                            source.slot_id,
                            src_idx,
                        );
                        self.add_slot_to_response(
                            &mut changed_containers,
                            destination.container_id,
                            destination.slot_id,
                            dst_idx,
                        );
                    }
                }
                StackRequestAction::Destroy { source, .. }
                | StackRequestAction::Drop { source, .. } => {
                    if let Some(slot_idx) = self.resolve_slot(source.container_id, source.slot_id) {
                        self.inventory.slots[slot_idx] = ItemStackWrapper::air();
                        self.add_slot_to_response(
                            &mut changed_containers,
                            source.container_id,
                            source.slot_id,
                            slot_idx,
                        );
                    }
                }
                StackRequestAction::Unknown(_) => {}
            }
        }

        // Send response
        let response = ItemStackResponse::ok(request.request_id, changed_containers);
        responses.push(
            self.encode_compressed_packet(packet_id::ITEM_STACK_RESPONSE, &response.encode()),
        );
    }

    /// Resolve a container_id + slot_id to an index in self.inventory.slots.
    fn resolve_slot(&self, container_id: u8, slot_id: u8) -> Option<usize> {
        match container_id {
            0 | 28 => {
                // Inventory / hotbar (container 0 or 28 for hotbar)
                let idx = slot_id as usize;
                if idx < 36 {
                    Some(idx)
                } else {
                    None
                }
            }
            _ => None, // Armor, offhand, etc. not handled yet
        }
    }

    /// Add a slot to the response containers.
    fn add_slot_to_response(
        &self,
        containers: &mut Vec<mc_rs_proto::packets::world::ItemStackResponseContainer>,
        container_id: u8,
        slot_id: u8,
        inventory_idx: usize,
    ) {
        use mc_rs_proto::packets::world::{ItemStackResponseContainer, ItemStackResponseSlot};

        let item = &self.inventory.slots[inventory_idx];
        let response_slot = ItemStackResponseSlot {
            slot: slot_id,
            hotbar_slot: slot_id,
            count: item.item.count as u8,
            stack_id: item.stack_id,
            custom_name: String::new(),
            filtered_custom_name: String::new(),
            durability_correction: 0,
        };

        // Find or create the container
        if let Some(container) = containers
            .iter_mut()
            .find(|c| c.container_id == container_id)
        {
            container.slots.push(response_slot);
        } else {
            containers.push(ItemStackResponseContainer {
                container_id,
                slots: vec![response_slot],
            });
        }
    }

    fn handle_interact(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(action) = reader.read_u8() else {
            return Vec::new();
        };
        let _actor_runtime_id = reader.read_var_u64().unwrap_or(0);

        info!("[{}] InteractPacket action={}", self.addr, action);

        if action == 6 {
            // OPEN_INVENTORY — use window_id=1 (not 0, which is HARDCODED for content sync)
            let container_open = ContainerOpen {
                window_id: 1,
                window_type: 0xFF, // WindowTypes::INVENTORY = -1
                position: [0, 0, 0],
                actor_unique_id: self.entity_runtime_id as i64,
            };
            let pkt =
                self.encode_compressed_packet(packet_id::CONTAINER_OPEN, &container_open.encode());
            info!("[{}] Opening player inventory (window_id=1)", self.addr);
            return vec![pkt];
        }

        Vec::new()
    }

    fn handle_container_close(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let window_id = reader.read_u8().unwrap_or(0);
        let window_type = reader.read_u8().unwrap_or(0);
        let _server = reader.read_bool().unwrap_or(false);

        info!(
            "[{}] ContainerClose window_id={} window_type={}",
            self.addr, window_id, window_type
        );

        // Echo back the close
        let close = ContainerClose {
            window_id,
            window_type,
            server: true,
        };
        vec![self.encode_compressed_packet(packet_id::CONTAINER_CLOSE, &close.encode())]
    }

    fn handle_mob_equipment(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let _runtime_entity_id = reader.read_var_u64().unwrap_or(0);
        // Skip the ItemStackWrapper (just skip the item ID to determine air)
        let _item_id = reader.read_var_i32().unwrap_or(0);
        // For non-air items, skip the rest — but we only need the hotbar slot
        // The remaining bytes are: count(u16), meta(VarU32), hasNetId, stackId?, blockRuntimeId, extraData
        // Then: inventory_slot(u8), hotbar_slot(u8), container_id(u8)
        // For simplicity, read remaining bytes and get the last 3
        let remaining = reader.read_remaining();
        if remaining.len() >= 3 {
            let hotbar_slot = remaining[remaining.len() - 2];
            if hotbar_slot < 9 {
                self.inventory.held_slot = hotbar_slot;
                debug!("[{}] Held slot changed to {}", self.addr, hotbar_slot);
            }
        }

        Vec::new()
    }

    fn handle_text(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(pkt) = mc_rs_proto::packets::player::Text::decode(reader) else {
            return Vec::new();
        };

        let player_name = self
            .display_name
            .clone()
            .unwrap_or_else(|| "Player".to_string());
        let xuid = self.xuid.clone().unwrap_or_default();

        // Check for commands (in case client sends via Text instead of CommandRequest)
        if pkt.message.starts_with('/') {
            let ctx = mc_rs_command::CommandContext {
                player_name: player_name.clone(),
                position: self.position,
            };
            let registry = mc_rs_command::CommandRegistry::new();
            let result = registry.execute(&pkt.message, &ctx);
            if let Some(ref response) = result.response {
                let msg = mc_rs_proto::packets::player::Text::system(response);
                return vec![self.encode_compressed_packet(packet_id::TEXT, &msg)];
            }
            return Vec::new();
        }

        info!("[CHAT] {}: {}", player_name, pkt.message);

        // Broadcast chat to all players (including self)
        let chat = mc_rs_proto::packets::player::Text::chat(&player_name, &pkt.message, &xuid);
        self.broadcasts
            .push(self.encode_compressed_packet(packet_id::TEXT, &chat));

        Vec::new()
    }

    fn handle_command_request(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(command) = reader.read_string() else {
            return Vec::new();
        };

        let ctx = mc_rs_command::CommandContext {
            player_name: self
                .display_name
                .clone()
                .unwrap_or_else(|| "Player".to_string()),
            position: self.position,
        };

        let registry = mc_rs_command::CommandRegistry::new();
        let result = registry.execute(&command, &ctx);

        let mut responses = Vec::new();

        // Handle action
        match &result.action {
            mc_rs_command::CommandAction::Teleport { x, y, z } => {
                self.position = [*x, *y, *z];
                let move_pkt = mc_rs_proto::packets::player::MovePlayer {
                    runtime_entity_id: self.entity_runtime_id,
                    position: self.position,
                    pitch: self.pitch,
                    yaw: self.yaw,
                    head_yaw: self.head_yaw,
                    mode: 2,
                    on_ground: true,
                    riding_runtime_id: 0,
                    tick: self.tick,
                };
                responses.push(
                    self.encode_compressed_packet(packet_id::MOVE_PLAYER, &move_pkt.encode()),
                );
            }
            mc_rs_command::CommandAction::Broadcast { message } => {
                let chat = mc_rs_proto::packets::player::Text::chat("Server", message, "");
                self.broadcasts
                    .push(self.encode_compressed_packet(packet_id::TEXT, &chat));
            }
            mc_rs_command::CommandAction::SetGamemode { mode } => {
                let pkts = self.apply_gamemode(*mode);
                responses.extend(pkts);
            }
            mc_rs_command::CommandAction::SetTime { .. }
            | mc_rs_command::CommandAction::SetWeather { .. }
            | mc_rs_command::CommandAction::Stop
            | mc_rs_command::CommandAction::Kill => {
                self.pending_actions.push(result.action);
            }
            mc_rs_command::CommandAction::None => {}
        }

        // Send text response
        if let Some(ref response) = result.response {
            let msg = mc_rs_proto::packets::player::Text::system(response);
            responses.push(self.encode_compressed_packet(packet_id::TEXT, &msg));
        }

        responses
    }

    /// Change the player's gamemode (PMMP syncGameMode flow).
    /// Sends: SetPlayerGameType + UpdateAbilities + UpdateAdventureSettings + SetActorData
    /// For spectator: broadcasts RemoveEntity to other players.
    /// When leaving spectator: broadcasts AddPlayer to other players.
    fn apply_gamemode(&mut self, mode: i32) -> Vec<Vec<u8>> {
        let old_mode = self.gamemode;
        self.gamemode = mode;
        let mut responses = Vec::new();

        // 1. SetPlayerGameType — single VarInt32
        let mut gt_writer = mc_rs_proto::io::ProtoWriter::with_capacity(4);
        gt_writer.write_var_i32(mode);
        responses.push(
            self.encode_compressed_packet(packet_id::SET_PLAYER_GAME_TYPE, gt_writer.as_bytes()),
        );

        // 2. UpdateAbilities — per-gamemode
        let abilities = match mode {
            1 => UpdateAbilities::default_creative(self.entity_runtime_id as i64),
            3 => UpdateAbilities::default_spectator(self.entity_runtime_id as i64),
            _ => UpdateAbilities::default_survival(self.entity_runtime_id as i64),
        };
        responses
            .push(self.encode_compressed_packet(packet_id::UPDATE_ABILITIES, &abilities.encode()));

        // 3. UpdateAdventureSettings
        let adventure = UpdateAdventureSettings::default_survival();
        responses.push(
            self.encode_compressed_packet(
                packet_id::UPDATE_ADVENTURE_SETTINGS,
                &adventure.encode(),
            ),
        );

        // 4. SetActorData — update collision/silent flags for spectator
        let player_name = self.display_name.clone().unwrap_or_default();
        let actor_data = if mode == 3 {
            SetActorData::player_spectator(self.entity_runtime_id, &player_name)
        } else {
            SetActorData::player_in_game(self.entity_runtime_id, &player_name)
        };
        responses
            .push(self.encode_compressed_packet(packet_id::SET_ACTOR_DATA, &actor_data.encode()));

        // 5. Broadcast despawn/respawn to other players
        if mode == 3 && old_mode != 3 {
            // Entering spectator → despawn from others
            let remove = RemoveEntity {
                entity_unique_id: self.entity_runtime_id as i64,
            }
            .encode();
            self.broadcasts
                .push(self.encode_compressed_packet(packet_id::REMOVE_ACTOR, &remove));
        } else if mode != 3 && old_mode == 3 {
            // Leaving spectator → respawn to others
            let uuid = self.uuid.map(|u| *u.as_bytes()).unwrap_or([0u8; 16]);
            let add = AddPlayer {
                uuid,
                username: player_name.clone(),
                runtime_entity_id: self.entity_runtime_id,
                platform_chat_id: String::new(),
                position: self.position,
                velocity: [0.0, 0.0, 0.0],
                pitch: self.pitch,
                yaw: self.yaw,
                head_yaw: self.head_yaw,
                gamemode: mode,
                entity_unique_id: self.entity_runtime_id as i64,
                permission_level: 1,
                command_permission: 0,
            }
            .encode();
            self.broadcasts
                .push(self.encode_compressed_packet(packet_id::ADD_PLAYER, &add));
        }

        info!(
            "[{}] Gamemode changed to {} for {}",
            self.addr,
            mode,
            self.display_name.as_deref().unwrap_or("Player")
        );

        responses
    }

    /// Take broadcast packets (to be sent to ALL other players).
    pub fn take_broadcasts(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.broadcasts)
    }

    // ── Resource pack helpers ──

    fn send_resource_packs_info(&self) -> Vec<Vec<u8>> {
        // Format from PMMP ResourcePacksInfoPacket.php (protocol 924):
        // mustAccept: bool
        // hasAddons: bool
        // hasScripts: bool
        // forceDisableVibrantVisuals: bool
        // worldTemplateId: UUID (2 x i64_le, all zeros = nil)
        // worldTemplateVersion: String
        // resourcePackCount: u16_le
        // [entries...]
        let mut writer = mc_rs_proto::io::ProtoWriter::with_capacity(64);
        writer.write_bool(false); // must_accept
        writer.write_bool(false); // has_addons
        writer.write_bool(false); // has_scripts
        writer.write_bool(false); // force_disable_vibrant_visuals
                                  // World template UUID (nil = 16 zero bytes, written as 2 x i64_le)
        writer.write_i64_le(0);
        writer.write_i64_le(0);
        writer.write_string(""); // world_template_version
        writer.write_u16_le(0); // resource_packs count

        vec![self.encode_compressed_packet(packet_id::RESOURCE_PACKS_INFO, writer.as_bytes())]
    }

    fn send_resource_pack_stack(&self) -> Vec<Vec<u8>> {
        // Format from PMMP ResourcePackStackPacket.php (protocol 924):
        // mustAccept: bool
        // resourcePackCount: VarUInt32
        // [entries...]
        // baseGameVersion: String
        // experiments: { count: u32_le, [name: String, enabled: bool]..., previously_toggled: bool }
        // useVanillaEditorPacks: bool
        let mut writer = mc_rs_proto::io::ProtoWriter::with_capacity(64);
        writer.write_bool(false); // must_accept
        writer.write_var_u32(0); // resource_pack_stack count
        writer.write_string("1.26.2"); // base_game_version
        writer.write_u32_le(0); // experiments count
        writer.write_bool(false); // experiments_previously_toggled
        writer.write_bool(false); // use_vanilla_editor_packs

        vec![self.encode_compressed_packet(packet_id::RESOURCE_PACK_STACK, writer.as_bytes())]
    }

    // ── PreSpawn placeholder (will be fully built in steps 11-14) ──

    fn send_pre_spawn_packets(&self) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();

        // StartGame
        let mut start_game =
            StartGame::default_with_id(self.entity_runtime_id as i64, self.position);
        start_game.player_gamemode = self.gamemode;
        start_game.world_gamemode = self.config.default_gamemode;
        start_game.difficulty = self.config.difficulty;
        start_game.world_name = self.config.world_name.clone();
        start_game.generator = self.config.generator_id;
        responses.push(self.encode_compressed_packet(packet_id::START_GAME, &start_game.encode()));

        // ItemRegistry (empty) — test if this crashes
        responses.push(
            self.encode_compressed_packet(packet_id::ITEM_REGISTRY, &ItemRegistry::encode_empty()),
        );

        // AvailableActorIdentifiers — real NBT from PMMP
        static ENTITY_IDENTIFIERS_NBT: &[u8] = include_bytes!("../data/entity_identifiers.nbt");
        responses.push(self.encode_compressed_packet(
            packet_id::AVAILABLE_ACTOR_IDENTIFIERS,
            ENTITY_IDENTIFIERS_NBT,
        ));

        // BiomeDefinitionList — empty (protocol 924 custom format)
        let mut biome_writer = mc_rs_proto::io::ProtoWriter::with_capacity(4);
        biome_writer.write_var_u32(0);
        biome_writer.write_var_u32(0);
        responses.push(
            self.encode_compressed_packet(
                packet_id::BIOME_DEFINITION_LIST,
                biome_writer.as_bytes(),
            ),
        );

        // 5. UpdateAttributes — health, hunger, movement speed (BEFORE abilities per PMMP)
        let attributes = UpdateAttributes::default_survival(self.entity_runtime_id);
        responses.push(
            self.encode_compressed_packet(packet_id::UPDATE_ATTRIBUTES, &attributes.encode()),
        );

        // 6. AvailableCommands with rich autocompletion (BEFORE abilities per PMMP)
        let cmd_registry = mc_rs_command::CommandRegistry::new();
        let cmd_defs = cmd_registry.all_command_defs();
        let cmd_entries: Vec<CmdEntry<'_>> = cmd_defs
            .iter()
            .map(|def| {
                let overloads = def
                    .overloads
                    .iter()
                    .map(|ov| CmdOverload {
                        params: ov
                            .params
                            .iter()
                            .map(|p| CmdParam {
                                name: p.name,
                                param_type: match &p.param_type {
                                    mc_rs_command::ParamType::HardEnum { name, values } => {
                                        CmdParamType::HardEnum {
                                            name,
                                            values: values.as_slice(),
                                        }
                                    }
                                    other => CmdParamType::Basic(other.type_id().unwrap()),
                                },
                                optional: p.optional,
                            })
                            .collect(),
                    })
                    .collect();
                CmdEntry {
                    name: def.name,
                    description: def.description,
                    aliases: def.aliases.clone(),
                    overloads,
                }
            })
            .collect();
        let commands = AvailableCommands::encode_rich(&cmd_entries);
        responses.push(self.encode_compressed_packet(packet_id::AVAILABLE_COMMANDS, &commands));

        // 7. UpdateAbilities — based on player's gamemode
        let abilities = if self.gamemode == 1 {
            UpdateAbilities::default_creative(self.entity_runtime_id as i64)
        } else {
            UpdateAbilities::default_survival(self.entity_runtime_id as i64)
        };
        responses
            .push(self.encode_compressed_packet(packet_id::UPDATE_ABILITIES, &abilities.encode()));

        // 8. UpdateAdventureSettings — PMMP sends this right after abilities
        let adventure = UpdateAdventureSettings::default_survival();
        responses.push(
            self.encode_compressed_packet(
                packet_id::UPDATE_ADVENTURE_SETTINGS,
                &adventure.encode(),
            ),
        );

        // 9. SetActorData — entity metadata (gravity, breathing, collision)
        let player_name = self.display_name.clone().unwrap_or_default();
        let actor_data = SetActorData::player_in_game(self.entity_runtime_id, &player_name);
        responses
            .push(self.encode_compressed_packet(packet_id::SET_ACTOR_DATA, &actor_data.encode()));

        // 9. Inventory sync (PMMP syncAll + syncSelectedHotbarSlot)
        // Main inventory (window 0, 36 slots)
        responses.push(self.encode_compressed_packet(
            packet_id::INVENTORY_CONTENT,
            &InventoryContent::encode_items(0, &self.inventory.slots, 0),
        ));
        // Armor inventory (window 120, 4 slots)
        responses.push(self.encode_compressed_packet(
            packet_id::INVENTORY_CONTENT,
            &InventoryContent::encode_items(120, &self.inventory.armor, 120),
        ));
        // Offhand (window 119, 1 slot)
        responses.push(self.encode_compressed_packet(
            packet_id::INVENTORY_CONTENT,
            &InventoryContent::encode_items(
                119,
                std::slice::from_ref(&self.inventory.offhand),
                119,
            ),
        ));
        // MobEquipment (selected hotbar slot)
        responses.push(self.encode_compressed_packet(
            packet_id::MOB_EQUIPMENT,
            &MobEquipment::encode_item(
                self.entity_runtime_id,
                self.inventory.held_item(),
                self.inventory.held_slot,
            ),
        ));

        // 10. CraftingData (empty)
        responses.push(
            self.encode_compressed_packet(packet_id::CRAFTING_DATA, &CraftingData::encode_empty()),
        );

        // 10. CreativeContent (empty)
        responses.push(self.encode_compressed_packet(
            packet_id::CREATIVE_CONTENT,
            &CreativeContent::encode_empty(),
        ));

        info!("[{}] Sent {} PreSpawn packets", self.addr, responses.len());

        responses
    }

    // ── Packet encoding helpers ──

    /// Encode a raw packet (no compression, no algo byte, no encryption).
    /// Used for the first response (NetworkSettings) before compression is negotiated.
    /// Format: 0xFE + VarUInt32(len) + packet_bytes (no compression byte!)
    fn encode_raw_packet(&self, pkt_id: u32, payload: &[u8]) -> Vec<u8> {
        let pkt_bytes = codec::encode_packet(pkt_id, payload);

        // Build raw batch: just VarUInt32(len) + packet_data, NO algo byte
        let mut batch_inner = mc_rs_proto::io::ProtoWriter::with_capacity(pkt_bytes.len() + 5);
        batch_inner.write_var_u32(pkt_bytes.len() as u32);
        batch_inner.write_raw(&pkt_bytes);

        // Wrap with 0xFE header
        let inner = batch_inner.into_bytes();
        let mut result = Vec::with_capacity(1 + inner.len());
        result.push(0xFE);
        result.extend_from_slice(&inner);
        result
    }

    /// Encode a compressed (and optionally encrypted) packet.
    pub fn is_in_game(&self) -> bool {
        self.state == ConnectionState::InGame
    }

    pub fn encode_compressed_packet(&self, pkt_id: u32, payload: &[u8]) -> Vec<u8> {
        let pkt_bytes = codec::encode_packet(pkt_id, payload);
        let batch_payload = batch::encode_batch(&[pkt_bytes], self.compression_algo, 7);

        // If encryption is enabled, we need to encrypt
        // But we can't mutate self here, so encryption is handled in send path
        let mut result = Vec::with_capacity(1 + batch_payload.len());
        result.push(0xFE);
        result.extend_from_slice(&batch_payload);
        result
    }

    /// Prepare a batch for sending: apply encryption if needed.
    /// This should be called right before sending over RakNet.
    pub fn prepare_for_send(&mut self, raw_batch: Vec<u8>) -> Vec<u8> {
        if raw_batch.is_empty() || raw_batch[0] != 0xFE {
            return raw_batch;
        }

        let result = if let Some(ref mut ctx) = self.encryption {
            let payload = &raw_batch[1..];
            let encrypted = ctx.encrypt(payload);
            let mut r = Vec::with_capacity(1 + encrypted.len());
            r.push(0xFE);
            r.extend_from_slice(&encrypted);
            r
        } else {
            raw_batch
        };

        // Activate pending encryption AFTER sending this packet
        if let Some(key) = self.pending_encryption_key.take() {
            debug!("[{}] Encryption now active", self.addr);
            self.encryption = Some(EncryptionContext::new(key));
        }

        result
    }
}
