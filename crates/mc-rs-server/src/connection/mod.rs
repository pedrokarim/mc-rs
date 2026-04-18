mod chat;
mod chunks;
mod forms;
mod inventory;
mod login;
mod movement;
mod spawn;

use std::collections::{HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tracing::{debug, info, warn};

use mc_rs_crypto::ecdh::ServerKeyPair;
use mc_rs_crypto::encrypt::EncryptionContext;
use mc_rs_proto::batch::{self, CompressionAlgorithm};
use mc_rs_proto::codec;
use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::packet_id;

use crate::attribute::{AttributeMap, HungerManager};
use crate::combat::CombatState;
use crate::config::ConnectionConfig;
use crate::event::EventManager;
use crate::inventory::PlayerInventory;
use crate::inventory_manager::InventoryManager;
use crate::item_entities::PendingItemEntitySpawn;
use crate::player_registry;
use crate::world::chunk_cache::ChunkCache;

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

pub(super) const CHUNKS_PER_TICK: usize = 4;
pub(super) const PLAYER_INVENTORY_SCREEN_ID: u8 = 1;
pub(super) const PLAYER_INVENTORY_WINDOW_TYPE: u8 = 0xFF;

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
    pub(super) pending_encryption_key: Option<[u8; 32]>,
    pub compression_algo: CompressionAlgorithm,

    // Player identity (set after login)
    pub display_name: Option<String>,
    pub uuid: Option<uuid::Uuid>,
    pub xuid: Option<String>,
    pub(super) client_pub_key_b64: Option<String>,

    // Player state
    pub spawn_position: [f32; 3],
    pub position: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub entity_runtime_id: u64,
    pub tick: u64,
    pub gamemode: i32, // 0=survival, 1=creative, 2=adventure, 3=spectator
    pub(super) world_gamemode: i32,
    pub(super) current_difficulty: i32,
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
    pub(super) next_form_id: u32,

    // Server keypair (shared across connections)
    pub(super) server_keypair: std::sync::Arc<ServerKeyPair>,

    // Player inventory
    pub inventory: PlayerInventory,
    pub(super) inventory_manager: InventoryManager,
    pub(super) player_inventory_window_id: u8,
    pub(super) player_inventory_open: bool,

    // Player stats : attributs (santé, faim, XP), combat, hunger.
    pub attributes: AttributeMap,
    pub combat: CombatState,
    pub hunger: HungerManager,
    /// Horloge game-tick (20 TPS = 1 tick / 5 server-ticks).
    pub(super) game_tick_accum: u64,

    /// Y du pic pendant le fall en cours ; None si pas en chute.
    pub(super) fall_peak_y: Option<f32>,

    /// Player est mort (HEALTH=0) et attend l'action RESPAWN du client.
    pub(super) dead: bool,

    /// Réserve d'air en game ticks (15 sec = 300). Décrémentée quand les
    /// yeux sont dans l'eau ; drowning damage quand = 0.
    pub(super) air_supply: i32,

    /// Compteur interne entre tick (drowning, lava) — incrémenté à chaque
    /// tick_game_state, utilisé pour espacer les tics de dégâts.
    pub(super) environment_tick: i32,

    // Event manager partagé (fire events pour plugins).
    pub events: Arc<Mutex<EventManager>>,

    // Shared chunk cache (world persistence)
    pub(super) chunk_cache: Arc<Mutex<ChunkCache>>,

    // Server config subset for this connection
    pub(super) config: Arc<ConnectionConfig>,
}

impl Connection {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        addr: SocketAddr,
        server_keypair: std::sync::Arc<ServerKeyPair>,
        chunk_cache: Arc<Mutex<ChunkCache>>,
        config: Arc<ConnectionConfig>,
        world_spawn_override: Option<[f32; 3]>,
        world_gamemode: i32,
        current_difficulty: i32,
        is_op: bool,
        events: Arc<Mutex<EventManager>>,
    ) -> Self {
        let spawn_position = world_spawn_override
            .unwrap_or_else(|| spawn::find_spawn_position(&chunk_cache, config.world_seed));
        let inventory = PlayerInventory::new();

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
            inventory_manager: InventoryManager::new(),
            player_inventory_window_id: PLAYER_INVENTORY_SCREEN_ID,
            player_inventory_open: false,
            attributes: AttributeMap::default_for_player(),
            combat: CombatState::new(),
            hunger: HungerManager::new(),
            game_tick_accum: 0,
            fall_peak_y: None,
            dead: false,
            air_supply: 300,
            environment_tick: 0,
            events,
            next_form_id: 1,
            server_keypair,
            chunk_cache,
            config,
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

        // PMMP `flushPendingUpdates` à la fin du traitement des paquets : envoie
        // tout `pending_sync` queued par les listeners (block place,
        // pickup-via-add_item, /give, etc.). Sans ça, les mutations server-side
        // n'arrivent jamais au client.
        if self.is_in_game() {
            for pkt in self.tick_inventory_flush() {
                responses.push(pkt);
            }
        }

        responses
    }

    /// Handle a single decoded packet. Returns response packets to send.
    fn handle_packet(&mut self, pkt_id: u32, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        match (self.state, pkt_id) {
            // -- SessionStart --
            (ConnectionState::SessionStart, packet_id::REQUEST_NETWORK_SETTINGS) => {
                self.handle_request_network_settings(reader)
            }

            // -- Login --
            (ConnectionState::Login, packet_id::LOGIN) => self.handle_login(reader),

            // -- Handshake --
            (ConnectionState::Handshake, packet_id::CLIENT_TO_SERVER_HANDSHAKE) => {
                self.handle_client_to_server_handshake(reader)
            }

            // -- ResourcePacks --
            (ConnectionState::ResourcePacks, packet_id::RESOURCE_PACK_CLIENT_RESPONSE) => {
                self.handle_resource_pack_client_response(reader)
            }

            // -- PreSpawn --
            (ConnectionState::PreSpawn, packet_id::REQUEST_CHUNK_RADIUS) => {
                self.handle_request_chunk_radius(reader)
            }

            // Silently ignore these in PreSpawn
            (ConnectionState::PreSpawn, packet_id::PLAYER_AUTH_INPUT)
            | (ConnectionState::PreSpawn, packet_id::SERVERBOUND_LOADING_SCREEN) => Vec::new(),

            // -- SpawnResponse --
            (ConnectionState::SpawnResponse, packet_id::SET_LOCAL_PLAYER_AS_INITIALIZED) => {
                self.handle_set_local_player_as_initialized()
            }
            // Le client envoie MobEquipment (sélection hotbar) juste avant
            // SET_LOCAL_PLAYER_AS_INITIALIZED. PMMP l'accepte silencieusement.
            (ConnectionState::SpawnResponse, packet_id::MOB_EQUIPMENT) => {
                self.handle_mob_equipment(reader)
            }

            // -- InGame --
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
            // RespawnPacket C→S : client envoie CLIENT_READY_TO_SPAWN (state=2)
            // après avoir reçu READY_TO_SPAWN. PMMP DeathPacketHandler::handleRespawn
            // renvoie un READY_TO_SPAWN pour confirmer la transition.
            (ConnectionState::InGame, packet_id::RESPAWN) => self.handle_client_respawn(reader),

            // -- Silently ignored --
            (_, packet_id::EMOTE_LIST)
            | (_, packet_id::SERVERBOUND_LOADING_SCREEN)
            | (_, packet_id::ANIMATE)
            | (_, packet_id::INTERACT)
            | (ConnectionState::SpawnResponse, packet_id::PLAYER_AUTH_INPUT)
            | (_, 0x081)
            | (_, 0x024) => Vec::new(),

            _ => {
                info!(
                    "[{}] Unhandled packet 0x{:03X} in state {:?}",
                    self.addr, pkt_id, self.state
                );
                Vec::new()
            }
        }
    }

    /// Take broadcast packets (to be sent to ALL other players).
    pub fn take_broadcasts(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.broadcasts)
    }

    pub fn take_pending_commands(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_commands)
    }

    pub fn take_pending_entity_attacks(&mut self) -> Vec<PendingEntityAttack> {
        std::mem::take(&mut self.pending_entity_attacks)
    }

    pub fn is_in_game(&self) -> bool {
        self.state == ConnectionState::InGame
    }

    // -- Packet encoding helpers --

    /// Encode a raw packet (no compression, no algo byte, no encryption).
    /// Used for the first response (NetworkSettings) before compression is negotiated.
    pub(super) fn encode_raw_packet(&self, pkt_id: u32, payload: &[u8]) -> Vec<u8> {
        let pkt_bytes = codec::encode_packet(pkt_id, payload);

        let mut batch_inner = mc_rs_proto::io::ProtoWriter::with_capacity(pkt_bytes.len() + 5);
        batch_inner.write_var_u32(pkt_bytes.len() as u32);
        batch_inner.write_raw(&pkt_bytes);

        let inner = batch_inner.into_bytes();
        let mut result = Vec::with_capacity(1 + inner.len());
        result.push(0xFE);
        result.extend_from_slice(&inner);
        result
    }

    /// Encode a compressed (and optionally encrypted) packet.
    pub fn encode_compressed_packet(&self, pkt_id: u32, payload: &[u8]) -> Vec<u8> {
        // DEBUG DUMP — même format que PMMP NetworkSession.php.
        // Dumpe le (pktId, payload) de chaque paquet envoyé, pour diff contre PMMP.
        let hex: String = payload
            .iter()
            .take(256)
            .map(|b| format!("{:02X}", b))
            .collect();
        let line = format!(
            "[{}] 0x{:03X} len={} hex={}\n",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            pkt_id,
            payload.len(),
            hex
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("pkt_sent.log")
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));

        let pkt_bytes = codec::encode_packet(pkt_id, payload);
        let batch_payload = batch::encode_batch(&[pkt_bytes], self.compression_algo, 7);

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
