use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Cursor;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine as _;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use rand::Rng;
use serde_json::Value;
use tracing::{debug, info, warn};

use mc_rs_proto::batch;
use mc_rs_proto::codec::*;
use mc_rs_proto::packets::{self, *};
use mc_rs_raknet::{Reliability, ServerHandle};

static SESSIONS: Mutex<Option<HashMap<SocketAddr, Session>>> = Mutex::new(None);
static TRACE_SPAWN_ENABLED: OnceLock<bool> = OnceLock::new();
static BLOCK_RUNTIME_IDS: OnceLock<Result<BlockRuntimeIds, String>> = OnceLock::new();
static TRACE_DUMPED_KEYS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static TRACE_DUMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn sessions() -> std::sync::MutexGuard<'static, Option<HashMap<SocketAddr, Session>>> {
    SESSIONS.lock().unwrap()
}

fn get_or_init() -> &'static Mutex<Option<HashMap<SocketAddr, Session>>> {
    let mut guard = SESSIONS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    drop(guard);
    &SESSIONS
}

const OVERWORLD_MIN_SUBCHUNK_Y: i32 = -4;
const OVERWORLD_MAX_SUBCHUNK_Y: i32 = 19;
const OVERWORLD_BIOME_SECTIONS: i32 = OVERWORLD_MAX_SUBCHUNK_Y - OVERWORLD_MIN_SUBCHUNK_Y + 1;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    SessionStart,
    Login,
    ResourcePacks,
    PreSpawn,
    WaitingForChunkRadius,
    SendingChunks,
    WaitingForSpawnResponse,
    InGame,
}

#[derive(Debug, Clone, Copy)]
struct BlockRuntimeIds {
    air: i32,
    bedrock: i32,
    stone: i32,
    dirt: i32,
    grass_block: i32,
    water: i32,
}

struct Session {
    phase: Phase,
    compression_enabled: bool,
    actor_runtime_id: u64,
    username: String,
    xuid: String,
    player_uuid: [u8; 16],
    player_skin_data: Vec<u8>,
    client_cache_enabled: Option<bool>,
}

impl Session {
    fn new() -> Self {
        Self {
            phase: Phase::SessionStart,
            compression_enabled: false,
            actor_runtime_id: 1,
            username: "Player".to_string(),
            xuid: String::new(),
            player_uuid: [0u8; 16],
            player_skin_data: packets::player_list::build_minimal_skin(),
            client_cache_enabled: None,
        }
    }
}

const DEFAULT_SPAWN_BLOCK_POS: [i32; 3] = [256, 70, 256];
const DEFAULT_PLAYER_POS: [f32; 3] = [258.619, 67.621, 258.7035];
const DEFAULT_CHUNK_PUBLISHER_BLOCK_POS: [i32; 3] = [258, 66, 258];
const DEFAULT_WORLD_TIME: i32 = 38904;
const DEFAULT_PLAYER_PITCH: f32 = f32::from_bits(0x41E8D3A0); // A0 D3 E8 41
const DEFAULT_PLAYER_YAW: f32 = f32::from_bits(0x43993C3F); // 3F 3C 99 43

pub fn on_disconnect(addr: SocketAddr) {
    get_or_init();
    let mut guard = sessions();
    if let Some(map) = guard.as_mut() {
        map.remove(&addr);
    }
}

pub async fn handle_packet(
    addr: SocketAddr,
    payload: &[u8],
    handle: &ServerHandle,
) -> Result<(), String> {
    get_or_init();

    let compression_enabled = {
        let mut guard = sessions();
        let map = guard.as_mut().unwrap();
        let session = map.entry(addr).or_insert_with(Session::new);
        session.compression_enabled
    };

    let sub_packets = batch::decode_batch(payload, compression_enabled)?;

    for sub_pkt in sub_packets {
        let mut cursor = Cursor::new(&sub_pkt[..]);
        let raw_id = read_unsigned_varint32(&mut cursor).map_err(|e| e.to_string())?;
        let packet_id = raw_id & 0x3FF; // lower 10 bits = packet ID
        let body = &sub_pkt[cursor.position() as usize..];

        handle_sub_packet(addr, packet_id, body, handle).await?;
    }

    Ok(())
}

async fn handle_sub_packet(
    addr: SocketAddr,
    packet_id: u32,
    body: &[u8],
    handle: &ServerHandle,
) -> Result<(), String> {
    let phase = {
        let guard = sessions();
        let map = guard.as_ref().unwrap();
        map.get(&addr)
            .map(|s| s.phase)
            .unwrap_or(Phase::SessionStart)
    };

    debug!(
        "[{addr}] phase={phase:?} packet_id=0x{packet_id:02X} len={}",
        body.len()
    );
    trace_spawn_rx(addr, packet_id, body);

    match (phase, packet_id) {
        // Phase 1: RequestNetworkSettings
        (Phase::SessionStart, ID_REQUEST_NETWORK_SETTINGS) => {
            handle_request_network_settings(addr, body, handle).await
        }

        // Phase 2: Login
        (Phase::Login, ID_LOGIN) => handle_login(addr, body, handle).await,

        // Phase 3: ResourcePackClientResponse
        (Phase::ResourcePacks, ID_RESOURCE_PACK_CLIENT_RESPONSE) => {
            handle_resource_pack_response(addr, body, handle).await
        }

        // Optional client cache capability signal (safe to accept in any pre-game phase).
        (Phase::Login, ID_CLIENT_CACHE_STATUS)
        | (Phase::ResourcePacks, ID_CLIENT_CACHE_STATUS)
        | (Phase::PreSpawn, ID_CLIENT_CACHE_STATUS)
        | (Phase::WaitingForChunkRadius, ID_CLIENT_CACHE_STATUS)
        | (Phase::WaitingForSpawnResponse, ID_CLIENT_CACHE_STATUS) => {
            handle_client_cache_status(addr, body).await
        }

        // Phase 5: RequestChunkRadius (during pre-spawn or later)
        (Phase::WaitingForChunkRadius, ID_REQUEST_CHUNK_RADIUS)
        | (Phase::PreSpawn, ID_REQUEST_CHUNK_RADIUS)
        | (Phase::WaitingForSpawnResponse, ID_REQUEST_CHUNK_RADIUS) => {
            handle_request_chunk_radius(addr, body, handle).await
        }

        // Phase 6: SetLocalPlayerAsInitialized
        (Phase::WaitingForSpawnResponse, ID_SET_LOCAL_PLAYER_AS_INITIALIZED) => {
            handle_set_local_player_as_initialized(addr, body, handle).await
        }

        // Diagnostic only. PMMP does not use this as a spawn success signal.
        (Phase::WaitingForSpawnResponse, ID_SERVERBOUND_LOADING_SCREEN) => {
            handle_serverbound_loading_screen(addr, body).await
        }

        // Silently ignore in other phases.
        (_, ID_SERVERBOUND_LOADING_SCREEN) => {
            debug!("[{addr}] Ignoring ServerboundLoadingScreen in phase {phase:?}");
            Ok(())
        }

        _ => {
            debug!("[{addr}] Unhandled packet 0x{packet_id:02X} in phase {phase:?}");
            Ok(())
        }
    }
}

// --- Phase 1: SessionStart ---
async fn handle_request_network_settings(
    addr: SocketAddr,
    body: &[u8],
    handle: &ServerHandle,
) -> Result<(), String> {
    if body.len() < 4 {
        return Err("RequestNetworkSettings too short".into());
    }
    let protocol = u32::from_be_bytes([body[0], body[1], body[2], body[3]]);
    info!("[{addr}] RequestNetworkSettings protocol={protocol}");

    // Send NetworkSettings (not compressed yet)
    let ns_body = packets::network_settings::encode_default();
    send_packet(addr, ID_NETWORK_SETTINGS, &ns_body, false, handle).await;

    // Enable compression for all subsequent packets
    {
        let mut guard = sessions();
        let map = guard.as_mut().unwrap();
        let session = map.get_mut(&addr).unwrap();
        session.compression_enabled = true;
        session.phase = Phase::Login;
    }

    info!("[{addr}] → NetworkSettings sent, compression enabled, waiting for Login");
    Ok(())
}

// --- Phase 2: Login ---
async fn handle_login(addr: SocketAddr, body: &[u8], handle: &ServerHandle) -> Result<(), String> {
    info!("[{addr}] Login received (len={})", body.len());

    let identity = extract_login_identity(body);
    let skin_data = extract_login_client_skin_data(body);

    if let Ok(identity) = identity {
        info!(
            "[{addr}] Login identity username='{}' uuid={} xuid='{}'",
            identity.username,
            identity.identity,
            identity.xuid
        );
        let mut guard = sessions();
        let map = guard.as_mut().unwrap();
        if let Some(session) = map.get_mut(&addr) {
            session.username = identity.username;
            session.xuid = identity.xuid;
            session.player_uuid = identity.uuid_bytes;
            session.player_skin_data = match skin_data {
                Ok(data) => data,
                Err(err) => {
                    warn!(
                        "[{addr}] Client skin parse failed ({err}), using minimal fallback skin"
                    );
                    packets::player_list::build_minimal_skin()
                }
            };
        }
    } else {
        warn!("[{addr}] Login identity parse failed, using fallback profile");
    }

    // Send PlayStatus(LOGIN_SUCCESS)
    let ps = packets::play_status::encode(packets::play_status::LOGIN_SUCCESS);
    send_packet(addr, ID_PLAY_STATUS, &ps, true, handle).await;

    // Send ResourcePacksInfo
    let rpi = packets::resource_packs_info::encode_empty();
    send_packet(addr, ID_RESOURCE_PACKS_INFO, &rpi, true, handle).await;

    {
        let mut guard = sessions();
        let map = guard.as_mut().unwrap();
        let session = map.get_mut(&addr).unwrap();
        session.phase = Phase::ResourcePacks;
    }

    info!("[{addr}] → PlayStatus(LOGIN_SUCCESS) + ResourcePacksInfo sent");
    Ok(())
}

// --- Phase 3: ResourcePacks ---
async fn handle_resource_pack_response(
    addr: SocketAddr,
    body: &[u8],
    handle: &ServerHandle,
) -> Result<(), String> {
    if body.is_empty() {
        return Err("ResourcePackClientResponse empty".into());
    }
    let status = body[0];
    info!("[{addr}] ResourcePackClientResponse status={status}");

    match status {
        3 => {
            // HAVE_ALL_PACKS → send ResourcePackStack
            let rps = packets::resource_pack_stack::encode_empty();
            send_packet(addr, ID_RESOURCE_PACK_STACK, &rps, true, handle).await;
            info!("[{addr}] → ResourcePackStack sent");
        }
        4 => {
            // COMPLETED → begin spawn sequence
            info!("[{addr}] Resource packs completed, beginning spawn sequence");
            begin_spawn_sequence(addr, handle).await?;
        }
        _ => {
            warn!("[{addr}] Unexpected resource pack status: {status}");
        }
    }

    Ok(())
}

async fn handle_client_cache_status(addr: SocketAddr, body: &[u8]) -> Result<(), String> {
    if body.is_empty() {
        return Err("ClientCacheStatus empty".into());
    }
    let enabled = body[0] != 0;
    {
        let mut guard = sessions();
        let map = guard.as_mut().unwrap();
        if let Some(session) = map.get_mut(&addr) {
            session.client_cache_enabled = Some(enabled);
        }
    }
    info!("[{addr}] ClientCacheStatus enabled={enabled}");
    Ok(())
}

// --- Phase 4: Pre-Spawn (send all game data packets) ---
async fn begin_spawn_sequence(addr: SocketAddr, handle: &ServerHandle) -> Result<(), String> {
    let (actor_runtime_id, username, xuid, player_uuid, player_skin_data) = {
        let mut guard = sessions();
        let map = guard.as_mut().unwrap();
        let session = map.get_mut(&addr).unwrap();
        session.phase = Phase::PreSpawn;
        (
            session.actor_runtime_id,
            session.username.clone(),
            session.xuid.clone(),
            session.player_uuid,
            session.player_skin_data.clone(),
        )
    };

    let spawn_pos = DEFAULT_SPAWN_BLOCK_POS;
    let player_pos = DEFAULT_PLAYER_POS;

    // 1. StartGame
    let sg = packets::start_game::encode(
        actor_runtime_id as i64,
        actor_runtime_id,
        0, // Survival
        player_pos,
        DEFAULT_PLAYER_PITCH,
        DEFAULT_PLAYER_YAW,
        DEFAULT_WORLD_TIME,
        spawn_pos,
        "MC-RS Server",
    );
    send_packet(addr, ID_START_GAME, &sg, true, handle).await;

    // PMMP onEnterWorld parity.
    let set_time = packets::set_time::encode(DEFAULT_WORLD_TIME);
    send_packet(addr, ID_SET_TIME, &set_time, true, handle).await;

    let set_difficulty = packets::set_difficulty::encode(2); // normal
    send_packet(addr, ID_SET_DIFFICULTY, &set_difficulty, true, handle).await;

    let player_spawn = packets::set_spawn_position::encode_player_spawn(
        spawn_pos[0],
        spawn_pos[1] as u32,
        spawn_pos[2],
        0,
    );
    send_packet(addr, ID_SET_SPAWN_POSITION, &player_spawn, true, handle).await;

    let world_spawn = packets::set_spawn_position::encode_world_spawn(
        spawn_pos[0],
        spawn_pos[1] as u32,
        spawn_pos[2],
        0,
    );
    send_packet(addr, ID_SET_SPAWN_POSITION, &world_spawn, true, handle).await;

    // 2. ItemRegistry (PMMP required_item_list.json parity)
    let ir = packets::item_registry::encode_full();
    send_packet(addr, ID_ITEM_REGISTRY, &ir, true, handle).await;

    // 3. AvailableActorIdentifiers
    let aai = packets::available_actor_identifiers::encode();
    send_packet(addr, ID_AVAILABLE_ACTOR_IDENTIFIERS, &aai, true, handle).await;

    // 4. BiomeDefinitionList
    let bdl = packets::biome_definition_list::encode();
    send_packet(addr, ID_BIOME_DEFINITION_LIST, &bdl, true, handle).await;

    // 5. UpdateAttributes
    let ua = packets::update_attributes::encode_default_player(actor_runtime_id);
    send_packet(addr, ID_UPDATE_ATTRIBUTES, &ua, true, handle).await;

    // 6. AvailableCommands (empty)
    let ac = packets::available_commands::encode_empty();
    send_packet(addr, ID_AVAILABLE_COMMANDS, &ac, true, handle).await;

    // 7. SetPlayerGameType
    let spgt = packets::set_player_game_type::encode(0); // survival
    send_packet(addr, ID_SET_PLAYER_GAME_TYPE, &spgt, true, handle).await;

    // 8. UpdateAbilities
    let uab = packets::update_abilities::encode_default_survival(actor_runtime_id as i64);
    send_packet(addr, ID_UPDATE_ABILITIES, &uab, true, handle).await;

    // 9. UpdateAdventureSettings
    let uas = packets::update_adventure_settings::encode_default();
    send_packet(addr, ID_UPDATE_ADVENTURE_SETTINGS, &uas, true, handle).await;

    // 10. SetActorData
    let sad = packets::set_actor_data::encode_player_default(actor_runtime_id);
    send_packet(addr, ID_SET_ACTOR_DATA, &sad, true, handle).await;

    // 11. CreativeContent (empty)
    let cc = packets::creative_content::encode_empty();
    send_packet(addr, ID_CREATIVE_CONTENT, &cc, true, handle).await;

    // 12. CraftingData (empty)
    let cd = packets::crafting_data::encode_empty();
    send_packet(addr, ID_CRAFTING_DATA, &cd, true, handle).await;

    // 13. PlayerList (add self)
    let pl = packets::player_list::encode_add(
        &player_uuid,
        actor_runtime_id as i64,
        &username,
        &xuid,
        "",
        -1,
        &player_skin_data,
        true,
        false,
        false,
        0xFFFF_FFFF,
        true,
    );
    send_packet(addr, ID_PLAYER_LIST, &pl, true, handle).await;

    // Now wait for RequestChunkRadius
    {
        let mut guard = sessions();
        let map = guard.as_mut().unwrap();
        let session = map.get_mut(&addr).unwrap();
        session.phase = Phase::WaitingForChunkRadius;
    }

    info!("[{addr}] → All pre-spawn packets sent, waiting for RequestChunkRadius");
    Ok(())
}

// --- Phase 5: Chunk loading ---
async fn handle_request_chunk_radius(
    addr: SocketAddr,
    body: &[u8],
    handle: &ServerHandle,
) -> Result<(), String> {
    let mut cursor = Cursor::new(body);
    let requested_radius = read_signed_varint32(&mut cursor).map_err(|e| e.to_string())?;
    let radius = requested_radius.clamp(2, 16);
    info!("[{addr}] RequestChunkRadius requested={requested_radius}, using={radius}");

    let (phase, actor_runtime_id) = {
        let guard = sessions();
        let map = guard.as_ref().unwrap();
        let session = map
            .get(&addr)
            .ok_or_else(|| "session missing while handling chunk radius".to_string())?;
        (session.phase, session.actor_runtime_id)
    };

    // Send ChunkRadiusUpdated
    let cru = packets::chunk_radius_updated::encode(radius);
    send_packet(addr, ID_CHUNK_RADIUS_UPDATED, &cru, true, handle).await;

    // Send NetworkChunkPublisherUpdate
    let spawn = DEFAULT_CHUNK_PUBLISHER_BLOCK_POS;
    let ncpu = packets::network_chunk_publisher_update::encode(
        spawn[0],
        spawn[1],
        spawn[2],
        (radius as u32) * 16,
    );
    send_packet(addr, ID_NETWORK_CHUNK_PUBLISHER_UPDATE, &ncpu, true, handle).await;

    // During spawn response waiting, the client may repeat RequestChunkRadius.
    // Do not restart the whole spawn stream in this phase, only refresh radius/view info.
    if phase == Phase::WaitingForSpawnResponse {
        info!(
            "[{addr}] RequestChunkRadius while waiting spawn response: radius/view updated, keeping existing spawn handshake"
        );
        return Ok(());
    }

    // Send chunks in the radius
    let chunk_count = send_spawn_chunks(addr, radius, handle).await?;
    info!("[{addr}] Sent {chunk_count} chunks");

    // PMMP sends a movement sync before gameplay starts; do the same to lock local actor state.
    let move_player = packets::move_player::encode(
        actor_runtime_id,
        DEFAULT_PLAYER_POS[0],
        DEFAULT_PLAYER_POS[1],
        DEFAULT_PLAYER_POS[2],
        DEFAULT_PLAYER_PITCH,
        DEFAULT_PLAYER_YAW,
        DEFAULT_PLAYER_YAW,
        packets::move_player::MODE_NORMAL,
        false,
        0,
    );
    send_packet(addr, ID_MOVE_PLAYER, &move_player, true, handle).await;

    // Send PlayStatus(PLAYER_SPAWN)
    let ps = packets::play_status::encode(packets::play_status::PLAYER_SPAWN);
    send_packet(addr, ID_PLAY_STATUS, &ps, true, handle).await;

    {
        let mut guard = sessions();
        let map = guard.as_mut().unwrap();
        let session = map.get_mut(&addr).unwrap();
        session.phase = Phase::WaitingForSpawnResponse;
    }

    info!("[{addr}] → PlayStatus(PLAYER_SPAWN) sent, waiting for SetLocalPlayerAsInitialized");
    Ok(())
}

async fn send_spawn_chunks(
    addr: SocketAddr,
    radius: i32,
    handle: &ServerHandle,
) -> Result<u32, String> {
    let mut count = 0u32;

    let spawn = DEFAULT_SPAWN_BLOCK_POS;
    let center_chunk_x = spawn[0].div_euclid(16);
    let center_chunk_z = spawn[2].div_euclid(16);

    // PMMP typically starts by sending a nearby chunk around the player center first.
    let preferred_first_x = center_chunk_x - 1;
    let preferred_first_z = center_chunk_z;
    if preferred_first_x >= center_chunk_x - radius
        && preferred_first_x <= center_chunk_x + radius
        && preferred_first_z >= center_chunk_z - radius
        && preferred_first_z <= center_chunk_z + radius
    {
        let SpawnChunkPayload {
            payload,
            sub_chunk_count,
        } = build_spawn_chunk_payload(preferred_first_x, preferred_first_z)?;
        let lc = packets::level_chunk::encode(
            preferred_first_x,
            preferred_first_z,
            0,
            sub_chunk_count,
            &payload,
        );
        trace_spawn_tx(addr, ID_LEVEL_CHUNK, &lc, "first");
        send_packet(addr, ID_LEVEL_CHUNK, &lc, true, handle).await;
        count += 1;
    }

    for cz in (center_chunk_z - radius)..=(center_chunk_z + radius) {
        for cx in (center_chunk_x - radius)..=(center_chunk_x + radius) {
            if cx == preferred_first_x && cz == preferred_first_z {
                continue;
            }
            let SpawnChunkPayload {
                payload,
                sub_chunk_count,
            } = build_spawn_chunk_payload(cx, cz)?;
            let lc = packets::level_chunk::encode(cx, cz, 0, sub_chunk_count, &payload);
            if count == 0 {
                trace_spawn_tx(addr, ID_LEVEL_CHUNK, &lc, "first");
            }
            send_packet(addr, ID_LEVEL_CHUNK, &lc, true, handle).await;
            count += 1;
        }
    }

    Ok(count)
}

struct SpawnChunkPayload {
    payload: Vec<u8>,
    sub_chunk_count: u32,
}

/// Builds a PMMP-compatible chunk column:
/// - subchunks from overworld min (-4) up to top non-empty y
/// - lower out-of-world subchunks encoded as v8 with 0 layers
/// - terrain subchunks encoded as runtime paletted storage
/// - all 24 biome sections are sent
/// - border block count is 0 and no tiles are appended
fn build_spawn_chunk_payload(chunk_x: i32, chunk_z: i32) -> Result<SpawnChunkPayload, String> {
    let ids = block_runtime_ids()?;
    let top_non_empty_subchunk_y = 4;
    let sub_chunk_count = (top_non_empty_subchunk_y - OVERWORLD_MIN_SUBCHUNK_Y + 1) as u32;

    let mut buf = Vec::with_capacity(8192);
    for subchunk_y in OVERWORLD_MIN_SUBCHUNK_Y..=top_non_empty_subchunk_y {
        if subchunk_y < 0 {
            // PMMP world format has empty virtual slices below y=0.
            write_empty_subchunk(&mut buf);
        } else {
            write_generated_overworld_subchunk(&mut buf, chunk_x, chunk_z, subchunk_y, ids);
        }
    }

    for _ in 0..OVERWORLD_BIOME_SECTIONS {
        write_single_valued_biome_section(&mut buf, 1);
    }

    // Border block array count (no border blocks) and no tiles.
    buf.push(0x00);

    Ok(SpawnChunkPayload {
        payload: buf,
        sub_chunk_count,
    })
}

fn write_empty_subchunk(buf: &mut Vec<u8>) {
    buf.push(8); // version
    buf.push(0); // num_layers
}

fn write_generated_overworld_subchunk(
    buf: &mut Vec<u8>,
    chunk_x: i32,
    chunk_z: i32,
    subchunk_y: i32,
    ids: BlockRuntimeIds,
) {
    let runtime_ids = generate_pmmp_like_runtime_ids(chunk_x, chunk_z, subchunk_y, ids);
    write_runtime_paletted_subchunk(buf, &runtime_ids);
}

fn generate_pmmp_like_runtime_ids(
    chunk_x: i32,
    chunk_z: i32,
    subchunk_y: i32,
    ids: BlockRuntimeIds,
) -> [i32; 4096] {
    // Observed PMMP first-chunk runtime palettes (protocol 924, overworld spawn area).
    // We intentionally mirror these palette cardinalities to match chunk stream structure.
    let (palette, weights): (Vec<i32>, Vec<u32>) = match subchunk_y {
        0 => (
            vec![ids.stone, ids.bedrock, 7336, 6356, 3203, 6501, ids.dirt, 15806],
            vec![68, 2, 8, 7, 4, 4, 6, 1],
        ),
        1 => (
            vec![ids.stone, 7336, 6356, ids.dirt, 15806],
            vec![72, 10, 8, 8, 2],
        ),
        2 => (
            vec![ids.stone, 6318, 7336, ids.dirt],
            vec![74, 8, 8, 10],
        ),
        3 => (
            vec![ids.stone, ids.dirt, ids.grass_block, 6318, 7336],
            vec![42, 24, 8, 14, 12],
        ),
        4 => (
            vec![ids.dirt, ids.grass_block, ids.air, 12421],
            vec![35, 10, 52, 3],
        ),
        _ => (vec![ids.air], vec![1]),
    };

    let mut runtime_ids = [palette[0]; 4096];
    if palette.len() == 1 {
        return runtime_ids;
    }

    // Seed first occurrences in deterministic order to lock palette ordering.
    for (i, rid) in palette.iter().enumerate() {
        runtime_ids[i] = *rid;
    }

    let total_weight: u32 = weights.iter().copied().sum::<u32>().max(1);
    for idx in palette.len()..4096 {
        let hash = mix_u64(
            (chunk_x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (chunk_z as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
                ^ (subchunk_y as u64).wrapping_mul(0x1656_67B1_9E37_79F9)
                ^ (idx as u64).wrapping_mul(0x85EB_CA6B_27D4_EB2F),
        );
        let mut ticket = (hash as u32) % total_weight;
        let mut pick = 0usize;
        for (i, w) in weights.iter().enumerate() {
            if ticket < *w {
                pick = i;
                break;
            }
            ticket -= *w;
        }
        runtime_ids[idx] = palette[pick];
    }

    runtime_ids
}

fn mix_u64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn write_runtime_paletted_subchunk(buf: &mut Vec<u8>, runtime_ids: &[i32; 4096]) {
    let mut palette = Vec::<i32>::new();
    let mut palette_indices = [0u16; 4096];

    for (i, runtime_id) in runtime_ids.iter().enumerate() {
        if let Some(existing) = palette.iter().position(|v| v == runtime_id) {
            palette_indices[i] = existing as u16;
        } else {
            palette.push(*runtime_id);
            palette_indices[i] = (palette.len() - 1) as u16;
        }
    }

    let bpb = bits_per_block_for_palette(palette.len());

    buf.push(8); // version
    buf.push(1); // num_layers
    buf.push((bpb << 1) | 1); // runtime IDs

    if bpb != 0 {
        let words = pack_palette_indices(&palette_indices, bpb as usize);
        for word in words {
            buf.extend_from_slice(&word.to_le_bytes());
        }
        write_zigzag_varint(buf, palette.len() as i32);
    }

    for runtime_id in palette {
        write_zigzag_varint(buf, runtime_id);
    }
}

fn bits_per_block_for_palette(palette_size: usize) -> u8 {
    match palette_size {
        0..=1 => 0,
        2 => 1,
        3..=4 => 2,
        5..=8 => 3,
        9..=16 => 4,
        17..=32 => 5,
        33..=64 => 6,
        65..=256 => 8,
        _ => 16,
    }
}

fn pack_palette_indices(indices: &[u16; 4096], bits_per_block: usize) -> Vec<u32> {
    if bits_per_block == 0 {
        return Vec::new();
    }
    let blocks_per_word = 32 / bits_per_block;
    let word_count = 4096_usize.div_ceil(blocks_per_word);
    let mut words = vec![0u32; word_count];

    // PMMP chunkutils layout: indices are packed in fixed-size slots per 32-bit word,
    // with no cross-word carry when bpb doesn't divide 32 (padding bits at word tail).
    for word_idx in 0..word_count {
        let mut word = 0u32;
        for slot in 0..blocks_per_word {
            let block_index = word_idx * blocks_per_word + slot;
            if block_index < 4096 {
                let palette_index = indices[block_index] as u32;
                word |= palette_index << (bits_per_block as u32 * slot as u32);
            }
        }
        words[word_idx] = word;
    }

    words
}

/// PMMP-compatible single-valued biome palette section.
/// Uses bpb=0 (single-valued palette) like PMMP when the biome section is uniform.
fn write_single_valued_biome_section(buf: &mut Vec<u8>, biome_id: i32) {
    // Header: (0 << 1) | 1 = 0x01
    buf.push(0x01);
    // No words for bpb=0, no palette count field.
    // Single palette entry
    write_zigzag_varint(buf, biome_id);
}

fn write_zigzag_varint(buf: &mut Vec<u8>, value: i32) {
    let mut zigzag = ((value << 1) ^ (value >> 31)) as u32;
    loop {
        if zigzag & !0x7F == 0 {
            buf.push(zigzag as u8);
            return;
        }
        buf.push((zigzag & 0x7F | 0x80) as u8);
        zigzag >>= 7;
    }
}

// --- Phase 6: Spawn response ---
async fn handle_set_local_player_as_initialized(
    addr: SocketAddr,
    _body: &[u8],
    _handle: &ServerHandle,
) -> Result<(), String> {
    info!("[{addr}] SetLocalPlayerAsInitialized received! Player is in-game!");

    {
        let mut guard = sessions();
        let map = guard.as_mut().unwrap();
        let session = map.get_mut(&addr).unwrap();
        session.phase = Phase::InGame;
    }

    info!("[{addr}] *** CONNECTION SUCCESSFUL — Player spawned in world! ***");
    Ok(())
}

/// Diagnostic signal only.
/// Packet body starts with SignedVarInt loading_screen_type.
async fn handle_serverbound_loading_screen(addr: SocketAddr, body: &[u8]) -> Result<(), String> {
    let mut cursor = Cursor::new(body);
    let loading_screen_type = read_signed_varint32(&mut cursor).map_err(|e| e.to_string())?;

    info!(
        "[{addr}] ServerboundLoadingScreen(type={loading_screen_type}) received (diagnostic only, waiting for SetLocalPlayerAsInitialized)"
    );

    Ok(())
}

fn trace_spawn_enabled() -> bool {
    *TRACE_SPAWN_ENABLED.get_or_init(|| {
        std::env::var("MC_RS_TRACE_SPAWN")
            .ok()
            .map(|v| {
                let v = v.trim();
                v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
            })
            .unwrap_or(false)
    })
}

fn trace_spawn_rx(addr: SocketAddr, packet_id: u32, body: &[u8]) {
    if !trace_spawn_enabled() {
        return;
    }
    let track = matches!(
        packet_id,
        ID_REQUEST_CHUNK_RADIUS
            | ID_SERVERBOUND_LOADING_SCREEN
            | ID_SET_LOCAL_PLAYER_AS_INITIALIZED
            | ID_CLIENT_CACHE_STATUS
    );
    if !track {
        return;
    }
    info!(
        "[{addr}] [TRACE_SPAWN RX] id=0x{packet_id:02X} len={} hex={}",
        body.len(),
        hex_preview(body, 64)
    );
}

fn trace_spawn_tx(addr: SocketAddr, packet_id: u32, body: &[u8], note: &str) {
    if !trace_spawn_enabled() {
        return;
    }
    let track = matches!(
        packet_id,
        ID_START_GAME
            | ID_UPDATE_ABILITIES
            | ID_PLAYER_LIST
            | ID_NETWORK_CHUNK_PUBLISHER_UPDATE
            | ID_LEVEL_CHUNK
    );
    if !track {
        return;
    }

    if let Err(err) = dump_trace_packet_if_needed(addr, packet_id, body, note) {
        warn!("[{addr}] Failed to dump trace packet id=0x{packet_id:02X}: {err}");
    }

    if note.is_empty() {
        info!(
            "[{addr}] [TRACE_SPAWN TX] id=0x{packet_id:02X} len={} hex={}",
            body.len(),
            hex_preview(body, 64)
        );
    } else {
        info!(
            "[{addr}] [TRACE_SPAWN TX] id=0x{packet_id:02X} ({note}) len={} hex={}",
            body.len(),
            hex_preview(body, 64)
        );
    }
}

fn dump_trace_packet_if_needed(
    addr: SocketAddr,
    packet_id: u32,
    body: &[u8],
    note: &str,
) -> Result<(), String> {
    let kind = match packet_id {
        ID_START_GAME => Some("start_game"),
        ID_UPDATE_ABILITIES => Some("update_abilities"),
        ID_PLAYER_LIST => Some("player_list"),
        ID_NETWORK_CHUNK_PUBLISHER_UPDATE => Some("network_chunk_publisher_update"),
        ID_LEVEL_CHUNK if note == "first" => Some("level_chunk_first"),
        _ => None,
    };

    let Some(kind) = kind else {
        return Ok(());
    };

    let key = format!("{addr}_{kind}");
    let dumped = TRACE_DUMPED_KEYS.get_or_init(|| Mutex::new(HashSet::new()));
    {
        let mut guard = dumped
            .lock()
            .map_err(|_| "TRACE_DUMPED_KEYS mutex poisoned".to_string())?;
        if !guard.insert(key) {
            return Ok(());
        }
    }

    let dump_dir = trace_dump_dir();
    fs::create_dir_all(&dump_dir)
        .map_err(|e| format!("create_dir_all({}): {e}", dump_dir.display()))?;

    let seq = TRACE_DUMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let file_name = format!(
        "{seq:03}_{kind}_{}_id_{packet_id:03X}_len_{}.bin",
        sanitize_for_file(addr.to_string()),
        body.len()
    );
    let path = dump_dir.join(file_name);

    fs::write(&path, body).map_err(|e| format!("write({}): {e}", path.display()))?;

    info!(
        "[{addr}] [TRACE_SPAWN DUMP] id=0x{packet_id:02X} kind={kind} path={}",
        path.display()
    );

    Ok(())
}

fn trace_dump_dir() -> PathBuf {
    Path::new(".reference").join("dumps").join("mc_rs")
}

fn sanitize_for_file(raw: String) -> String {
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn hex_preview(bytes: &[u8], max_len: usize) -> String {
    let shown = bytes.len().min(max_len);
    let mut out = String::with_capacity(shown * 3 + 16);
    for (i, b) in bytes[..shown].iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02X}");
    }
    if bytes.len() > shown {
        out.push_str(" ...");
    }
    out
}

#[derive(Debug, Clone)]
struct LoginIdentity {
    username: String,
    xuid: String,
    identity: String,
    uuid_bytes: [u8; 16],
}

fn extract_login_identity(body: &[u8]) -> Result<LoginIdentity, String> {
    let mut cursor = Cursor::new(body);

    if cursor.remaining() < 4 {
        return Err("Login body too short for protocol".into());
    }
    let _protocol_version = cursor.get_i32();

    let payload_len =
        read_unsigned_varint32(&mut cursor).map_err(|e| format!("login payload len: {e}"))?
            as usize;
    if cursor.remaining() < payload_len {
        return Err(format!(
            "Login payload truncated: need {payload_len}, have {}",
            cursor.remaining()
        ));
    }

    let payload_start = cursor.position() as usize;
    let payload_end = payload_start + payload_len;
    let payload = &body[payload_start..payload_end];
    let mut payload_cursor = Cursor::new(payload);

    if payload_cursor.remaining() < 4 {
        return Err("Login payload too short for chain length".into());
    }
    let chain_len_i32 = payload_cursor.get_i32_le();
    if chain_len_i32 < 0 {
        return Err("Login chain length is negative".into());
    }
    let chain_len = chain_len_i32 as usize;
    if payload_cursor.remaining() < chain_len {
        return Err(format!(
            "Login chain truncated: need {chain_len}, have {}",
            payload_cursor.remaining()
        ));
    }
    let chain_start = payload_cursor.position() as usize;
    let chain_end = chain_start + chain_len;
    let chain_bytes = &payload[chain_start..chain_end];

    let chain_json: Value =
        serde_json::from_slice(chain_bytes).map_err(|e| format!("login chain json: {e}"))?;

    let chain_container = if let Some(cert) = chain_json.get("Certificate").and_then(Value::as_str)
    {
        serde_json::from_str::<Value>(cert).map_err(|e| format!("certificate json: {e}"))?
    } else {
        chain_json
    };

    let chain = chain_container
        .get("chain")
        .and_then(Value::as_array)
        .ok_or_else(|| "login chain does not contain `chain` array".to_string())?;

    for link in chain.iter().rev() {
        let Some(jwt) = link.as_str() else {
            continue;
        };
        let payload = decode_jwt_payload(jwt)?;
        let Some(extra) = payload.get("extraData").and_then(Value::as_object) else {
            continue;
        };

        let identity = extra
            .get("identity")
            .and_then(Value::as_str)
            .ok_or_else(|| "extraData.identity missing".to_string())?
            .to_string();
        let username = extra
            .get("displayName")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .unwrap_or("Player")
            .to_string();
        let xuid = extra
            .get("XUID")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let uuid_bytes = parse_uuid_bytes(&identity).unwrap_or([0u8; 16]);

        return Ok(LoginIdentity {
            username,
            xuid,
            identity,
            uuid_bytes,
        });
    }

    Err("no identity link found in login chain".into())
}

fn decode_jwt_payload(jwt: &str) -> Result<Value, String> {
    let mut parts = jwt.splitn(3, '.');
    let _header = parts.next().ok_or_else(|| "jwt missing header".to_string())?;
    let payload = parts.next().ok_or_else(|| "jwt missing payload".to_string())?;
    let _signature = parts.next().ok_or_else(|| "jwt missing signature".to_string())?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| URL_SAFE.decode(payload))
        .map_err(|e| format!("jwt payload base64: {e}"))?;

    serde_json::from_slice(&payload_bytes).map_err(|e| format!("jwt payload json: {e}"))
}

fn extract_login_client_skin_data(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(body);
    if cursor.remaining() < 4 {
        return Err("Login body too short for protocol".into());
    }
    let _protocol_version = cursor.get_i32();

    let payload_len =
        read_unsigned_varint32(&mut cursor).map_err(|e| format!("login payload len: {e}"))?
            as usize;
    if cursor.remaining() < payload_len {
        return Err(format!(
            "Login payload truncated: need {payload_len}, have {}",
            cursor.remaining()
        ));
    }

    let payload_start = cursor.position() as usize;
    let payload_end = payload_start + payload_len;
    let payload = &body[payload_start..payload_end];
    let mut payload_cursor = Cursor::new(payload);

    if payload_cursor.remaining() < 4 {
        return Err("Login payload too short for chain length".into());
    }
    let chain_len = payload_cursor.get_i32_le();
    if chain_len < 0 {
        return Err("Login chain length is negative".into());
    }
    let chain_len = chain_len as usize;
    if payload_cursor.remaining() < chain_len {
        return Err(format!(
            "Login chain truncated: need {chain_len}, have {}",
            payload_cursor.remaining()
        ));
    }
    payload_cursor.advance(chain_len);

    if payload_cursor.remaining() < 4 {
        return Err("Login payload too short for client_data length".into());
    }
    let client_data_len = payload_cursor.get_i32_le();
    if client_data_len < 0 {
        return Err("Login client_data length is negative".into());
    }
    let client_data_len = client_data_len as usize;
    if payload_cursor.remaining() < client_data_len {
        return Err(format!(
            "Login client_data truncated: need {client_data_len}, have {}",
            payload_cursor.remaining()
        ));
    }

    let start = payload_cursor.position() as usize;
    let end = start + client_data_len;
    let client_data_jwt = std::str::from_utf8(&payload[start..end])
        .map_err(|e| format!("client_data jwt utf8: {e}"))?;
    let client_payload = decode_jwt_payload(client_data_jwt)?;
    Ok(build_skin_data_from_client_payload(&client_payload))
}

fn build_skin_data_from_client_payload(client_payload: &Value) -> Vec<u8> {
    // Mirror PMMP's login pipeline:
    // ClientData -> SkinData (decoded base64) -> Skin -> LegacySkinAdapter::toSkinData().
    if json_bool(client_payload, "PersonaSkin", false) {
        return packets::player_list::build_minimal_skin();
    }

    let mut skin_id = json_string(client_payload, "SkinId");
    if skin_id.is_empty() {
        skin_id = "Standard_Custom".to_string();
    }

    let skin_pixels = decode_base64_standard(json_string(client_payload, "SkinData").as_str());
    let Some((skin_image_width, skin_image_height)) = legacy_skin_dimensions(skin_pixels.len()) else {
        return packets::player_list::build_minimal_skin();
    };

    let skin_resource_patch_raw =
        decode_base64_standard_to_string(json_string(client_payload, "SkinResourcePatch").as_str());
    let geometry_name = extract_geometry_name(&skin_resource_patch_raw)
        .unwrap_or_else(|| "geometry.humanoid.custom".to_string());
    let skin_resource_patch = serde_json::json!({
        "geometry": { "default": geometry_name }
    })
    .to_string();

    let skin_geometry_data_raw =
        decode_base64_standard_to_string(json_string(client_payload, "SkinGeometryData").as_str());
    let skin_geometry_data = minify_json_or_passthrough(&skin_geometry_data_raw);

    let cape_pixels = if json_bool(client_payload, "CapeOnClassicSkin", false) {
        Vec::new()
    } else {
        let decoded = decode_base64_standard(json_string(client_payload, "CapeData").as_str());
        if decoded.len() == 8192 {
            decoded
        } else {
            Vec::new()
        }
    };

    let (cape_image_width, cape_image_height) = if cape_pixels.is_empty() {
        (0u32, 0u32)
    } else {
        (64u32, 32u32)
    };

    let full_skin_id = random_uuid_v4_string();

    let mut buf = Vec::new();
    write_string_vec(&mut buf, &skin_id);
    write_string_vec(&mut buf, ""); // playFabId (PMMP legacy adapter leaves empty)
    write_string_vec(&mut buf, &skin_resource_patch);

    // Skin image (legacy dimensions derived from bytes length).
    buf.extend_from_slice(&skin_image_width.to_le_bytes());
    buf.extend_from_slice(&skin_image_height.to_le_bytes());
    write_unsigned_varint32_vec(&mut buf, skin_pixels.len() as u32);
    buf.extend_from_slice(&skin_pixels);

    // Animations: none (legacy adapter output).
    buf.extend_from_slice(&0u32.to_le_bytes());

    // Cape image
    buf.extend_from_slice(&cape_image_width.to_le_bytes());
    buf.extend_from_slice(&cape_image_height.to_le_bytes());
    write_unsigned_varint32_vec(&mut buf, cape_pixels.len() as u32);
    buf.extend_from_slice(&cape_pixels);

    write_string_vec(&mut buf, &skin_geometry_data);
    write_string_vec(&mut buf, "1.26.0"); // ProtocolInfo::MINECRAFT_VERSION_NETWORK in PMMP
    write_string_vec(&mut buf, ""); // animationData
    write_string_vec(&mut buf, ""); // capeId
    write_string_vec(&mut buf, &full_skin_id);
    write_string_vec(&mut buf, "wide");
    write_string_vec(&mut buf, "");

    // Persona pieces and piece tint colors: none.
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());

    // premium, persona, capeOnClassic, isPrimaryUser, override
    buf.push(0);
    buf.push(0);
    buf.push(0);
    buf.push(1);
    buf.push(1);

    buf
}

fn json_string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn json_bool(value: &Value, key: &str, default: bool) -> bool {
    value
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn decode_base64_standard(input: &str) -> Vec<u8> {
    STANDARD_NO_PAD
        .decode(input)
        .or_else(|_| STANDARD.decode(input))
        .unwrap_or_default()
}

fn decode_base64_standard_to_string(input: &str) -> String {
    String::from_utf8_lossy(&decode_base64_standard(input)).to_string()
}

fn extract_geometry_name(resource_patch_json: &str) -> Option<String> {
    let value: Value = serde_json::from_str(resource_patch_json).ok()?;
    value
        .get("geometry")?
        .get("default")?
        .as_str()
        .map(|s| s.to_string())
}

fn minify_json_or_passthrough(raw: &str) -> String {
    match serde_json::from_str::<Value>(raw) {
        Ok(value) => value.to_string(),
        Err(_) => raw.to_string(),
    }
}

fn legacy_skin_dimensions(byte_len: usize) -> Option<(u32, u32)> {
    match byte_len {
        8192 => Some((64, 32)),
        16384 => Some((64, 64)),
        65536 => Some((128, 128)),
        _ => None,
    }
}

fn random_uuid_v4_string() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill(&mut bytes);
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    )
}

fn write_string_vec(buf: &mut Vec<u8>, s: &str) {
    write_unsigned_varint32_vec(buf, s.len() as u32);
    buf.extend_from_slice(s.as_bytes());
}

fn write_unsigned_varint32_vec(buf: &mut Vec<u8>, mut v: u32) {
    loop {
        if v & !0x7F == 0 {
            buf.push(v as u8);
            return;
        }
        buf.push((v & 0x7F | 0x80) as u8);
        v >>= 7;
    }
}

fn parse_uuid_bytes(raw: &str) -> Option<[u8; 16]> {
    let hex: Vec<u8> = raw
        .bytes()
        .filter(|b| *b != b'-')
        .map(|b| b.to_ascii_lowercase())
        .collect();
    if hex.len() != 32 {
        return None;
    }

    let mut out = [0u8; 16];
    for i in 0..16 {
        let hi = hex_nibble(hex[i * 2])?;
        let lo = hex_nibble(hex[i * 2 + 1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_nibble(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        b'a'..=b'f' => Some(10 + (ch - b'a')),
        _ => None,
    }
}

fn block_runtime_ids() -> Result<BlockRuntimeIds, String> {
    let cached = BLOCK_RUNTIME_IDS
        .get_or_init(load_block_runtime_ids)
        .clone();
    match cached {
        Ok(ids) => Ok(ids),
        Err(err) => {
            warn!("Failed to parse canonical_block_states.nbt ({err}); using fallback runtime IDs");
            Ok(BlockRuntimeIds {
                air: 0,
                bedrock: 1,
                stone: 1,
                dirt: 1,
                grass_block: 1,
                water: 0,
            })
        }
    }
}

fn load_block_runtime_ids() -> Result<BlockRuntimeIds, String> {
    let bytes = include_bytes!("../data/canonical_block_states.nbt");
    let names = parse_canonical_block_state_names(bytes)?;

    let air = first_runtime_id(&names, "minecraft:air")
        .ok_or_else(|| "minecraft:air not found in canonical block states".to_string())?;
    let bedrock = first_runtime_id(&names, "minecraft:bedrock")
        .ok_or_else(|| "minecraft:bedrock not found in canonical block states".to_string())?;
    let stone = first_runtime_id(&names, "minecraft:stone")
        .ok_or_else(|| "minecraft:stone not found in canonical block states".to_string())?;
    let dirt = first_runtime_id(&names, "minecraft:dirt")
        .ok_or_else(|| "minecraft:dirt not found in canonical block states".to_string())?;
    let grass_block = first_runtime_id(&names, "minecraft:grass_block")
        .or_else(|| first_runtime_id(&names, "minecraft:grass"))
        .ok_or_else(|| "minecraft:grass_block not found in canonical block states".to_string())?;
    let water = first_runtime_id(&names, "minecraft:water")
        .ok_or_else(|| "minecraft:water not found in canonical block states".to_string())?;

    info!(
        "Resolved runtime IDs from canonical palette: air={air}, bedrock={bedrock}, stone={stone}, dirt={dirt}, grass_block={grass_block}, water={water}, entries={}",
        names.len(),
    );

    Ok(BlockRuntimeIds {
        air,
        bedrock,
        stone,
        dirt,
        grass_block,
        water,
    })
}

fn first_runtime_id(names: &[String], name: &str) -> Option<i32> {
    names.iter().position(|n| n == name).map(|idx| idx as i32)
}

fn parse_canonical_block_state_names(data: &[u8]) -> Result<Vec<String>, String> {
    let mut offset = 0usize;
    let mut names = Vec::new();
    while offset < data.len() {
        if data[offset] == 0 {
            offset += 1;
            continue;
        }
        names.push(parse_root_compound_block_name(data, &mut offset)?);
    }
    if names.is_empty() {
        return Err("canonical block states file is empty".into());
    }
    Ok(names)
}

fn parse_root_compound_block_name(data: &[u8], offset: &mut usize) -> Result<String, String> {
    let tag = read_u8(data, offset)?;
    if tag != 10 {
        return Err(format!(
            "expected TAG_Compound(10) at offset {:#X}, got {tag}",
            offset.saturating_sub(1)
        ));
    }

    // Bedrock network NBT: string lengths are VarUInt32.
    let _root_name = read_string_network(data, offset)?;

    let mut block_name = None::<String>;
    loop {
        let tag_id = read_u8(data, offset)?;
        if tag_id == 0 {
            break;
        }
        let name = read_string_network(data, offset)?;
        if tag_id == 8 && name == "name" {
            block_name = Some(read_string_network(data, offset)?);
        } else {
            skip_nbt_payload(data, offset, tag_id)?;
        }
    }

    block_name.ok_or_else(|| "block state compound is missing `name`".to_string())
}

fn skip_nbt_payload(data: &[u8], offset: &mut usize, tag_id: u8) -> Result<(), String> {
    match tag_id {
        1 => skip_bytes(data, offset, 1), // byte
        2 => skip_bytes(data, offset, 2), // short
        3 => {
            let _ = read_varint32(data, offset)?;
            Ok(())
        }
        4 => skip_bytes(data, offset, 8), // long
        5 => skip_bytes(data, offset, 4), // float
        6 => skip_bytes(data, offset, 8), // double
        7 => {
            let len = read_varint32(data, offset)?;
            if len < 0 {
                return Err("negative byte-array length in NBT".into());
            }
            skip_bytes(data, offset, len as usize)
        }
        8 => {
            let _ = read_string_network(data, offset)?;
            Ok(())
        }
        9 => {
            let inner = read_u8(data, offset)?;
            let len = read_varint32(data, offset)?;
            if len < 0 {
                return Err("negative list length in NBT".into());
            }
            for _ in 0..len {
                skip_nbt_payload(data, offset, inner)?;
            }
            Ok(())
        }
        10 => loop {
            let inner_tag = read_u8(data, offset)?;
            if inner_tag == 0 {
                return Ok(());
            }
            let _ = read_string_network(data, offset)?;
            skip_nbt_payload(data, offset, inner_tag)?;
        },
        11 => {
            let len = read_varint32(data, offset)?;
            if len < 0 {
                return Err("negative int-array length in NBT".into());
            }
            for _ in 0..len {
                let _ = read_varint32(data, offset)?;
            }
            Ok(())
        }
        12 => {
            let len = read_varint32(data, offset)?;
            if len < 0 {
                return Err("negative long-array length in NBT".into());
            }
            skip_bytes(data, offset, len as usize * 8)
        }
        _ => Err(format!("unsupported NBT tag type {tag_id}")),
    }
}

fn read_string_network(data: &[u8], offset: &mut usize) -> Result<String, String> {
    let len = read_varuint32(data, offset)? as usize;
    let start = *offset;
    let end = start + len;
    if end > data.len() {
        return Err("unexpected EOF while reading NBT string".into());
    }
    *offset = end;
    std::str::from_utf8(&data[start..end])
        .map(|s| s.to_string())
        .map_err(|_| "invalid UTF-8 in NBT string".into())
}

fn read_u8(data: &[u8], offset: &mut usize) -> Result<u8, String> {
    if *offset >= data.len() {
        return Err("unexpected EOF while reading NBT u8".into());
    }
    let value = data[*offset];
    *offset += 1;
    Ok(value)
}

fn read_varuint32(data: &[u8], offset: &mut usize) -> Result<u32, String> {
    let mut value = 0u32;
    let mut shift = 0u32;

    for _ in 0..5 {
        let byte = read_u8(data, offset)?;
        value |= ((byte & 0x7F) as u32) << shift;
        if (byte & 0x80) == 0 {
            return Ok(value);
        }
        shift += 7;
    }
    Err("VarUInt32 is too long".into())
}

fn read_varint32(data: &[u8], offset: &mut usize) -> Result<i32, String> {
    let u = read_varuint32(data, offset)?;
    let value = ((u >> 1) as i32) ^ (-((u & 1) as i32));
    Ok(value)
}

fn skip_bytes(data: &[u8], offset: &mut usize, len: usize) -> Result<(), String> {
    let end = offset.saturating_add(len);
    if end > data.len() {
        return Err("unexpected EOF while skipping NBT bytes".into());
    }
    *offset = end;
    Ok(())
}

// --- Packet sending ---
async fn send_packet(
    addr: SocketAddr,
    packet_id: u32,
    body: &[u8],
    compression_enabled: bool,
    handle: &ServerHandle,
) {
    if packet_id != ID_LEVEL_CHUNK {
        trace_spawn_tx(addr, packet_id, body, "");
    }

    let frame = packets::frame_packet(packet_id, body);
    let batch_data = batch::encode_batch(&[frame], compression_enabled);

    // Prepend 0xFE (game packet marker for RakNet)
    let mut payload = BytesMut::with_capacity(1 + batch_data.len());
    payload.put_u8(0xFE);
    payload.extend_from_slice(&batch_data);

    handle
        .send_to(addr, payload.freeze(), Reliability::ReliableOrdered, 0)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_palette_contains_air_and_bedrock() {
        let bytes = include_bytes!("../../../.reference/BedrockData/canonical_block_states.nbt");
        let names = parse_canonical_block_state_names(bytes)
            .expect("canonical_block_states.nbt should parse as network NBT stream");
        assert!(
            first_runtime_id(&names, "minecraft:air").is_some(),
            "missing minecraft:air in canonical palette"
        );
        assert!(
            first_runtime_id(&names, "minecraft:bedrock").is_some(),
            "missing minecraft:bedrock in canonical palette"
        );
    }

    #[test]
    fn spawn_payload_uses_nine_subchunks_up_to_y4() {
        let payload = build_spawn_chunk_payload(15, 16).expect("spawn chunk payload should build");
        assert_eq!(
            payload.sub_chunk_count, 9,
            "expected subchunks for y=-4..4"
        );
    }

    #[test]
    fn spawn_payload_is_not_tiny_empty_column() {
        let payload = build_spawn_chunk_payload(15, 16).expect("spawn chunk payload should build");
        assert!(
            payload.payload.len() > 3000,
            "payload too small: {} bytes",
            payload.payload.len()
        );
    }
}
