use std::net::SocketAddr;

use tokio::sync::watch;
use tracing::{error, info};

mod connection;

use mc_rs_raknet::{RakNetConfig, RakNetEvent, RakNetServer, Reliability, ServerMotd};

const BIND_ADDR: &str = "0.0.0.0:19132";
const PROTOCOL_VERSION: u32 = 924;
const MC_VERSION: &str = "1.26.0";
const SERVER_NAME: &str = "mc-rs";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("Starting {SERVER_NAME} on {BIND_ADDR} (protocol {PROTOCOL_VERSION})");

    let config = RakNetConfig {
        address: BIND_ADDR.parse().unwrap(),
        server_guid: rand::random(),
        motd: ServerMotd {
            server_name: SERVER_NAME.into(),
            protocol_version: PROTOCOL_VERSION,
            game_version: MC_VERSION.into(),
            online_players: 0,
            max_players: 20,
            server_guid: rand::random(),
            world_name: "mc-rs world".into(),
            gamemode: "Creative".into(),
            gamemode_numeric: 1,
            ipv4_port: 19132,
            ipv6_port: 19133,
            is_editor_mode: 0,
        },
        max_connections: 20,
    };

    let (mut server, mut events, handle) = RakNetServer::bind(config)
        .await
        .expect("failed to bind RakNet server");

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("Shutdown signal received");
        let _ = shutdown_tx.send(true);
    });

    let handle_clone = handle.clone();
    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            match event {
                RakNetEvent::SessionConnected { addr, guid } => {
                    info!("Session connected: {addr} (guid={guid})");
                }
                RakNetEvent::SessionDisconnected { addr } => {
                    info!("Session disconnected: {addr}");
                    connection::on_disconnect(addr);
                }
                RakNetEvent::Packet { addr, payload } => {
                    if let Err(e) = connection::handle_packet(addr, &payload, &handle_clone).await {
                        error!("Error handling packet from {addr}: {e}");
                    }
                }
            }
        }
    });

    server.run(shutdown_rx).await;
    info!("Server stopped");
}
