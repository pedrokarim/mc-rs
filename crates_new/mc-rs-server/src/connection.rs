use std::collections::HashMap;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::{Mutex, OnceLock};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use tracing::{debug, info, warn};

use mc_rs_proto::batch;
use mc_rs_proto::codec::*;
use mc_rs_proto::packets::{self, *};
use mc_rs_raknet::{Reliability, ServerHandle};

static SESSIONS: Mutex<Option<HashMap<SocketAddr, Session>>> = Mutex::new(None);
static TRACE_SPAWN_ENABLED: OnceLock<bool> = OnceLock::new();
static BLOCK_RUNTIME_IDS: OnceLock<Result<BlockRuntimeIds, String>> = OnceLock::new();

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
}

struct Session {
    phase: Phase,
    compression_enabled: bool,
    actor_runtime_id: u64,
    username: String,
    client_cache_enabled: Option<bool>,
}

impl Session {
    fn new() -> Self {
        Self {
            phase: Phase::SessionStart,
            compression_enabled: false,
            actor_runtime_id: 1,
            username: String::new(),
            client_cache_enabled: None,
        }
    }
}

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
        | (Phase::PreSpawn, ID_REQUEST_CHUNK_RADIUS) => {
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
    let actor_runtime_id = {
        let mut guard = sessions();
        let map = guard.as_mut().unwrap();
        let session = map.get_mut(&addr).unwrap();
        session.phase = Phase::PreSpawn;
        session.actor_runtime_id
    };

    let spawn_pos = [0i32, 64, 0];
    let player_pos = [0.0f32, 66.62, 0.0]; // PocketMine adds eye height offset

    // 1. StartGame
    let sg = packets::start_game::encode(
        actor_runtime_id as i64,
        actor_runtime_id,
        1, // Creative
        player_pos,
        0.0,
        0.0,
        spawn_pos,
        "mc-rs",
    );
    send_packet(addr, ID_START_GAME, &sg, true, handle).await;

    // PMMP onEnterWorld parity.
    let set_time = packets::set_time::encode(0);
    send_packet(addr, ID_SET_TIME, &set_time, true, handle).await;

    let set_difficulty = packets::set_difficulty::encode(2); // normal
    send_packet(addr, ID_SET_DIFFICULTY, &set_difficulty, true, handle).await;

    let player_spawn = packets::set_spawn_position::encode_player_spawn(0, 64, 0, 0);
    send_packet(addr, ID_SET_SPAWN_POSITION, &player_spawn, true, handle).await;

    let world_spawn = packets::set_spawn_position::encode_world_spawn(0, 64, 0, 0);
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
    let spgt = packets::set_player_game_type::encode(1); // creative
    send_packet(addr, ID_SET_PLAYER_GAME_TYPE, &spgt, true, handle).await;

    // 8. UpdateAbilities
    let uab = packets::update_abilities::encode_default_creative(actor_runtime_id as i64);
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
    let skin = packets::player_list::build_minimal_skin();
    let uuid = [0u8; 16]; // placeholder
    let pl = packets::player_list::encode_add(
        &uuid,
        actor_runtime_id as i64,
        "Player",
        "",
        "",
        7,
        &skin,
        false,
        false,
        false,
        0xFFFF_FFFF,
        false,
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
    let radius = requested_radius.min(4).max(2);
    info!("[{addr}] RequestChunkRadius requested={requested_radius}, using={radius}");

    let actor_runtime_id = {
        let guard = sessions();
        let map = guard.as_ref().unwrap();
        map.get(&addr)
            .map(|s| s.actor_runtime_id)
            .ok_or_else(|| "session missing while handling chunk radius".to_string())?
    };

    // Send ChunkRadiusUpdated
    let cru = packets::chunk_radius_updated::encode(radius);
    send_packet(addr, ID_CHUNK_RADIUS_UPDATED, &cru, true, handle).await;

    // Send NetworkChunkPublisherUpdate
    let ncpu = packets::network_chunk_publisher_update::encode(0, 64, 0, (radius as u32) * 16);
    send_packet(addr, ID_NETWORK_CHUNK_PUBLISHER_UPDATE, &ncpu, true, handle).await;

    // Send chunks in the radius
    let chunk_count = send_spawn_chunks(addr, radius, handle).await?;
    info!("[{addr}] Sent {chunk_count} chunks");

    // PMMP sends a movement sync before gameplay starts; do the same to lock local actor state.
    let move_player = packets::move_player::encode(
        actor_runtime_id,
        0.0,
        66.62,
        0.0,
        0.0,
        0.0,
        0.0,
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
    let SpawnChunkPayload {
        payload,
        sub_chunk_count,
    } = build_spawn_chunk_payload()?;

    for cx in -radius..=radius {
        for cz in -radius..=radius {
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
/// - empty subchunks encoded as v8 with 0 layers
/// - top non-empty subchunk encoded as single-valued runtime palette
/// - all 24 biome sections are sent
/// - border block count is 0 and no tiles are appended
fn build_spawn_chunk_payload() -> Result<SpawnChunkPayload, String> {
    let ids = block_runtime_ids()?;
    let top_non_empty_subchunk_y = 0;
    let sub_chunk_count = (top_non_empty_subchunk_y - OVERWORLD_MIN_SUBCHUNK_Y + 1) as u32;

    let mut buf = Vec::with_capacity(1600);
    for subchunk_y in OVERWORLD_MIN_SUBCHUNK_Y..=top_non_empty_subchunk_y {
        if subchunk_y == top_non_empty_subchunk_y {
            write_single_valued_subchunk(&mut buf, ids.bedrock);
        } else {
            write_empty_subchunk(&mut buf);
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

/// PMMP-compatible single-valued runtime palette subchunk.
/// Uses bpb=1 (matching ext-chunkutils2 behavior used by PMMP).
fn write_single_valued_subchunk(buf: &mut Vec<u8>, runtime_id: i32) {
    buf.push(8); // version
    buf.push(1); // num_layers

    // Storage header: (1 << 1) | 1 = 0x03
    buf.push(0x03);

    // 128 u32 words (all indices=0)
    buf.extend_from_slice(&[0u8; 128 * 4]);

    // Palette count (zigzag signed varint): 1 -> 0x02
    buf.push(0x02);

    // Single palette entry (runtime ID as zigzag varint)
    write_zigzag_varint(buf, runtime_id);
}

/// PMMP-compatible single-valued biome palette section.
/// Uses bpb=1 (matching ext-chunkutils2 behavior used by PMMP).
fn write_single_valued_biome_section(buf: &mut Vec<u8>, biome_id: i32) {
    // Header: (1 << 1) | 1 = 0x03
    buf.push(0x03);
    // 2 u32 words (64 entries at 1 bit, all index=0)
    buf.extend_from_slice(&[0u8; 8]);
    // Palette count = 1 (zigzag)
    buf.push(0x02);
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

fn block_runtime_ids() -> Result<BlockRuntimeIds, String> {
    let cached = BLOCK_RUNTIME_IDS
        .get_or_init(load_block_runtime_ids)
        .clone();
    match cached {
        Ok(ids) => Ok(ids),
        Err(err) => {
            warn!("Failed to parse canonical_block_states.nbt ({err}); using fallback runtime IDs");
            Ok(BlockRuntimeIds { air: 0, bedrock: 1 })
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

    info!(
        "Resolved runtime IDs from canonical palette: air={air}, bedrock={bedrock}, entries={}",
        names.len()
    );

    Ok(BlockRuntimeIds { air, bedrock })
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
}
