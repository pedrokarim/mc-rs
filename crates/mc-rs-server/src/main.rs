#[allow(dead_code)]
mod commands;
#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod connection;
#[allow(dead_code)]
mod entity;
#[allow(dead_code)]
pub mod event;
#[allow(dead_code)]
pub mod attribute;
#[allow(dead_code)]
pub mod combat;
#[allow(dead_code)]
pub mod combat_packets;
#[allow(dead_code)]
pub mod durability;
#[allow(dead_code)]
pub mod passive_entities;
#[allow(dead_code)]
pub mod visuals;
#[allow(dead_code)]
pub mod scheduler;
#[allow(dead_code)]
pub mod survival;
#[allow(dead_code)]
pub mod effects;
#[allow(dead_code)]
pub mod armor;
#[allow(dead_code)]
pub mod enchantments;
#[allow(dead_code)]
pub mod block_behaviors;
#[allow(dead_code)]
pub mod block_entities;
#[allow(dead_code)]
pub mod crafting;
#[allow(dead_code)]
pub mod projectiles;
#[allow(dead_code)]
pub mod mob_ai;
#[allow(dead_code)]
pub mod redstone;
#[allow(dead_code)]
pub mod dimensions;
#[allow(dead_code)]
pub mod scoreboard;
#[allow(dead_code)]
pub mod world_border;
#[allow(dead_code)]
pub mod sleep;
#[allow(dead_code)]
pub mod trading;
#[allow(dead_code)]
pub mod fishing;
#[allow(dead_code)]
pub mod advancements;
#[allow(dead_code)]
pub mod statistics;
#[allow(dead_code)]
pub mod loot_tables;
#[allow(dead_code)]
pub mod sound_events;
#[allow(dead_code)]
pub mod structures;
#[allow(dead_code)]
pub mod particles_registry;
#[allow(dead_code)]
pub mod explosion;
#[allow(dead_code)]
pub mod transfer;
#[allow(dead_code)]
pub mod raid;
#[allow(dead_code)]
pub mod maps;
#[allow(dead_code)]
pub mod banner;
#[allow(dead_code)]
pub mod weather;
#[allow(dead_code)]
pub mod brewing;
#[allow(dead_code)]
pub mod firework;
#[allow(dead_code)]
pub mod composter;
#[allow(dead_code)]
pub mod beacon;
#[allow(dead_code)]
pub mod anvil;
#[allow(dead_code)]
pub mod spawn_eggs;
#[allow(dead_code)]
pub mod dyes;
#[allow(dead_code)]
pub mod liquids;
#[allow(dead_code)]
pub mod biomes_registry;
#[allow(dead_code)]
pub mod workstations;
#[allow(dead_code)]
pub mod skins;
#[allow(dead_code)]
pub mod sculk;
#[allow(dead_code)]
pub mod conduit;
#[allow(dead_code)]
pub mod form_api;
#[allow(dead_code)]
pub mod chunk_serializer_ext;
#[allow(dead_code)]
pub mod emotes;
#[allow(dead_code)]
pub mod vehicles;
#[allow(dead_code)]
pub mod furnace;
#[allow(dead_code)]
pub mod inventory;
#[allow(dead_code)]
pub mod inventory_manager;
#[allow(dead_code)]
mod item_entities;
#[allow(dead_code)]
mod item_registry;
#[allow(dead_code)]
mod mob_entities;
#[allow(dead_code)]
pub mod player_data;
#[allow(dead_code)]
pub mod player_registry;
#[allow(dead_code)]
mod plugin;
#[allow(dead_code)]
mod server_state;
#[allow(dead_code)]
mod world;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use mc_rs_crypto::ecdh::ServerKeyPair;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::*;
use mc_rs_raknet::motd::Motd;
use mc_rs_raknet::protocol::datagram::Reliability;
use mc_rs_raknet::session::SessionEvent;
use mc_rs_raknet::RakNetServer;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::commands::{
    build_command_system, dispatch_command_line, CommandSource, ExecutionContext,
    ServerCommandRuntime, ServerCommandSystem,
};
use crate::config::ServerConfig;
use crate::connection::Connection;
use crate::item_entities::{hex_preview, ItemEntityManager, PendingItemEntitySpawn};
use crate::mob_entities::MobEntityManager;
use crate::player_registry::PlayerRegistry;
use crate::plugin::{PluginLoadOrder, PluginManager};
use crate::server_state::ServerState;
use crate::world::chunk_cache::ChunkCache;
use crate::world::tick::{encode_set_time, WorldPacket, WorldState};

/// Writer qui flush stdout après chaque `write`. Sans ça, PowerShell `>` buffer
/// stdout par blocs de 4 Ko et les logs de gameplay restent invisibles jusqu'à
/// arrêt du process.
struct LineFlushStdout;

impl std::io::Write for LineFlushStdout {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut out = std::io::stdout().lock();
        let n = out.write(buf)?;
        out.flush()?;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LineFlushStdout {
    type Writer = LineFlushStdout;
    fn make_writer(&'a self) -> Self::Writer {
        LineFlushStdout
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging avec flush immédiat (nécessaire pour PowerShell `>`).
    tracing_subscriber::fmt()
        .with_writer(LineFlushStdout)
        .with_ansi(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,mc_rs_raknet=debug")),
        )
        .init();

    // Installe un hook de panic qui écrit aussi sur stdout — sinon un panic
    // part sur stderr (non redirigé par `>`) et on loupe le message.
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("\n==== PANIC ====\n{info}\n================\n");
        let _ = std::io::Write::write_all(&mut std::io::stdout(), msg.as_bytes());
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let _ = std::io::Write::write_all(&mut std::io::stderr(), msg.as_bytes());
    }));

    info!("MC-RS Server starting...");

    // Load config
    let config = ServerConfig::load("server.toml");
    let world_dir = std::path::Path::new("worlds").join(&config.world.name);
    let world_seed = config.resolve_world_seed(&world_dir);
    let conn_config = config.connection_config(world_seed);
    let mut server_state = ServerState::load(
        config.server.motd.clone(),
        config.world.name.clone(),
        world_seed,
        config.server.max_players,
    );
    let plugin_manager = Arc::new(Mutex::new(PluginManager::load_from_dir(
        std::path::Path::new("plugins"),
    )));
    let mut command_system = build_command_system();
    if let Ok(mut manager) = plugin_manager.lock() {
        manager.register_permissions(&mut command_system.permissions);
        manager.enable_plugins(PluginLoadOrder::Startup, &mut command_system);
    } else {
        warn!("Plugin manager lock is poisoned during startup; plugins are disabled.");
    }

    // Generate server keypair (reused across all connections)
    let server_keypair = Arc::new(ServerKeyPair::generate());
    info!("Server EC keypair generated");

    // Generate server GUID
    let server_guid: i64 = rand::random();

    // Build MOTD
    let motd = Motd {
        name: config.server.motd.clone(),
        protocol_version: 944,
        version_string: "1.26.10".to_string(),
        online_players: 0,
        max_players: config.server.max_players,
        server_guid,
        world_name: config.world.name.clone(),
        gamemode: config.gameplay.gamemode_display().to_string(),
    };

    // Bind RakNet server
    let addr: SocketAddr = format!("0.0.0.0:{}", config.server.port).parse().unwrap();
    let mut raknet = match RakNetServer::bind(addr, motd, server_guid).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to bind: {}", e);
            return;
        }
    };

    info!("Server listening on {} — waiting for connections", addr);

    // Connection tracking
    let mut connections: HashMap<SocketAddr, Connection> = HashMap::new();
    let mut peers: HashMap<SocketAddr, mc_rs_raknet::RakNetPeer> = HashMap::new();
    let mut registry = PlayerRegistry::new();
    let mut item_entities = ItemEntityManager::new();
    let mut mob_entities = MobEntityManager::new();
    // Passive entities (TNT / FallingBlock / XPOrb) — spawned via commands or events.
    let _passive_entities = crate::passive_entities::PassiveEntityManager::new();
    // Event manager partagé (tous les Connection le clonent).
    let event_manager: Arc<std::sync::Mutex<crate::event::EventManager>> =
        Arc::new(std::sync::Mutex::new(crate::event::EventManager::new()));
    let mut world_state = WorldState::new(
        config.gameplay.do_daylight_cycle,
        config.gameplay.do_weather_cycle,
    );

    // World chunk cache with LevelDB persistence
    let chunk_cache = std::sync::Arc::new(std::sync::Mutex::new(ChunkCache::new(
        &world_dir,
        world_seed,
        &config.world.generator,
    )));
    if let Ok(mut manager) = plugin_manager.lock() {
        manager.enable_plugins(PluginLoadOrder::PostWorld, &mut command_system);
    } else {
        warn!("Plugin manager lock is poisoned before POSTWORLD enable; plugins are disabled.");
    }
    let mut auto_save_counter: u32 = 0;
    let (console_tx, mut console_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        let mut lines = BufReader::new(tokio::io::stdin()).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if console_tx.send(trimmed.to_string()).is_err() {
                        break;
                    }
                }
                Ok(None) => {
                    info!("Local console input closed; server will continue running.");
                    break;
                }
                Err(error) => {
                    warn!(
                        "Local console input stopped after a read error: {}. Server will continue running.",
                        error
                    );
                    break;
                }
            }
        }
    });

    // Session tick interval (100 TPS = 10ms)
    let mut tick_timer = interval(Duration::from_millis(config.server.tick_rate));
    let mut should_stop = false;
    let mut console_input_open = true;

    loop {
        if should_stop {
            info!("Server stopping...");
            if let Ok(mut manager) = plugin_manager.lock() {
                manager.disable_all(&mut command_system);
            } else {
                warn!("Plugin manager lock is poisoned during shutdown.");
            }
            // Save all dirty chunks
            if let Ok(mut cache) = chunk_cache.lock() {
                cache.save_dirty();
            }
            // Save all connected players
            for (_, conn) in connections.iter() {
                if let Some(ref xuid) = conn.xuid {
                    let save = player_data::PlayerSaveData::from_runtime(
                        conn.position,
                        [conn.yaw, conn.pitch],
                        conn.gamemode,
                        20.0,
                        20.0,
                        conn.spawn_position,
                        &conn.inventory,
                    );
                    let _ = player_data::save_player(xuid, &save);
                }
            }
            info!("World and player data saved.");
            break;
        }
        tokio::select! {
            // Receive UDP packets (this awaits until data arrives)
            result = raknet.recv_and_process() => {
                if !result {
                    continue;
                }

                // Accept new peers
                while let Some(peer) = raknet.accept() {
                    let addr = peer.addr;
                    info!("New peer: {}", addr);
                    let conn = Connection::new(
                        addr,
                        Arc::clone(&server_keypair),
                        Arc::clone(&chunk_cache),
                        Arc::clone(&conn_config),
                        server_state.persistent.world_spawn,
                        server_state.effective_default_gamemode(conn_config.default_gamemode),
                        server_state.effective_difficulty(conn_config.difficulty),
                        false,
                        Arc::clone(&event_manager),
                    );
                    connections.insert(addr, conn);
                    peers.insert(addr, peer);
                }

                // Process events from all peers
                process_peer_events(
                    &mut peers,
                    &mut connections,
                    &mut raknet,
                    &mut registry,
                    &mut item_entities,
                    &mut mob_entities,
                    &mut world_state,
                    &mut server_state,
                    &plugin_manager,
                    &command_system,
                    &chunk_cache,
                    &mut should_stop,
                    &event_manager,
                );
            }

            // Tick sessions periodically
            _ = tick_timer.tick() => {
                raknet.tick_sessions().await;

                // World tick (day/night cycle, weather)
                let world_packets = world_state.tick();
                for wp in world_packets {
                    match wp {
                        WorldPacket::SetTime(time) => {
                            let time_bytes = encode_set_time(time);
                            for (addr, conn) in connections.iter_mut() {
                                if conn.is_in_game() {
                                    let pkt = conn.encode_compressed_packet(
                                        packet_id::SET_TIME,
                                        &time_bytes,
                                    );
                                    let prepared = conn.prepare_for_send(pkt);
                                    raknet.send_to_session(addr, prepared, Reliability::ReliableOrdered, false);
                                }
                            }
                        }
                    }
                }

                // Game tick (20 TPS = 1 game tick / 5 server ticks).
                // Met à jour : combat i-frames, hunger drain/regen, attribute desync sync.
                for (addr, conn) in connections.iter_mut() {
                    if !conn.is_in_game() {
                        continue;
                    }
                    conn.game_tick_accum = conn.game_tick_accum.saturating_add(1);
                    if conn.game_tick_accum >= 5 {
                        conn.game_tick_accum = 0;
                        for pkt in conn.tick_game_state() {
                            let prepared = conn.prepare_for_send(pkt);
                            raknet.send_to_session(
                                addr,
                                prepared,
                                Reliability::ReliableOrdered,
                                false,
                            );
                        }
                    }
                }

                // Tick-based chunk sending (rate limited, spiral order)
                for (addr, conn) in connections.iter_mut() {
                    if conn.should_stream_chunks() {
                        let chunk_responses = conn.send_queued_chunks();
                        for resp in chunk_responses {
                            let prepared = conn.prepare_for_send(resp);
                            raknet.send_to_session(addr, prepared, Reliability::ReliableOrdered, false);
                        }
                    }
                }

                let mut item_tick_result = if let Ok(mut cache) = chunk_cache.lock() {
                    item_entities.tick(&registry, &mut cache)
                } else {
                    crate::item_entities::TickResult {
                        despawned: Vec::new(),
                        pickup_candidates: Vec::new(),
                        movement_updates: Vec::new(),
                    }
                };
                for (move_bytes, motion_bytes) in item_tick_result.movement_updates.drain(..) {
                    for (addr, conn) in connections.iter_mut() {
                        if conn.is_in_game() {
                            let move_pkt = conn.encode_compressed_packet(
                                packet_id::MOVE_ACTOR_ABSOLUTE,
                                &move_bytes,
                            );
                            let move_prepared = conn.prepare_for_send(move_pkt);
                            raknet.send_to_session(
                                addr,
                                move_prepared,
                                Reliability::ReliableOrdered,
                                false,
                            );

                            let motion_pkt = conn.encode_compressed_packet(
                                packet_id::SET_ACTOR_MOTION,
                                &motion_bytes,
                            );
                            let motion_prepared = conn.prepare_for_send(motion_pkt);
                            raknet.send_to_session(
                                addr,
                                motion_prepared,
                                Reliability::ReliableOrdered,
                                false,
                            );
                        }
                    }
                }

                for entity in item_tick_result.despawned {
                    let remove_bytes = entity.remove_packet();
                    for (addr, conn) in connections.iter_mut() {
                        if conn.is_in_game() {
                            let pkt = conn.encode_compressed_packet(
                                packet_id::REMOVE_ACTOR,
                                &remove_bytes,
                            );
                            let prepared = conn.prepare_for_send(pkt);
                            raknet.send_to_session(
                                addr,
                                prepared,
                                Reliability::ReliableOrdered,
                                true,
                            );
                        }
                    }
                }

                for pickup in item_tick_result.pickup_candidates {
                    let Some(entity) = item_entities
                        .all()
                        .find(|entity| entity.entity_runtime_id == pickup.entity_runtime_id)
                        .cloned()
                    else {
                        continue;
                    };

                    let collected = if let Some(conn) = connections.get_mut(&pickup.player_addr) {
                        // PMMP `Inventory::addItem()` via le manager — déclenche
                        // le listener (track + pending_sync) pour chaque slot
                        // modifié. Le sync wire est ensuite émis par
                        // `tick_inventory_flush()`.
                        let added = conn
                            .inventory_manager
                            .add_item_to_main(&mut conn.inventory, entity.item.clone());
                        if added {
                            let sync_pkts: Vec<Vec<u8>> = conn
                                .tick_inventory_flush()
                                .into_iter()
                                .map(|p| conn.prepare_for_send(p))
                                .collect();
                            Some((conn.entity_runtime_id, sync_pkts))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let Some((collector_runtime_id, sync_packets)) = collected else {
                        continue;
                    };

                    let Some(removed_entity) = item_entities.remove(pickup.entity_runtime_id) else {
                        continue;
                    };

                    info!(
                        "[{}] Picked up item entity {} (item_id={})",
                        pickup.player_addr,
                        removed_entity.entity_runtime_id,
                        removed_entity.item.id,
                    );

                    for packet in sync_packets {
                        raknet.send_to_session(
                            &pickup.player_addr,
                            packet,
                            Reliability::ReliableOrdered,
                            true,
                        );
                    }

                    let take_bytes = TakeItemActor {
                        item_actor_runtime_id: removed_entity.entity_runtime_id,
                        taker_actor_runtime_id: collector_runtime_id,
                    }
                    .encode();
                    let remove_bytes = removed_entity.remove_packet();

                    for (addr, conn) in connections.iter_mut() {
                        if conn.is_in_game() {
                            let take_pkt = conn.encode_compressed_packet(
                                packet_id::TAKE_ITEM_ACTOR,
                                &take_bytes,
                            );
                            let take_prepared = conn.prepare_for_send(take_pkt);
                            raknet.send_to_session(
                                addr,
                                take_prepared,
                                Reliability::ReliableOrdered,
                                true,
                            );

                            let remove_pkt = conn.encode_compressed_packet(
                                packet_id::REMOVE_ACTOR,
                                &remove_bytes,
                            );
                            let remove_prepared = conn.prepare_for_send(remove_pkt);
                            raknet.send_to_session(
                                addr,
                                remove_prepared,
                                Reliability::ReliableOrdered,
                                true,
                            );
                        }
                    }
                }

                if let Ok(mut cache) = chunk_cache.lock() {
                    let tick_result = mob_entities.tick(&mut cache);
                    for update in tick_result.movement_updates {
                        for (addr, conn) in connections.iter_mut() {
                            if conn.is_in_game() {
                                let move_pkt = conn.encode_compressed_packet(
                                    packet_id::MOVE_ACTOR_ABSOLUTE,
                                    &update.move_packet,
                                );
                                let move_prepared = conn.prepare_for_send(move_pkt);
                                raknet.send_to_session(
                                    addr,
                                    move_prepared,
                                    Reliability::ReliableOrdered,
                                    false,
                                );

                                let motion_pkt = conn.encode_compressed_packet(
                                    packet_id::SET_ACTOR_MOTION,
                                    &update.motion_packet,
                                );
                                let motion_prepared = conn.prepare_for_send(motion_pkt);
                                raknet.send_to_session(
                                    addr,
                                    motion_prepared,
                                    Reliability::ReliableOrdered,
                                    false,
                                );
                            }
                        }
                    }
                }

                // Auto-save every 30000 ticks (~5 minutes at 100 TPS)
                auto_save_counter += 1;
                if server_state.auto_save_enabled && auto_save_counter >= 30000 {
                    auto_save_counter = 0;
                    if let Ok(mut cache) = chunk_cache.lock() {
                        cache.save_dirty();
                    }
                }

                // Also check for events after session ticks
                process_peer_events(
                    &mut peers,
                    &mut connections,
                    &mut raknet,
                    &mut registry,
                    &mut item_entities,
                    &mut mob_entities,
                    &mut world_state,
                    &mut server_state,
                    &plugin_manager,
                    &command_system,
                    &chunk_cache,
                    &mut should_stop,
                    &event_manager,
                );
            }

            console_line = console_rx.recv(), if console_input_open => {
                match console_line {
                    Some(line) => {
                        dispatch_command_line(
                            CommandSource::Console,
                            &line,
                            &command_system,
                            &mut connections,
                            &mut peers,
                            &mut raknet,
                            &mut registry,
                            &mut item_entities,
                            &mut mob_entities,
                            &mut world_state,
                            &mut server_state,
                            &plugin_manager,
                            &chunk_cache,
                            &mut should_stop,
                        );
                    }
                    None => {
                        console_input_open = false;
                    }
                }
            }
        }
    }
}

fn spawn_and_broadcast_item_entity(
    log_context: &str,
    connections: &mut HashMap<SocketAddr, Connection>,
    raknet: &mut RakNetServer,
    item_entities: &mut ItemEntityManager,
    spawn: PendingItemEntitySpawn,
) {
    let entity = item_entities.spawn(spawn);
    let add_item_bytes = entity.add_actor_packet();
    info!(
        "[{}] AddItemActor entity prepared: {}",
        log_context,
        entity.debug_summary()
    );
    info!(
        "[{}] AddItemActor body: len={} hex={}",
        log_context,
        add_item_bytes.len(),
        hex_preview(&add_item_bytes, 96)
    );

    for (other_addr, other_conn) in connections.iter_mut() {
        if other_conn.is_in_game() {
            let pkt =
                other_conn.encode_compressed_packet(packet_id::ADD_ITEM_ACTOR, &add_item_bytes);
            info!(
                "[{}] Sending ADD_ITEM_ACTOR to {}: compressed_len={} body_len={}",
                log_context,
                other_addr,
                pkt.len(),
                add_item_bytes.len()
            );
            let prepared = other_conn.prepare_for_send(pkt);
            raknet.send_to_session(other_addr, prepared, Reliability::ReliableOrdered, true);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_peer_events(
    peers: &mut HashMap<SocketAddr, mc_rs_raknet::RakNetPeer>,
    connections: &mut HashMap<SocketAddr, Connection>,
    raknet: &mut RakNetServer,
    registry: &mut PlayerRegistry,
    item_entities: &mut ItemEntityManager,
    mob_entities: &mut MobEntityManager,
    world_state: &mut WorldState,
    server_state: &mut ServerState,
    plugin_manager: &Arc<Mutex<PluginManager>>,
    command_system: &ServerCommandSystem,
    chunk_cache: &std::sync::Arc<std::sync::Mutex<ChunkCache>>,
    should_stop: &mut bool,
    event_manager: &Arc<std::sync::Mutex<crate::event::EventManager>>,
) {
    let addrs: Vec<SocketAddr> = peers.keys().copied().collect();
    for addr in addrs {
        let (pending_events, receiver_disconnected) = {
            let Some(peer) = peers.get_mut(&addr) else {
                continue;
            };
            let mut pending_events = Vec::new();
            let receiver_disconnected = loop {
                match peer.event_rx.try_recv() {
                    Ok(event) => pending_events.push(event),
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break false,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break true,
                }
            };
            (pending_events, receiver_disconnected)
        };

        for event in pending_events {
            match event {
                SessionEvent::Connected => {
                    info!("[{}] RakNet session fully connected", addr);
                }
                SessionEvent::Packet(data) => {
                    // Extract data from connection (scoped borrow)
                    let (
                        responses,
                        broadcasts,
                        join_info,
                        pending_commands,
                        item_spawns,
                        pending_entity_attacks,
                    ) = {
                        let Some(conn) = connections.get_mut(&addr) else {
                            continue;
                        };
                        let was_in_game = conn.is_in_game();

                        let responses = conn.handle_raw_batch(&data);
                        let responses: Vec<Vec<u8>> = responses
                            .into_iter()
                            .map(|r| conn.prepare_for_send(r))
                            .collect();

                        let join_info = if !was_in_game && conn.is_in_game() {
                            Some((
                                conn.display_name.clone().unwrap_or_default(),
                                conn.uuid.map(|u| *u.as_bytes()).unwrap_or([0u8; 16]),
                                conn.xuid.clone().unwrap_or_default(),
                                conn.entity_runtime_id,
                                conn.position,
                            ))
                        } else {
                            None
                        };

                        registry.update_position(
                            &addr,
                            conn.position,
                            conn.pitch,
                            conn.yaw,
                            conn.head_yaw,
                        );

                        let broadcasts = conn.take_broadcasts();
                        let pending_commands = conn.take_pending_commands();
                        let item_spawns = std::mem::take(&mut conn.pending_item_spawns);
                        let pending_entity_attacks = conn.take_pending_entity_attacks();

                        (
                            responses,
                            broadcasts,
                            join_info,
                            pending_commands,
                            item_spawns,
                            pending_entity_attacks,
                        )
                    };
                    // Borrow of conn dropped here

                    // Send responses to this player
                    for response in responses {
                        raknet.send_to_session(&addr, response, Reliability::ReliableOrdered, true);
                    }

                    // If player just joined, broadcast to others
                    if let Some((name, uuid, xuid, runtime_id, position)) = join_info {
                        let entity_id = runtime_id as i64;

                        let player_gamemode =
                            connections.get(&addr).map(|c| c.gamemode).unwrap_or(0);
                        let is_op = server_state.is_op(&name);
                        if let Some(connection) = connections.get_mut(&addr) {
                            connection.is_op = is_op;
                        }
                        registry.players.insert(
                            addr,
                            crate::player_registry::PlayerInfo {
                                addr,
                                name: name.clone(),
                                uuid,
                                xuid: xuid.clone(),
                                entity_id,
                                position,
                                pitch: 0.0,
                                yaw: 0.0,
                                head_yaw: 0.0,
                                gamemode: player_gamemode,
                            },
                        );

                        let add_player_bytes = AddPlayer {
                            uuid,
                            username: name.clone(),
                            runtime_entity_id: runtime_id,
                            platform_chat_id: String::new(),
                            position,
                            velocity: [0.0, 0.0, 0.0],
                            pitch: 0.0,
                            yaw: 0.0,
                            head_yaw: 0.0,
                            gamemode: player_gamemode,
                            entity_unique_id: entity_id,
                            permission_level: if is_op { 2 } else { 1 },
                            command_permission: if is_op { 1 } else { 0 },
                        }
                        .encode();

                        let player_list_add = PlayerList {
                            action: 0,
                            entries: vec![PlayerListAdd {
                                uuid,
                                entity_id,
                                username: name.clone(),
                                xuid: xuid.clone(),
                                platform_chat_id: String::new(),
                                build_platform: 0,
                                is_teacher: false,
                                is_host: false,
                                is_subclient: false,
                            }],
                        }
                        .encode();

                        // Don't broadcast AddPlayer for spectators (they're invisible)
                        for (other_addr, other_conn) in connections.iter_mut() {
                            if *other_addr != addr {
                                // Always send PlayerList (needed for tab list)
                                let pkt = other_conn.encode_compressed_packet(
                                    packet_id::PLAYER_LIST,
                                    &player_list_add,
                                );
                                let prepared = other_conn.prepare_for_send(pkt);
                                raknet.send_to_session(
                                    other_addr,
                                    prepared,
                                    Reliability::ReliableOrdered,
                                    true,
                                );

                                // Only send AddPlayer if NOT spectator
                                if player_gamemode != 3 {
                                    let pkt = other_conn.encode_compressed_packet(
                                        packet_id::ADD_PLAYER,
                                        &add_player_bytes,
                                    );
                                    let prepared = other_conn.prepare_for_send(pkt);
                                    raknet.send_to_session(
                                        other_addr,
                                        prepared,
                                        Reliability::ReliableOrdered,
                                        true,
                                    );
                                }
                            }
                        }

                        info!(
                            "[{}] {} joined the game (entity_id={})",
                            addr, name, entity_id
                        );

                        // PMMP `PlayerJoinEvent`. Fire pour les plugins.
                        if let Ok(mut ev_mgr) = event_manager.lock() {
                            let mut ev = crate::event::player::PlayerJoinEvent {
                                player_addr: addr,
                                display_name: name.clone(),
                                xuid: xuid.clone(),
                                entity_runtime_id: runtime_id,
                                position,
                                gamemode: player_gamemode,
                                join_message: format!("{} joined the game", name),
                            };
                            ev_mgr.call(&mut ev);
                        }

                        if let Some(joined_conn) = connections.get_mut(&addr) {
                            for entity in item_entities.all() {
                                let pkt = joined_conn.encode_compressed_packet(
                                    packet_id::ADD_ITEM_ACTOR,
                                    &entity.add_actor_packet(),
                                );
                                let prepared = joined_conn.prepare_for_send(pkt);
                                raknet.send_to_session(
                                    &addr,
                                    prepared,
                                    Reliability::ReliableOrdered,
                                    true,
                                );
                            }
                            for entity in mob_entities.all() {
                                let pkt = joined_conn.encode_compressed_packet(
                                    packet_id::ADD_ACTOR,
                                    &entity.add_actor_packet(),
                                );
                                let prepared = joined_conn.prepare_for_send(pkt);
                                raknet.send_to_session(
                                    &addr,
                                    prepared,
                                    Reliability::ReliableOrdered,
                                    true,
                                );
                            }
                        }

                        let mut command_runtime = ExecutionContext::new(
                            CommandSource::Console,
                            command_system,
                            connections,
                            peers,
                            raknet,
                            registry,
                            item_entities,
                            mob_entities,
                            world_state,
                            server_state,
                            plugin_manager,
                            chunk_cache,
                            should_stop,
                        );
                        command_runtime.sync_available_commands_for_all();
                    }

                    // Broadcast packets to all OTHER connections
                    if !broadcasts.is_empty() {
                        for broadcast in &broadcasts {
                            for (other_addr, other_conn) in connections.iter_mut() {
                                if *other_addr != addr {
                                    let prepared = other_conn.prepare_for_send(broadcast.clone());
                                    raknet.send_to_session(
                                        other_addr,
                                        prepared,
                                        Reliability::ReliableOrdered,
                                        true,
                                    );
                                }
                            }
                        }
                    }

                    for command in pending_commands {
                        if !connections.contains_key(&addr) {
                            break;
                        }
                        dispatch_command_line(
                            CommandSource::Player(addr),
                            &command,
                            command_system,
                            connections,
                            peers,
                            raknet,
                            registry,
                            item_entities,
                            mob_entities,
                            world_state,
                            server_state,
                            plugin_manager,
                            chunk_cache,
                            should_stop,
                        );
                    }

                    let log_context = addr.to_string();
                    for spawn in item_spawns {
                        spawn_and_broadcast_item_entity(
                            &log_context,
                            connections,
                            raknet,
                            item_entities,
                            spawn,
                        );
                    }

                    for attack in pending_entity_attacks {
                        const ACTION_ATTACK: u32 = 1;
                        if attack.action_type != ACTION_ATTACK {
                            continue;
                        }

                        // Si la target est un autre joueur → combat PvP via combat::attack_entity.
                        let target_player_addr = connections
                            .iter()
                            .find(|(other_addr, c)| {
                                **other_addr != addr
                                    && c.entity_runtime_id == attack.target_runtime_id
                                    && c.is_in_game()
                            })
                            .map(|(a, _)| *a);

                        if let Some(tgt_addr) = target_player_addr {
                            // Récupère la position de l'attaquant (sera utilisée pour knockback).
                            let attacker_pos = connections
                                .get(&addr)
                                .map(|c| c.position)
                                .unwrap_or([0.0, 0.0, 0.0]);
                            // Calcule dégât de base depuis held item (durability::base_attack_points) + 1 main nue.
                            let base_damage = connections
                                .get(&addr)
                                .and_then(|c| {
                                    let held = &c.inventory.slots[c.inventory.held_slot as usize];
                                    crate::durability::durable_info(held.item.id)
                                        .map(|i| i.tier.base_attack_points() as f32)
                                })
                                .unwrap_or(1.0);

                            // Appliquer l'attaque sur la target.
                            let outcome_info = if let Some(target_conn) = connections.get_mut(&tgt_addr) {
                                let outcome = {
                                    let events = target_conn.events.clone();
                                    let mut ev = events.lock().unwrap();
                                    crate::combat::attack_entity(
                                        &mut *ev,
                                        target_conn.entity_runtime_id,
                                        target_conn.position,
                                        &mut target_conn.attributes,
                                        &mut target_conn.combat,
                                        crate::event::entity::DamageCause::EntityAttack,
                                        base_damage,
                                        Some(attack.target_runtime_id),
                                        Some(attacker_pos),
                                        crate::combat::DEFAULT_KNOCKBACK_FORCE,
                                    )
                                };
                                info!(
                                    "[{}] PvP attack on {}: {} damage, died={}",
                                    addr,
                                    target_conn.entity_runtime_id,
                                    outcome.applied_damage,
                                    outcome.died,
                                );
                                Some((
                                    target_conn.entity_runtime_id,
                                    target_conn.position,
                                    outcome.knockback,
                                    outcome.died,
                                    outcome.applied_damage > 0.0,
                                    target_conn.spawn_position,
                                ))
                            } else {
                                None
                            };

                            if let Some((target_rid, target_pos, kb, died, hit, target_spawn)) =
                                outcome_info
                            {
                                // Hurt animation broadcast à tous les viewers.
                                if hit && !died {
                                    let hurt_bytes = crate::combat_packets::hurt_animation(target_rid);
                                    for (other_addr, other_conn) in connections.iter_mut() {
                                        if other_conn.is_in_game() {
                                            let pkt = other_conn.encode_compressed_packet(
                                                packet_id::ACTOR_EVENT,
                                                &hurt_bytes,
                                            );
                                            let prep = other_conn.prepare_for_send(pkt);
                                            raknet.send_to_session(
                                                other_addr,
                                                prep,
                                                Reliability::ReliableOrdered,
                                                true,
                                            );
                                        }
                                    }
                                }

                                // Knockback motion → envoyé à la target.
                                if let Some((kx, ky, kz)) = kb {
                                    if let Some(tc) = connections.get_mut(&tgt_addr) {
                                        let tick = tc.tick;
                                        let bytes = crate::combat_packets::encode_set_actor_motion(
                                            target_rid,
                                            [kx, ky, kz],
                                            tick,
                                        );
                                        let pkt = tc.encode_compressed_packet(
                                            packet_id::SET_ACTOR_MOTION,
                                            &bytes,
                                        );
                                        let prep = tc.prepare_for_send(pkt);
                                        raknet.send_to_session(
                                            &tgt_addr,
                                            prep,
                                            Reliability::ReliableOrdered,
                                            false,
                                        );
                                    }
                                }

                                if died {
                                    // Death animation à tous + message broadcast.
                                    let death_bytes = crate::combat_packets::death_animation(target_rid);
                                    for (other_addr, other_conn) in connections.iter_mut() {
                                        if other_conn.is_in_game() {
                                            let pkt = other_conn.encode_compressed_packet(
                                                packet_id::ACTOR_EVENT,
                                                &death_bytes,
                                            );
                                            let prep = other_conn.prepare_for_send(pkt);
                                            raknet.send_to_session(
                                                other_addr,
                                                prep,
                                                Reliability::ReliableOrdered,
                                                true,
                                            );
                                        }
                                    }
                                    // Respawn packet envoyé à la target pour
                                    // l'autoriser à respawn (READY_TO_SPAWN=1).
                                    if let Some(tc) = connections.get_mut(&tgt_addr) {
                                        // Restore full HP pour le respawn.
                                        tc.attributes
                                            .must_get_mut(crate::attribute::ids::HEALTH)
                                            .set_value(20.0, true);
                                        tc.attributes
                                            .must_get_mut(crate::attribute::ids::HUNGER)
                                            .set_value(20.0, true);
                                        tc.combat = crate::combat::CombatState::new();
                                        tc.position = target_spawn;
                                        let respawn_bytes = crate::combat_packets::encode_respawn(
                                            target_spawn,
                                            crate::combat_packets::respawn_state::READY_TO_SPAWN,
                                            target_rid,
                                        );
                                        let pkt = tc.encode_compressed_packet(
                                            packet_id::RESPAWN,
                                            &respawn_bytes,
                                        );
                                        let prep = tc.prepare_for_send(pkt);
                                        raknet.send_to_session(
                                            &tgt_addr,
                                            prep,
                                            Reliability::ReliableOrdered,
                                            true,
                                        );
                                    }
                                }

                                // Ignore unused
                                let _ = target_pos;
                            }
                            continue;
                        }

                        if let Some(result) =
                            mob_entities.apply_attack(attack.target_runtime_id, 4.0)
                        {
                            if let Some(update_bytes) = result.update_attributes_packet {
                                for (other_addr, other_conn) in connections.iter_mut() {
                                    if other_conn.is_in_game() {
                                        let pkt = other_conn.encode_compressed_packet(
                                            packet_id::UPDATE_ATTRIBUTES,
                                            &update_bytes,
                                        );
                                        let prepared = other_conn.prepare_for_send(pkt);
                                        raknet.send_to_session(
                                            other_addr,
                                            prepared,
                                            Reliability::ReliableOrdered,
                                            true,
                                        );
                                    }
                                }
                            }

                            if let Some(remove_bytes) = result.remove_packet {
                                for (other_addr, other_conn) in connections.iter_mut() {
                                    if other_conn.is_in_game() {
                                        let pkt = other_conn.encode_compressed_packet(
                                            packet_id::REMOVE_ACTOR,
                                            &remove_bytes,
                                        );
                                        let prepared = other_conn.prepare_for_send(pkt);
                                        raknet.send_to_session(
                                            other_addr,
                                            prepared,
                                            Reliability::ReliableOrdered,
                                            true,
                                        );
                                    }
                                }
                            }

                            if let Some(death_position) = result.death_position {
                                let log_context = format!("{addr}:mob-death");
                                for drop in result.drops {
                                    spawn_and_broadcast_item_entity(
                                        &log_context,
                                        connections,
                                        raknet,
                                        item_entities,
                                        PendingItemEntitySpawn::with_scatter(drop, death_position),
                                    );
                                }
                            }
                        }
                    }
                }
                SessionEvent::Disconnected => {
                    // Save player data before removing
                    if let Some(conn) = connections.get(&addr) {
                        if let Some(ref xuid) = conn.xuid {
                            let save_data = player_data::PlayerSaveData::from_runtime(
                                conn.position,
                                [conn.yaw, conn.pitch],
                                conn.gamemode,
                                20.0,
                                20.0,
                                conn.spawn_position,
                                &conn.inventory,
                            );
                            if let Err(e) = player_data::save_player(xuid, &save_data) {
                                warn!("Failed to save player data: {}", e);
                            }
                        }
                    }

                    // Broadcast RemoveEntity + PlayerList(REMOVE) to all others
                    if let Some(player_info) = registry.remove(&addr) {
                        info!("[{}] {} left the game", addr, player_info.name);

                        // PMMP `PlayerQuitEvent` pour les plugins.
                        if let Ok(mut ev_mgr) = event_manager.lock() {
                            let mut ev = crate::event::player::PlayerQuitEvent {
                                player_addr: addr,
                                display_name: player_info.name.clone(),
                                xuid: player_info.xuid.clone(),
                                entity_runtime_id: player_info.entity_id as u64,
                                quit_message: format!("{} left the game", player_info.name),
                                quit_reason: "Client Disconnect".to_string(),
                            };
                            ev_mgr.call(&mut ev);
                        }

                        let remove_entity = RemoveEntity {
                            entity_unique_id: player_info.entity_id,
                        }
                        .encode();

                        let player_list_remove = PlayerList {
                            action: 1,
                            entries: vec![PlayerListAdd {
                                uuid: player_info.uuid,
                                entity_id: player_info.entity_id,
                                username: String::new(),
                                xuid: String::new(),
                                platform_chat_id: String::new(),
                                build_platform: 0,
                                is_teacher: false,
                                is_host: false,
                                is_subclient: false,
                            }],
                        }
                        .encode();

                        for (other_addr, other_conn) in connections.iter_mut() {
                            if *other_addr != addr {
                                let pkt = other_conn.encode_compressed_packet(
                                    packet_id::REMOVE_ACTOR,
                                    &remove_entity,
                                );
                                let prepared = other_conn.prepare_for_send(pkt);
                                raknet.send_to_session(
                                    other_addr,
                                    prepared,
                                    Reliability::ReliableOrdered,
                                    true,
                                );

                                let pkt = other_conn.encode_compressed_packet(
                                    packet_id::PLAYER_LIST,
                                    &player_list_remove,
                                );
                                let prepared = other_conn.prepare_for_send(pkt);
                                raknet.send_to_session(
                                    other_addr,
                                    prepared,
                                    Reliability::ReliableOrdered,
                                    true,
                                );
                            }
                        }
                        let mut command_runtime = ExecutionContext::new(
                            CommandSource::Console,
                            command_system,
                            connections,
                            peers,
                            raknet,
                            registry,
                            item_entities,
                            mob_entities,
                            world_state,
                            server_state,
                            plugin_manager,
                            chunk_cache,
                            should_stop,
                        );
                        command_runtime.sync_available_commands_for_all();
                    } else {
                        info!("[{}] Disconnected", addr);
                    }

                    connections.remove(&addr);
                    peers.remove(&addr);
                    break;
                }
            }
        }

        if receiver_disconnected && peers.contains_key(&addr) {
            if let Some(player_info) = registry.remove(&addr) {
                info!("[{}] {} connection lost", addr, player_info.name);
            }
            connections.remove(&addr);
            peers.remove(&addr);
        }
    }
}
