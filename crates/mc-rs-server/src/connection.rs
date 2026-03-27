use std::collections::{HashMap, HashSet, VecDeque};
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
use mc_rs_proto::packets::forms::*;
use mc_rs_proto::packets::login::*;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::*;
use mc_rs_proto::packets::world::*;
use serde_json::json;

use crate::config::ConnectionConfig;
use crate::inventory::PlayerInventory;
use crate::item_entities::PendingItemEntitySpawn;
use crate::item_registry;
use crate::player_data;
use crate::player_registry;
use crate::world::biome;
use crate::world::block_registry::BLOCKS;
use crate::world::chunk_cache::ChunkCache;
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

const CHUNKS_PER_TICK: usize = 4;
const HUB_MENU_SLOT: usize = 0;
const PLAYER_INVENTORY_SCREEN_ID: u8 = 1;
const PLAYER_INVENTORY_WINDOW_TYPE: u8 = 0xFF;

#[derive(Debug, Clone, Copy)]
enum PendingForm {
    HubMenu,
}

#[derive(Debug, Clone)]
pub struct PendingEntityAttack {
    pub target_runtime_id: u64,
    pub action_type: u32,
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
    pub spawn_position: [f32; 3],
    pub position: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub entity_runtime_id: u64,
    pub tick: u64,
    pub gamemode: i32, // 0=survival, 1=creative, 2=adventure, 3=spectator
    pub world_gamemode: i32,
    pub current_difficulty: i32,
    pub is_op: bool,

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

    pub pending_commands: Vec<String>,
    pub pending_item_spawns: Vec<PendingItemEntitySpawn>,
    pub pending_entity_attacks: Vec<PendingEntityAttack>,

    // Server-driven Bedrock forms
    next_form_id: u32,
    pending_forms: HashMap<u32, PendingForm>,

    // Server keypair (shared across connections)
    server_keypair: std::sync::Arc<ServerKeyPair>,

    // Player inventory
    pub inventory: PlayerInventory,
    player_inventory_window_id: u8,
    player_inventory_open: bool,

    // Shared chunk cache (world persistence)
    chunk_cache: Arc<Mutex<ChunkCache>>,

    // Server config subset for this connection
    config: Arc<ConnectionConfig>,
}

fn make_spawn_position(world_x: i32, world_y: i32, world_z: i32) -> [f32; 3] {
    let feet_y = (world_y + 1) as f32;
    [world_x as f32 + 0.5, feet_y + 1.621, world_z as f32 + 0.5]
}

fn hub_menu_item_id() -> i32 {
    item_registry::required_item_id("minecraft:compass")
}

fn find_surface_in_loaded_world(cache: &mut ChunkCache, world_x: i32, world_z: i32) -> Option<i32> {
    for world_y in (-64..=319).rev() {
        let block_id = cache.get_block(world_x, world_y, world_z);
        if block_id != BLOCKS.air && block_id != BLOCKS.water {
            let head = cache.get_block(world_x, world_y + 1, world_z);
            let head_above = cache.get_block(world_x, world_y + 2, world_z);
            if head == BLOCKS.air && head_above == BLOCKS.air {
                return Some(world_y);
            }
        }
    }

    None
}

fn find_spawn_position(chunk_cache: &Arc<Mutex<ChunkCache>>, seed: u64) -> [f32; 3] {
    const SEARCH_STEP: i32 = 8;
    const MAX_RADIUS: i32 = 128;

    let mut fallback = None;
    if let Ok(mut cache) = chunk_cache.lock() {
        for radius in (0..=MAX_RADIUS).step_by(SEARCH_STEP as usize) {
            if radius == 0 {
                if let Some(surface_y) = find_surface_in_loaded_world(&mut cache, 0, 0) {
                    if surface_y > 62 {
                        return make_spawn_position(0, surface_y, 0);
                    }
                    fallback = Some((0, surface_y, 0));
                }
                continue;
            }

            for edge in (-radius..=radius).step_by(SEARCH_STEP as usize) {
                let perimeter_points = [
                    (-radius, edge),
                    (radius, edge),
                    (edge, -radius),
                    (edge, radius),
                ];

                for (world_x, world_z) in perimeter_points {
                    if let Some(surface_y) =
                        find_surface_in_loaded_world(&mut cache, world_x, world_z)
                    {
                        if fallback.is_none_or(|(_, best_y, _)| surface_y > best_y) {
                            fallback = Some((world_x, surface_y, world_z));
                        }

                        if surface_y > 62 {
                            return make_spawn_position(world_x, surface_y, world_z);
                        }
                    }
                }
            }
        }
    }

    if let Some((world_x, surface_y, world_z)) = fallback {
        make_spawn_position(world_x, surface_y, world_z)
    } else {
        terrain_generator::find_spawn_position(seed)
    }
}

impl Connection {
    pub fn new(
        addr: SocketAddr,
        server_keypair: std::sync::Arc<ServerKeyPair>,
        chunk_cache: Arc<Mutex<ChunkCache>>,
        config: Arc<ConnectionConfig>,
        world_spawn_override: Option<[f32; 3]>,
        world_gamemode: i32,
        current_difficulty: i32,
        is_op: bool,
    ) -> Self {
        let spawn_position = world_spawn_override
            .unwrap_or_else(|| find_spawn_position(&chunk_cache, config.world_seed));
        let mut inventory = PlayerInventory::new();
        let menu_stack_id = inventory.next_stack_id();
        inventory.slots[HUB_MENU_SLOT] =
            ItemStackWrapper::new(ItemStack::new(hub_menu_item_id(), 1, 0), menu_stack_id);

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
            spawn_position,
            position: spawn_position,
            pitch: 0.0,
            yaw: 0.0,
            head_yaw: 0.0,
            entity_runtime_id: player_registry::next_entity_id() as u64,
            tick: 0,
            gamemode: config.default_gamemode,
            world_gamemode,
            current_difficulty,
            is_op,
            sent_chunks: HashSet::new(),
            view_distance: config.max_view_distance,
            last_chunk_x: 0,
            last_chunk_z: 0,
            chunk_load_queue: VecDeque::new(),
            chunk_order_countdown: 5, // reorder shortly after spawn (like PMMP)
            broadcasts: Vec::new(),
            pending_commands: Vec::new(),
            pending_item_spawns: Vec::new(),
            pending_entity_attacks: Vec::new(),
            inventory,
            player_inventory_window_id: PLAYER_INVENTORY_SCREEN_ID,
            player_inventory_open: false,
            next_form_id: 1,
            pending_forms: HashMap::new(),
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
        // PMMP-style countdown: doChunkRequests() { if(nextChunkOrderRun-- <= 0) { orderChunks(); } }
        if self.chunk_order_countdown != u32::MAX {
            if self.chunk_order_countdown == 0 {
                self.order_chunks();
                self.chunk_order_countdown = u32::MAX; // idle until next trigger

                debug!(
                    "[{}] order_chunks: queue={}, sent_chunks={}, pos=({},{})",
                    self.addr,
                    self.chunk_load_queue.len(),
                    self.sent_chunks.len(),
                    self.last_chunk_x,
                    self.last_chunk_z,
                );

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

    /// Send a small batch of chunks from the load queue.
    fn send_chunk_batch(&mut self) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();
        let mut sent = 0;
        let queue_before = self.chunk_load_queue.len();

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

        if sent > 0 {
            debug!(
                "[{}] send_chunk_batch: sent={}, queue_remaining={} (was {})",
                self.addr,
                sent,
                self.chunk_load_queue.len(),
                queue_before,
            );
        }

        responses
    }

    pub fn should_stream_chunks(&self) -> bool {
        matches!(
            self.state,
            ConnectionState::PreSpawn | ConnectionState::SpawnResponse | ConnectionState::InGame
        )
    }

    fn ensure_hub_menu_item(&mut self) {
        if self
            .inventory
            .slots
            .iter()
            .any(|slot| slot.item.id == hub_menu_item_id() && !slot.item.is_air())
        {
            return;
        }

        let menu_item = ItemStack::new(hub_menu_item_id(), 1, 0);
        if self.inventory.slots[HUB_MENU_SLOT].item.is_air() {
            let stack_id = self.inventory.next_stack_id();
            self.inventory.slots[HUB_MENU_SLOT] = ItemStackWrapper::new(menu_item, stack_id);
        } else {
            let _ = self.inventory.add_item(menu_item);
        }
    }

    fn open_form(&mut self, form: PendingForm, form_data: String) -> Vec<Vec<u8>> {
        let form_id = self.next_form_id;
        self.next_form_id = self.next_form_id.wrapping_add(1).max(1);
        self.pending_forms.insert(form_id, form);

        let request = ModalFormRequest { form_id, form_data };
        vec![self.encode_compressed_packet(packet_id::MODAL_FORM_REQUEST, &request.encode())]
    }

    fn open_hub_menu(&mut self) -> Vec<Vec<u8>> {
        let form_json = json!({
            "type": "form",
            "title": "§l§bMC-RS Hub",
            "content": "§7Prototype de menu Bedrock inspire des hubs type Hive.\n§fCompass: slot 1\n§8Version simple sans resource pack custom.",
            "buttons": [
                { "text": "§lSpawn Plaza\n§r§7Retourner au spawn" },
                { "text": "§lCreative Flight\n§r§7Passer en creatif" },
                { "text": "§lSurvival Loop\n§r§7Revenir en survie" },
                { "text": "§lBiome Scanner\n§r§7Afficher le biome courant" }
            ]
        });

        self.open_form(PendingForm::HubMenu, form_json.to_string())
    }

    fn handle_hub_menu_selection(&mut self, button_index: u32) -> Vec<Vec<u8>> {
        match button_index {
            0 => {
                self.position = self.spawn_position;
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
                let mut responses =
                    vec![self.encode_compressed_packet(packet_id::MOVE_PLAYER, &move_pkt.encode())];
                self.push_system_message(&mut responses, "Teleported to spawn plaza.");
                responses
            }
            1 => {
                let mut responses = self.apply_gamemode(1);
                self.push_system_message(
                    &mut responses,
                    "Creative mode enabled from the hub menu.",
                );
                responses
            }
            2 => {
                let mut responses = self.apply_gamemode(0);
                self.push_system_message(&mut responses, "Back to survival mode.");
                responses
            }
            3 => {
                let world_x = self.position[0].floor() as i32;
                let world_z = self.position[2].floor() as i32;
                let debug = terrain_generator::get_biome_debug_info(
                    world_x,
                    world_z,
                    self.config.world_seed,
                );
                let biome_def = biome::get_biome(debug.biome_id);
                let mut responses = Vec::new();
                self.push_system_message(
                    &mut responses,
                    format!(
                        "Biome: {} (id={}) | temp={:.3} rain={:.3} | surface_y={} | terrain={:.0}..{:.0} | chunk=({}, {})",
                        biome::biome_name(debug.biome_id),
                        debug.biome_id,
                        debug.temperature,
                        debug.rainfall,
                        debug.surface_y,
                        biome_def.min_elevation,
                        biome_def.max_elevation,
                        world_x.div_euclid(16),
                        world_z.div_euclid(16),
                    ),
                );
                responses
            }
            _ => Vec::new(),
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
            (ConnectionState::InGame, packet_id::INVENTORY_TRANSACTION) => {
                self.handle_inventory_transaction(reader)
            }
            (ConnectionState::InGame, packet_id::INTERACT) => self.handle_interact(reader),
            (ConnectionState::InGame, packet_id::CONTAINER_CLOSE) => {
                self.handle_container_close(reader)
            }
            (ConnectionState::InGame, packet_id::MOB_EQUIPMENT) => {
                self.handle_mob_equipment(reader)
            }
            (ConnectionState::InGame, packet_id::MODAL_FORM_RESPONSE) => {
                self.handle_modal_form_response(reader)
            }
            (ConnectionState::InGame, packet_id::TEXT) => self.handle_text(reader),
            (ConnectionState::InGame, packet_id::COMMAND_REQUEST) => {
                self.handle_command_request(reader)
            }

            // ── Silently ignored ──
            (_, packet_id::EMOTE_LIST)
            | (_, packet_id::SERVERBOUND_LOADING_SCREEN)
            | (_, packet_id::ANIMATE)        // Arm swing — client-side only
            | (_, packet_id::INTERACT)       // InteractPacket outside InGame (e.g. SpawnResponse)
            | (ConnectionState::SpawnResponse, packet_id::PLAYER_AUTH_INPUT)
            | (_, 0x081)
            | (_, 0x024) => Vec::new(),      // BlockPickRequestPacket

            _ => {
                info!(
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

        if pkt.protocol_version != 944 {
            warn!(
                "[{}] Incompatible protocol: {} (expected 944)",
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
                        if let Some(spawn_position) = save.spawn_position {
                            self.spawn_position = [
                                spawn_position[0] as f32,
                                spawn_position[1] as f32,
                                spawn_position[2] as f32,
                            ];
                        }
                        self.inventory = save.inventory.into_runtime();
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
                self.ensure_hub_menu_item();

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

        let client_pub_key = match ecdh::parse_client_public_key(client_pub_b64) {
            Ok(key) => key,
            Err(e) => {
                warn!(
                    "[{}] Failed to parse client public key: {} (key_b64_len={}, first_chars={})",
                    self.addr,
                    e,
                    client_pub_b64.len(),
                    &client_pub_b64[..client_pub_b64.len().min(40)]
                );
                return Vec::new();
            }
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
                self.ensure_hub_menu_item();
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

        let spawn_chunk_x = spawn_x >> 4;
        let spawn_chunk_z = spawn_z >> 4;
        self.last_chunk_x = spawn_chunk_x;
        self.last_chunk_z = spawn_chunk_z;
        self.order_chunks();
        self.chunk_order_countdown = u32::MAX;
        responses.extend(self.send_chunk_batch());
        info!(
            "[{}] Queued spawn chunks (radius={}), first batch={}, remaining_queue={}",
            self.addr,
            clamped,
            self.sent_chunks.len(),
            self.chunk_load_queue.len()
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
        let welcome =
            Text::system("Use /menu or right-click the compass in slot 1 to open the hub menu.");
        vec![self.encode_compressed_packet(packet_id::TEXT, &welcome)]
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
            self.position = self.spawn_position;
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

            // CRITICAL: Tell client the new center of the chunk render area.
            // Without this, the client stops rendering chunks far from the old center.
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

            info!(
                "[{}] NCPU sent: pos=({},{},{}), radius={} blocks, chunk=({},{})",
                self.addr,
                self.position[0] as i32,
                self.position[1] as i32,
                self.position[2] as i32,
                self.view_distance * 16,
                chunk_x,
                chunk_z,
            );

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
            // PMMP uses integer block position (cast to float), NOT +0.5 center
            let block_pos = [bx as f32, by as f32, bz as f32];
            let block_center = [bx as f32 + 0.5, by as f32 + 0.5, bz as f32 + 0.5];

            match action.action_type {
                // START_BREAK (0) or CONTINUE_DESTROY_BLOCK (27)
                0 | 27 => {
                    // Calculate break speed: PMMP = 1.0 / (breakTime * 20)
                    // breakTime comes from block hardness and tool efficiency
                    let break_speed: f32 = {
                        let block_id = if let Ok(mut cache) = self.chunk_cache.lock() {
                            cache.get_block(bx, by, bz)
                        } else {
                            0
                        };
                        match block_id {
                            13079 => 0.0,         // bedrock — unbreakable
                            12421 | 11669 => 1.0, // short grass/tall grass — instant
                            _ => 1.0 / 30.0,      // default ~1.5s with hand
                        }
                    };

                    // PMMP: broadcastPacketToViewers(blockPos, LevelEvent(BLOCK_START_BREAK, speed*65535, blockPos))
                    let event = LevelEvent {
                        event_id: LevelEvent::BLOCK_START_BREAK,
                        position: block_pos,
                        event_data: (65535.0 * break_speed) as i32,
                    };
                    let event_bytes =
                        self.encode_compressed_packet(packet_id::LEVEL_EVENT, &event.encode());
                    responses.push(event_bytes.clone());
                    self.broadcasts.push(event_bytes);
                }

                // ABORT_BREAK (1) or STOP_BREAK (2)
                1 | 2 => {
                    // PMMP: broadcastPacketToViewers(blockPos, LevelEvent(BLOCK_STOP_BREAK, 0, blockPos))
                    let event = LevelEvent {
                        event_id: LevelEvent::BLOCK_STOP_BREAK,
                        position: block_pos,
                        event_data: 0,
                    };
                    let event_bytes =
                        self.encode_compressed_packet(packet_id::LEVEL_EVENT, &event.encode());
                    responses.push(event_bytes.clone());
                    self.broadcasts.push(event_bytes);
                }

                // PREDICT_DESTROY_BLOCK (26)
                26 => {
                    let air_id = crate::world::block_registry::BLOCKS.air;

                    // Send BLOCK_STOP_BREAK to clear crack animation
                    let stop_event = LevelEvent {
                        event_id: LevelEvent::BLOCK_STOP_BREAK,
                        position: block_pos,
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
                        cache.save_chunk_now(bx.div_euclid(16), bz.div_euclid(16));
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

                    // Spawn a dropped item entity instead of inserting directly into inventory.
                    if old_block_id != air_id {
                        if let Some(drop_item) = crate::inventory::block_drop(old_block_id) {
                            let item_id = drop_item.id;
                            self.pending_item_spawns
                                .push(PendingItemEntitySpawn::stationary(
                                    drop_item,
                                    [bx as f32 + 0.5, by as f32 + 0.75, bz as f32 + 0.5],
                                ));
                            info!(
                                "[{}] Queued dropped item entity: item_id={} at ({}, {}, {})",
                                self.addr, item_id, bx, by, bz
                            );
                        } else {
                            info!("[{}] No drop for block {}", self.addr, old_block_id);
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
            let held_item_id = self.inventory.held_item().item.id;

            if interaction.action_type == 1 && held_item_id == hub_menu_item_id() {
                responses.extend(self.open_hub_menu());
            } else if interaction.action_type == 0 {
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
            if existing != crate::world::block_registry::BLOCKS.air {
                return; // Can't place on a non-air block
            }
            cache.set_block(tx, ty, tz, block_runtime_id);
            cache.save_chunk_now(tx.div_euclid(16), tz.div_euclid(16));
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
        let slot_pkt = InventorySlot::encode(
            0,
            slot as u32,
            &self.inventory.slots[slot],
            &self.inventory_screen_container_name(),
        );
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

        debug!(
            "[{}] ItemStackRequest id={} actions={}",
            self.addr,
            request.request_id,
            request.actions.len()
        );

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
                StackRequestAction::Destroy { source, .. } => {
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
                StackRequestAction::Drop { source, .. } => {
                    if let Some(slot_idx) = self.resolve_slot(source.container_id, source.slot_id) {
                        debug!(
                            "[{}] Ignoring drop request for slot {} until item entities are implemented",
                            self.addr, slot_idx
                        );
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

    fn inventory_screen_container_name(&self) -> FullContainerName {
        FullContainerName::new(self.player_inventory_window_id)
    }

    fn advance_player_inventory_window_id(&mut self) -> u8 {
        self.player_inventory_window_id = if self.player_inventory_window_id >= 99 {
            PLAYER_INVENTORY_SCREEN_ID
        } else {
            self.player_inventory_window_id + 1
        };
        self.player_inventory_window_id
    }

    fn push_inventory_sync(&self, responses: &mut Vec<Vec<u8>>) {
        let full_container_name = self.inventory_screen_container_name();
        responses.push(self.encode_compressed_packet(
            packet_id::INVENTORY_CONTENT,
            &InventoryContent::encode_items(0, &self.inventory.slots, &full_container_name),
        ));
        responses.push(self.encode_compressed_packet(
            packet_id::INVENTORY_CONTENT,
            &InventoryContent::encode_items(120, &self.inventory.armor, &full_container_name),
        ));
        responses.push(self.encode_compressed_packet(
            packet_id::INVENTORY_CONTENT,
            &InventoryContent::encode_items(
                119,
                std::slice::from_ref(&self.inventory.offhand),
                &full_container_name,
            ),
        ));
        responses.push(self.encode_compressed_packet(
            packet_id::MOB_EQUIPMENT,
            &MobEquipment::encode_item(
                self.entity_runtime_id,
                self.inventory.held_item(),
                self.inventory.held_slot,
            ),
        ));
    }

    pub fn prepared_inventory_sync_packets(&mut self) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();
        self.push_inventory_sync(&mut responses);
        responses
            .into_iter()
            .map(|response| self.prepare_for_send(response))
            .collect()
    }

    fn push_open_inventory_window_sync(&self, _responses: &mut Vec<Vec<u8>>) {
        // Bedrock opens the main player inventory from ContainerOpen alone.
        // The inventory/backing containers are synced separately on login and slot updates.
    }

    fn handle_inventory_transaction(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        use mc_rs_proto::packets::player::InventoryTransactionData;

        let Ok(transaction) = mc_rs_proto::packets::player::InventoryTransaction::decode(reader)
        else {
            warn!("[{}] Failed to decode InventoryTransaction", self.addr);
            let mut responses = Vec::new();
            self.push_inventory_sync(&mut responses);
            return responses;
        };
        let mc_rs_proto::packets::player::InventoryTransaction {
            request_id: _request_id,
            changed_slots,
            data,
        } = transaction;

        let mut responses = Vec::new();

        match data {
            InventoryTransactionData::Normal { actions } => {
                let is_drop_attempt = actions.iter().any(|action| {
                    action.source_type == 2
                        && action.source_flags == Some(0)
                        && !action.new_item.item.is_air()
                });
                if is_drop_attempt {
                    debug!(
                        "[{}] Rejecting legacy drop transaction until item entities are implemented",
                        self.addr
                    );
                    self.push_inventory_sync(&mut responses);
                }
            }
            InventoryTransactionData::Mismatch { .. } => {
                self.push_inventory_sync(&mut responses);
            }
            InventoryTransactionData::UseItem { .. }
            | InventoryTransactionData::ReleaseItem { .. }
            | InventoryTransactionData::Unknown { .. } => {}
            InventoryTransactionData::UseItemOnEntity {
                actor_runtime_id,
                action_type,
                hotbar_slot,
                ..
            } => {
                if (0..=8).contains(&hotbar_slot) {
                    self.inventory.held_slot = hotbar_slot as u8;
                }
                self.pending_entity_attacks.push(PendingEntityAttack {
                    target_runtime_id: actor_runtime_id,
                    action_type,
                });
                debug!(
                    "[{}] Queued entity interaction: target_runtime_id={} action_type={}",
                    self.addr, actor_runtime_id, action_type
                );
            }
        }

        if !changed_slots.is_empty() && responses.is_empty() {
            self.push_inventory_sync(&mut responses);
        }

        responses
    }

    /// Resolve a container_id + slot_id to an index in self.inventory.slots.
    fn resolve_slot(&self, container_id: u8, slot_id: u8) -> Option<usize> {
        match container_id {
            0 | 12 | 28 | 29 => {
                // Inventory / hotbar / combined inventory UI containers.
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
            stack_id: if item.item.is_air() { 0 } else { 1 },
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
            if self.player_inventory_open {
                return Vec::new();
            }

            let window_id = self.advance_player_inventory_window_id();
            let container_open =
                ContainerOpen::entity_inventory(window_id, self.entity_runtime_id as i64);
            let mut responses = Vec::new();
            responses.push(
                self.encode_compressed_packet(packet_id::CONTAINER_OPEN, &container_open.encode()),
            );
            self.player_inventory_open = true;
            self.push_open_inventory_window_sync(&mut responses);
            info!(
                "[{}] Opening player inventory (window_id={})",
                self.addr, container_open.window_id
            );
            return responses;
        }

        Vec::new()
    }

    fn handle_modal_form_response(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(response) = ModalFormResponse::decode(reader) else {
            warn!("[{}] Failed to decode ModalFormResponse", self.addr);
            return Vec::new();
        };

        let Some(form) = self.pending_forms.remove(&response.form_id) else {
            debug!(
                "[{}] Ignoring response for unknown form_id={}",
                self.addr, response.form_id
            );
            return Vec::new();
        };

        if let Some(reason) = response.cancel_reason {
            debug!(
                "[{}] Form {} closed with cancel_reason={}",
                self.addr, response.form_id, reason
            );
            return Vec::new();
        }

        match form {
            PendingForm::HubMenu => {
                let Some(raw) = response.response_data else {
                    return Vec::new();
                };

                let button_index = serde_json::from_str::<u32>(&raw)
                    .ok()
                    .or_else(|| raw.trim().parse::<u32>().ok());

                if let Some(button_index) = button_index {
                    self.handle_hub_menu_selection(button_index)
                } else {
                    warn!("[{}] Invalid hub menu response payload: {}", self.addr, raw);
                    Vec::new()
                }
            }
        }
    }

    fn handle_container_close(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let raw_window_id = reader.read_u8().unwrap_or(0);
        let window_type = reader.read_u8().unwrap_or(0);
        let _server = reader.read_bool().unwrap_or(false);

        let window_id = if raw_window_id == u8::MAX {
            self.player_inventory_window_id
        } else {
            raw_window_id
        };

        info!(
            "[{}] ContainerClose window_id={} window_type={}",
            self.addr, window_id, window_type
        );

        if window_id == self.player_inventory_window_id {
            self.player_inventory_open = false;
        }

        // Echo back the close
        let close = ContainerClose {
            window_id,
            window_type: if window_id == self.player_inventory_window_id {
                PLAYER_INVENTORY_WINDOW_TYPE
            } else {
                window_type
            },
            server: false,
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
            self.pending_commands.push(pkt.message);
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
            warn!("[{}] Failed to read command string", self.addr);
            return Vec::new();
        };

        info!("[{}] CommandRequest received: {}", self.addr, command);
        self.pending_commands.push(command);
        Vec::new()
    }

    pub fn take_pending_commands(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_commands)
    }

    pub fn take_pending_entity_attacks(&mut self) -> Vec<PendingEntityAttack> {
        std::mem::take(&mut self.pending_entity_attacks)
    }

    pub fn encode_system_message(&self, message: impl Into<String>) -> Vec<u8> {
        let msg = mc_rs_proto::packets::player::Text::system(&message.into());
        self.encode_compressed_packet(packet_id::TEXT, &msg)
    }

    fn push_system_message(&self, responses: &mut Vec<Vec<u8>>, message: impl Into<String>) {
        responses.push(self.encode_system_message(message));
    }

    pub fn teleport_to(&mut self, position: [f32; 3]) -> Vec<Vec<u8>> {
        self.position = position;
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
        vec![self.encode_compressed_packet(packet_id::MOVE_PLAYER, &move_pkt.encode())]
    }

    pub fn open_hub_menu_packets(&mut self) -> Vec<Vec<u8>> {
        self.open_hub_menu()
    }

    pub fn apply_gamemode_packets(&mut self, mode: i32) -> Vec<Vec<u8>> {
        self.apply_gamemode(mode)
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
        writer.write_string("1.26.10"); // base_game_version
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
        start_game.world_gamemode = self.world_gamemode;
        start_game.difficulty = self.current_difficulty;
        start_game.world_name = self.config.world_name.clone();
        start_game.generator = self.config.generator_id;
        responses.push(self.encode_compressed_packet(packet_id::START_GAME, &start_game.encode()));

        responses.push(
            self.encode_compressed_packet(packet_id::ITEM_REGISTRY, item_registry::payload()),
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

        // 6. AvailableCommands are synced after spawn from the shared command map.

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
        self.push_inventory_sync(&mut responses);

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
