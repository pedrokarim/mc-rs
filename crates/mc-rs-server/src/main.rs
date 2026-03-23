#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod connection;
#[allow(dead_code)]
pub mod inventory;
#[allow(dead_code)]
pub mod player_data;
#[allow(dead_code)]
pub mod player_registry;
#[allow(dead_code)]
mod world;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use mc_rs_crypto::ecdh::ServerKeyPair;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::*;
use mc_rs_raknet::motd::Motd;
use mc_rs_raknet::protocol::datagram::Reliability;
use mc_rs_raknet::session::SessionEvent;
use mc_rs_raknet::RakNetServer;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::config::ServerConfig;
use crate::connection::Connection;
use crate::player_registry::PlayerRegistry;
use crate::world::chunk_cache::ChunkCache;
use crate::world::tick::{encode_set_time, WorldPacket, WorldState};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,mc_rs_raknet=debug")),
        )
        .init();

    info!("MC-RS Server starting...");

    // Load config
    let config = ServerConfig::load("server.toml");
    let conn_config = config.connection_config();

    // Generate server keypair (reused across all connections)
    let server_keypair = Arc::new(ServerKeyPair::generate());
    info!("Server EC keypair generated");

    // Generate server GUID
    let server_guid: i64 = rand::random();

    // Build MOTD
    let motd = Motd {
        name: config.server.motd.clone(),
        protocol_version: 924,
        version_string: "1.26.2".to_string(),
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
    let mut world_state = WorldState::new(
        config.gameplay.do_daylight_cycle,
        config.gameplay.do_weather_cycle,
    );

    // World chunk cache with LevelDB persistence
    let world_dir = std::path::Path::new("worlds").join(&config.world.name);
    let chunk_cache = std::sync::Arc::new(std::sync::Mutex::new(ChunkCache::new(
        &world_dir,
        config.world.seed as u64,
        &config.world.generator,
    )));
    let mut auto_save_counter: u32 = 0;
    let mut server_tick: u64 = 0;

    // Session tick interval (100 TPS = 10ms)
    let mut tick_timer = interval(Duration::from_millis(config.server.tick_rate));
    let mut should_stop = false;

    loop {
        if should_stop {
            info!("Server stopping...");
            // Save all dirty chunks
            if let Ok(mut cache) = chunk_cache.lock() {
                cache.save_dirty();
            }
            // Save all connected players
            for (_, conn) in connections.iter() {
                if let Some(ref xuid) = conn.xuid {
                    let save = player_data::PlayerSaveData {
                        position: [
                            conn.position[0] as f64,
                            conn.position[1] as f64,
                            conn.position[2] as f64,
                        ],
                        rotation: [conn.yaw, conn.pitch],
                        gamemode: 0,
                        health: 20.0,
                        hunger: 20.0,
                    };
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
                    let conn = Connection::new(addr, Arc::clone(&server_keypair), Arc::clone(&chunk_cache), Arc::clone(&conn_config));
                    connections.insert(addr, conn);
                    peers.insert(addr, peer);
                }

                // Process events from all peers
                process_peer_events(&mut peers, &mut connections, &mut raknet, &mut registry, &mut world_state, &mut should_stop);
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

                // Tick-based chunk sending (rate limited, spiral order)
                server_tick += 1;
                for (addr, conn) in connections.iter_mut() {
                    if conn.is_in_game() {
                        let chunk_responses = conn.send_queued_chunks(server_tick);
                        for resp in chunk_responses {
                            let prepared = conn.prepare_for_send(resp);
                            raknet.send_to_session(addr, prepared, Reliability::ReliableOrdered, false);
                        }
                    }
                }

                // Auto-save every 30000 ticks (~5 minutes at 100 TPS)
                auto_save_counter += 1;
                if auto_save_counter >= 30000 {
                    auto_save_counter = 0;
                    if let Ok(mut cache) = chunk_cache.lock() {
                        cache.save_dirty();
                    }
                }

                // Also check for events after session ticks
                process_peer_events(&mut peers, &mut connections, &mut raknet, &mut registry, &mut world_state, &mut should_stop);
            }
        }
    }
}

fn process_peer_events(
    peers: &mut HashMap<SocketAddr, mc_rs_raknet::RakNetPeer>,
    connections: &mut HashMap<SocketAddr, Connection>,
    raknet: &mut RakNetServer,
    registry: &mut PlayerRegistry,
    world_state: &mut WorldState,
    should_stop: &mut bool,
) {
    let addrs: Vec<SocketAddr> = peers.keys().copied().collect();
    for addr in addrs {
        let Some(peer) = peers.get_mut(&addr) else {
            continue;
        };

        loop {
            match peer.event_rx.try_recv() {
                Ok(event) => match event {
                    SessionEvent::Connected => {
                        info!("[{}] RakNet session fully connected", addr);
                    }
                    SessionEvent::Packet(data) => {
                        // Extract data from connection (scoped borrow)
                        let (responses, broadcasts, join_info, actions) = {
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
                            let actions = std::mem::take(&mut conn.pending_actions);

                            (responses, broadcasts, join_info, actions)
                        };
                        // Borrow of conn dropped here

                        // Send responses to this player
                        for response in responses {
                            raknet.send_to_session(
                                &addr,
                                response,
                                Reliability::ReliableOrdered,
                                true,
                            );
                        }

                        // If player just joined, broadcast to others
                        if let Some((name, uuid, xuid, runtime_id, position)) = join_info {
                            let entity_id = runtime_id as i64;

                            let player_gamemode =
                                connections.get(&addr).map(|c| c.gamemode).unwrap_or(0);
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
                                permission_level: 1,   // MEMBER
                                command_permission: 0, // NORMAL
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
                        }

                        // Broadcast packets to all OTHER connections
                        if !broadcasts.is_empty() {
                            for broadcast in &broadcasts {
                                for (other_addr, other_conn) in connections.iter_mut() {
                                    if *other_addr != addr {
                                        let prepared =
                                            other_conn.prepare_for_send(broadcast.clone());
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

                        // Process server-side actions from commands
                        for action in actions {
                            match action {
                                mc_rs_command::CommandAction::SetTime { time } => {
                                    world_state.set_time(time);
                                    // Broadcast immediately
                                    let time_bytes = encode_set_time(time);
                                    for (a, c) in connections.iter_mut() {
                                        if c.is_in_game() {
                                            let pkt = c.encode_compressed_packet(
                                                packet_id::SET_TIME,
                                                &time_bytes,
                                            );
                                            let prepared = c.prepare_for_send(pkt);
                                            raknet.send_to_session(
                                                a,
                                                prepared,
                                                Reliability::ReliableOrdered,
                                                true,
                                            );
                                        }
                                    }
                                }
                                mc_rs_command::CommandAction::SetWeather { rain, thunder } => {
                                    world_state.set_weather(rain, thunder);
                                    info!("Weather changed: rain={}, thunder={}", rain, thunder);
                                }
                                mc_rs_command::CommandAction::Stop => {
                                    *should_stop = true;
                                }
                                _ => {} // Other actions handled in connection.rs
                            }
                        }
                    }
                    SessionEvent::Disconnected => {
                        // Save player data before removing
                        if let Some(conn) = connections.get(&addr) {
                            if let Some(ref xuid) = conn.xuid {
                                let save_data = player_data::PlayerSaveData {
                                    position: [
                                        conn.position[0] as f64,
                                        conn.position[1] as f64,
                                        conn.position[2] as f64,
                                    ],
                                    rotation: [conn.yaw, conn.pitch],
                                    gamemode: conn.gamemode,
                                    health: 20.0,
                                    hunger: 20.0,
                                };
                                if let Err(e) = player_data::save_player(xuid, &save_data) {
                                    warn!("Failed to save player data: {}", e);
                                }
                            }
                        }

                        // Broadcast RemoveEntity + PlayerList(REMOVE) to all others
                        if let Some(player_info) = registry.remove(&addr) {
                            info!("[{}] {} left the game", addr, player_info.name);

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
                        } else {
                            info!("[{}] Disconnected", addr);
                        }

                        connections.remove(&addr);
                        peers.remove(&addr);
                        break;
                    }
                },
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    if let Some(player_info) = registry.remove(&addr) {
                        info!("[{}] {} connection lost", addr, player_info.name);
                    }
                    connections.remove(&addr);
                    peers.remove(&addr);
                    break;
                }
            }
        }
    }
}
