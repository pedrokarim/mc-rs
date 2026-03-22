mod config;
mod connection;
mod world;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use mc_rs_crypto::ecdh::ServerKeyPair;
use mc_rs_raknet::motd::Motd;
use mc_rs_raknet::protocol::datagram::Reliability;
use mc_rs_raknet::session::SessionEvent;
use mc_rs_raknet::RakNetServer;
use tokio::time::{interval, Duration};
use tracing::{error, info};

use crate::config::ServerConfig;
use crate::connection::Connection;

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
    info!(
        "Config: port={}, motd={}",
        config.server.port, config.server.motd
    );

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
        gamemode: "Survival".to_string(),
    };

    // Bind RakNet server
    let addr: SocketAddr = format!("0.0.0.0:{}", config.server.port)
        .parse()
        .unwrap();
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

    // Session tick interval (100 TPS = 10ms)
    let mut tick_timer = interval(Duration::from_millis(10));

    loop {
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
                    let conn = Connection::new(addr, Arc::clone(&server_keypair));
                    connections.insert(addr, conn);
                    peers.insert(addr, peer);
                }

                // Process events from all peers
                process_peer_events(&mut peers, &mut connections, &mut raknet);
            }

            // Tick sessions periodically
            _ = tick_timer.tick() => {
                raknet.tick_sessions().await;

                // Also check for events after session ticks
                process_peer_events(&mut peers, &mut connections, &mut raknet);
            }
        }
    }
}

fn process_peer_events(
    peers: &mut HashMap<SocketAddr, mc_rs_raknet::RakNetPeer>,
    connections: &mut HashMap<SocketAddr, Connection>,
    raknet: &mut RakNetServer,
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
                        if let Some(conn) = connections.get_mut(&addr) {
                            let responses = conn.handle_raw_batch(&data);
                            for response in responses {
                                let prepared = conn.prepare_for_send(response);
                                raknet.send_to_session(
                                    &addr,
                                    prepared,
                                    Reliability::ReliableOrdered,
                                    true,
                                );
                            }

                            // Broadcast packets to ALL other connections
                            let broadcasts = conn.take_broadcasts();
                            if !broadcasts.is_empty() {
                                let other_addrs: Vec<SocketAddr> = connections
                                    .keys()
                                    .filter(|a| **a != addr)
                                    .copied()
                                    .collect();

                                for broadcast in &broadcasts {
                                    // Send to all OTHER players
                                    for &other_addr in &other_addrs {
                                        if let Some(other_conn) = connections.get_mut(&other_addr) {
                                            let prepared = other_conn.prepare_for_send(broadcast.clone());
                                            raknet.send_to_session(
                                                &other_addr,
                                                prepared,
                                                Reliability::ReliableOrdered,
                                                true,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    SessionEvent::Disconnected => {
                        info!("[{}] Disconnected", addr);
                        connections.remove(&addr);
                        peers.remove(&addr);
                        break;
                    }
                },
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    connections.remove(&addr);
                    peers.remove(&addr);
                    break;
                }
            }
        }
    }
}
