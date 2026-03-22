use std::collections::HashSet;
use std::net::SocketAddr;

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

use crate::player_registry;
use crate::world::flat_generator;

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

    // Chunk tracking
    pub sent_chunks: HashSet<(i32, i32)>,
    pub view_distance: i32,
    pub last_chunk_x: i32,
    pub last_chunk_z: i32,

    // Packets to broadcast to ALL other players
    pub broadcasts: Vec<Vec<u8>>,

    // Server-side actions from commands (read by main.rs)
    pub pending_actions: Vec<mc_rs_command::CommandAction>,

    // Server keypair (shared across connections)
    server_keypair: std::sync::Arc<ServerKeyPair>,
}

impl Connection {
    pub fn new(addr: SocketAddr, server_keypair: std::sync::Arc<ServerKeyPair>) -> Self {
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
            position: [0.5, -58.379, 0.5], // feet=-60 + 1.621 eye offset
            pitch: 0.0,
            yaw: 0.0,
            head_yaw: 0.0,
            entity_runtime_id: player_registry::next_entity_id() as u64,
            tick: 0,
            sent_chunks: HashSet::new(),
            view_distance: 8,
            last_chunk_x: 0,
            last_chunk_z: 0,
            broadcasts: Vec::new(),
            pending_actions: Vec::new(),
            server_keypair,
        }
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
            let algo = CompressionAlgorithm::from_u8(decrypted[0])
                .unwrap_or(CompressionAlgorithm::None);
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
            | (ConnectionState::PreSpawn, packet_id::SERVERBOUND_LOADING_SCREEN) => {
                Vec::new()
            }

            // ── SpawnResponse ──
            (ConnectionState::SpawnResponse, packet_id::SET_LOCAL_PLAYER_AS_INITIALIZED) => {
                self.handle_set_local_player_as_initialized()
            }

            // ── Silently ignored packets ──
            // ── InGame ──
            (ConnectionState::InGame, packet_id::PLAYER_AUTH_INPUT) => {
                self.handle_player_auth_input(reader)
            }
            (ConnectionState::InGame, packet_id::TEXT) => {
                self.handle_text(reader)
            }
            (ConnectionState::InGame, packet_id::COMMAND_REQUEST) => {
                self.handle_command_request(reader)
            }

            // ── Silently ignored ──
            (_, packet_id::EMOTE_LIST)
            | (_, packet_id::SERVERBOUND_LOADING_SCREEN)
            | (ConnectionState::PreSpawn, packet_id::PLAYER_AUTH_INPUT)
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

        info!(
            "[{}] Login: protocol={}",
            self.addr, pkt.protocol_version
        );

        // Parse authInfoJson to extract identity and public key
        match jwt::extract_login_identity(&pkt.chain_data) {
            Ok(identity) => {
                self.client_pub_key_b64 = Some(identity.public_key_b64);
                self.display_name = Some(identity.display_name);
                self.xuid = if identity.xuid.is_empty() { None } else { Some(identity.xuid) };
                if !identity.uuid_str.is_empty() {
                    self.uuid = uuid::Uuid::parse_str(&identity.uuid_str).ok();
                }
                info!(
                    "[{}] Player: {} (xuid={}, auth={})",
                    self.addr,
                    self.display_name.as_deref().unwrap_or("?"),
                    self.xuid.as_deref().unwrap_or("none"),
                    if identity.authenticated { "xbox" } else { "offline" },
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

        let handshake_pkt = ServerToClientHandshake {
            jwt: handshake_jwt,
        };
        let response =
            self.encode_compressed_packet(packet_id::SERVER_TO_CLIENT_HANDSHAKE, &handshake_pkt.encode());

        // DON'T enable encryption yet — the ServerToClientHandshake must be sent unencrypted.
        // Store the key to activate AFTER this packet is sent.
        self.pending_encryption_key = Some(aes_key);

        self.state = ConnectionState::Handshake;
        debug!("[{}] → Handshake state (encryption pending)", self.addr);

        vec![response]
    }

    fn handle_client_to_server_handshake(&mut self, _reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        info!("[{}] ClientToServerHandshake received — encryption verified", self.addr);

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

        debug!("[{}] ResourcePackClientResponse: status={}", self.addr, status);

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
                debug!("[{}] Unexpected resource pack status: {}", self.addr, status);
                Vec::new()
            }
        }
    }

    fn handle_request_chunk_radius(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let radius = reader.read_var_i32().unwrap_or(4);
        let clamped = radius.clamp(2, 8);
        self.view_distance = clamped;
        info!("[{}] RequestChunkRadius: {} (responding with {})", self.addr, radius, clamped);

        let mut responses = Vec::new();

        // ChunkRadiusUpdated
        let radius_pkt = ChunkRadiusUpdated { radius: clamped };
        responses.push(self.encode_compressed_packet(
            packet_id::CHUNK_RADIUS_UPDATED,
            &radius_pkt.encode(),
        ));

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

        // Send flat chunks around spawn and track them
        let spawn_chunk_x = spawn_x >> 4;
        let spawn_chunk_z = spawn_z >> 4;
        self.last_chunk_x = spawn_chunk_x;
        self.last_chunk_z = spawn_chunk_z;

        let (sub_chunk_count, chunk_payload) = flat_generator::generate_flat_chunk();
        for cx in (spawn_chunk_x - clamped)..=(spawn_chunk_x + clamped) {
            for cz in (spawn_chunk_z - clamped)..=(spawn_chunk_z + clamped) {
                let chunk = LevelChunk {
                    chunk_x: cx,
                    chunk_z: cz,
                    dimension_id: 0,
                    sub_chunk_count,
                    cache_enabled: false,
                    payload: chunk_payload.clone(),
                };
                responses.push(self.encode_compressed_packet(
                    packet_id::LEVEL_CHUNK,
                    &chunk.encode(),
                ));
                self.sent_chunks.insert((cx, cz));
            }
        }
        info!("[{}] Sent {} chunks (radius={})", self.addr, self.sent_chunks.len(), clamped);

        // PLAYER_SPAWN — send after chunks
        let spawn_status = PlayStatus {
            status: PlayStatusType::PlayerSpawn,
        };
        responses.push(self.encode_compressed_packet(
            packet_id::PLAY_STATUS,
            &spawn_status.encode(),
        ));
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
        let response = self.encode_compressed_packet(
            packet_id::PLAY_STATUS,
            &spawn_status.encode(),
        );
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

        // Send SetActorData with NO_AI=false — enables client-side physics/gravity
        let player_name = self.display_name.clone().unwrap_or_default();
        let actor_data = SetActorData::player_in_game(self.entity_runtime_id, &player_name);
        vec![self.encode_compressed_packet(
            packet_id::SET_ACTOR_DATA,
            &actor_data.encode(),
        )]
    }

    // ── InGame handlers ──

    fn handle_player_auth_input(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(pkt) = mc_rs_proto::packets::player::PlayerAuthInput::decode(reader) else {
            return Vec::new();
        };

        // Validate position (anti-cheat basics)
        if !pkt.position[0].is_finite() || !pkt.position[1].is_finite() || !pkt.position[2].is_finite() {
            return Vec::new(); // ignore invalid position
        }

        // Void kill check
        if pkt.position[1] < -128.0 {
            // Player fell into the void — teleport back to spawn
            self.position = [0.5, -57.0, 0.5];
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
            return vec![self.encode_compressed_packet(
                packet_id::MOVE_PLAYER,
                &reset.encode(),
            )];
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
        self.broadcasts.push(self.encode_compressed_packet(
            packet_id::MOVE_PLAYER,
            &move_pkt.encode(),
        ));

        // Check if player moved to a new chunk — send new chunks dynamically
        let mut responses = Vec::new();
        let chunk_x = (self.position[0] as i32) >> 4;
        let chunk_z = (self.position[2] as i32) >> 4;

        if chunk_x != self.last_chunk_x || chunk_z != self.last_chunk_z {
            self.last_chunk_x = chunk_x;
            self.last_chunk_z = chunk_z;

            // Send NetworkChunkPublisherUpdate with new position
            let ncpu = NetworkChunkPublisherUpdate {
                position: [
                    self.position[0] as i32,
                    self.position[1] as i32,
                    self.position[2] as i32,
                ],
                radius: (self.view_distance * 16) as u32,
            };
            responses.push(self.encode_compressed_packet(
                packet_id::NETWORK_CHUNK_PUBLISHER_UPDATE,
                &ncpu.encode(),
            ));

            // Send missing chunks in view distance
            let mut new_chunks = 0;
            for dx in -self.view_distance..=self.view_distance {
                for dz in -self.view_distance..=self.view_distance {
                    let cx = chunk_x + dx;
                    let cz = chunk_z + dz;
                    if !self.sent_chunks.contains(&(cx, cz)) {
                        let (sub_count, payload) = flat_generator::generate_flat_chunk();
                        let chunk_pkt = LevelChunk {
                            chunk_x: cx,
                            chunk_z: cz,
                            dimension_id: 0,
                            sub_chunk_count: sub_count,
                            cache_enabled: false,
                            payload: payload.clone(),
                        };
                        responses.push(self.encode_compressed_packet(
                            packet_id::LEVEL_CHUNK,
                            &chunk_pkt.encode(),
                        ));
                        self.sent_chunks.insert((cx, cz));
                        new_chunks += 1;
                    }
                }
            }

            if new_chunks > 0 {
                debug!("[{}] Sent {} new chunks around ({}, {})", self.addr, new_chunks, chunk_x, chunk_z);
            }
        }

        // Handle block actions (breaking/placing)
        for action in &pkt.block_actions {
            match action.action_type {
                // PREDICT_DESTROY_BLOCK (26) or CREATIVE_PLAYER_DESTROY_BLOCK
                26 => {
                    let air_id = flat_generator::block_ids::AIR;
                    let update = UpdateBlock {
                        position: action.position,
                        runtime_id: air_id,
                        flags: 3, // FLAG_NEIGHBORS | FLAG_NETWORK
                        layer: 0,
                    };
                    let update_bytes = self.encode_compressed_packet(
                        packet_id::UPDATE_BLOCK,
                        &update.encode(),
                    );
                    // Send to the player who broke the block
                    responses.push(update_bytes.clone());
                    // Broadcast to all other players
                    self.broadcasts.push(update_bytes);
                    info!(
                        "[{}] Block broken at ({}, {}, {})",
                        self.addr, action.position[0], action.position[1], action.position[2]
                    );
                }
                _ => {} // Other actions (START_BREAK, CRACK_BREAK, etc.) — ignored for now
            }
        }

        responses
    }

    fn handle_text(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(pkt) = mc_rs_proto::packets::player::Text::decode(reader) else {
            return Vec::new();
        };

        let player_name = self.display_name.clone().unwrap_or_else(|| "Player".to_string());
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
        self.broadcasts.push(self.encode_compressed_packet(
            packet_id::TEXT,
            &chat,
        ));

        Vec::new()
    }

    fn handle_command_request(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(command) = reader.read_string() else {
            return Vec::new();
        };

        let ctx = mc_rs_command::CommandContext {
            player_name: self.display_name.clone().unwrap_or_else(|| "Player".to_string()),
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
                responses.push(self.encode_compressed_packet(
                    packet_id::MOVE_PLAYER,
                    &move_pkt.encode(),
                ));
            }
            mc_rs_command::CommandAction::Broadcast { message } => {
                let chat = mc_rs_proto::packets::player::Text::chat("Server", message, "");
                self.broadcasts.push(self.encode_compressed_packet(packet_id::TEXT, &chat));
            }
            mc_rs_command::CommandAction::SetTime { .. }
            | mc_rs_command::CommandAction::SetGamemode { .. }
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
        writer.write_string("");  // world_template_version
        writer.write_u16_le(0);   // resource_packs count

        vec![self.encode_compressed_packet(
            packet_id::RESOURCE_PACKS_INFO,
            writer.as_bytes(),
        )]
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
        writer.write_bool(false);            // must_accept
        writer.write_var_u32(0);             // resource_pack_stack count
        writer.write_string("1.26.2");       // base_game_version
        writer.write_u32_le(0);              // experiments count
        writer.write_bool(false);            // experiments_previously_toggled
        writer.write_bool(false);            // use_vanilla_editor_packs

        vec![self.encode_compressed_packet(
            packet_id::RESOURCE_PACK_STACK,
            writer.as_bytes(),
        )]
    }

    // ── PreSpawn placeholder (will be fully built in steps 11-14) ──

    fn send_pre_spawn_packets(&self) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();

        // StartGame
        let start_game = StartGame::default_flat_with_id(self.entity_runtime_id as i64);
        responses.push(self.encode_compressed_packet(
            packet_id::START_GAME,
            &start_game.encode(),
        ));

        // ItemRegistry (empty) — test if this crashes
        responses.push(self.encode_compressed_packet(
            packet_id::ITEM_REGISTRY,
            &ItemRegistry::encode_empty(),
        ));

        // AvailableActorIdentifiers — real NBT from PMMP
        static ENTITY_IDENTIFIERS_NBT: &[u8] =
            include_bytes!("../data/entity_identifiers.nbt");
        responses.push(self.encode_compressed_packet(
            packet_id::AVAILABLE_ACTOR_IDENTIFIERS,
            ENTITY_IDENTIFIERS_NBT,
        ));

        // BiomeDefinitionList — empty (protocol 924 custom format)
        let mut biome_writer = mc_rs_proto::io::ProtoWriter::with_capacity(4);
        biome_writer.write_var_u32(0);
        biome_writer.write_var_u32(0);
        responses.push(self.encode_compressed_packet(
            packet_id::BIOME_DEFINITION_LIST,
            biome_writer.as_bytes(),
        ));

        // CraftingData (empty)
        responses.push(self.encode_compressed_packet(
            packet_id::CRAFTING_DATA,
            &CraftingData::encode_empty(),
        ));

        // CreativeContent (empty — needs BOTH groups and items counts)
        responses.push(self.encode_compressed_packet(
            packet_id::CREATIVE_CONTENT,
            &CreativeContent::encode_empty(),
        ));

        // UpdateAbilities — survival mode abilities (no fly, no noclip)
        let abilities = UpdateAbilities::default_survival(self.entity_runtime_id as i64);
        responses.push(self.encode_compressed_packet(
            packet_id::UPDATE_ABILITIES,
            &abilities.encode(),
        ));

        // UpdateAttributes — health, hunger, movement speed
        let attributes = UpdateAttributes::default_survival(self.entity_runtime_id);
        responses.push(self.encode_compressed_packet(
            packet_id::UPDATE_ATTRIBUTES,
            &attributes.encode(),
        ));

        // UpdateAdventureSettings
        let adventure = UpdateAdventureSettings {
            no_pvm: false,
            no_mvp: false,
            immutable_world: false,
            show_name_tags: true,
            auto_jump: true,
        };
        responses.push(self.encode_compressed_packet(
            packet_id::UPDATE_ADVENTURE_SETTINGS,
            &adventure.encode(),
        ));

        // SetActorData — player metadata with NO_AI=true (freeze during chunk loading)
        let player_name = self.display_name.clone().unwrap_or_default();
        let actor_data = SetActorData::player_pre_spawn(self.entity_runtime_id, &player_name);
        responses.push(self.encode_compressed_packet(
            packet_id::SET_ACTOR_DATA,
            &actor_data.encode(),
        ));

        // AvailableCommands — register commands for tab-complete
        let cmd_registry = mc_rs_command::CommandRegistry::new();
        let cmd_list = cmd_registry.all_commands();
        let cmd_refs: Vec<(&str, &str)> = cmd_list.iter().map(|&(n, d)| (n, d)).collect();
        let commands = AvailableCommands::encode_simple(&cmd_refs);
        responses.push(self.encode_compressed_packet(
            packet_id::AVAILABLE_COMMANDS,
            &commands,
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
        let batch_payload =
            batch::encode_batch(&[pkt_bytes], self.compression_algo, 7);

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
