mod config;
mod connection;
mod permissions;
mod persistence;
mod plugin_manager;
mod query;
mod rcon;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use config::ServerConfig;
use connection::ConnectionHandler;
use mc_rs_raknet::{RakNetConfig, RakNetServer, ServerMotd};
use tokio::io::AsyncBufReadExt;
use tracing::info;

fn gamemode_to_numeric(gamemode: &str) -> u8 {
    match gamemode.to_lowercase().as_str() {
        "survival" => 0,
        "creative" => 1,
        "adventure" => 2,
        "spectator" => 3,
        _ => 0,
    }
}

#[tokio::main]
async fn main() {
    let config = Arc::new(match ServerConfig::load("server.toml") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load server.toml: {e}");
            std::process::exit(1);
        }
    });

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.logging.level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    info!(
        "MC-RS Server v{} starting on {}:{}",
        env!("CARGO_PKG_VERSION"),
        config.server.address,
        config.server.port
    );
    info!("MOTD: {}", config.server.motd);
    info!("Max players: {}", config.server.max_players);
    info!(
        "Gamemode: {}, Difficulty: {}",
        config.server.gamemode, config.server.difficulty
    );
    info!("Online mode: {}", config.server.online_mode);
    info!(
        "World: {} (generator: {}, seed: {})",
        config.world.name, config.world.generator, config.world.seed
    );

    let addr: SocketAddr = format!("{}:{}", config.server.address, config.server.port)
        .parse()
        .expect("invalid bind address");

    let server_guid: i64 = rand::random();

    let motd = ServerMotd {
        server_name: config.server.motd.clone(),
        protocol_version: 924,
        game_version: "1.26.0".into(),
        online_players: 0,
        max_players: config.server.max_players,
        server_guid,
        world_name: config.world.name.clone(),
        gamemode: config.server.gamemode.clone(),
        gamemode_numeric: gamemode_to_numeric(&config.server.gamemode),
        ipv4_port: config.server.port,
        ipv6_port: config.server.port + 1,
    };

    let raknet_config = RakNetConfig {
        address: addr,
        server_guid,
        motd,
        max_connections: config.server.max_players as usize,
    };

    let (mut server, mut events, server_handle) = RakNetServer::bind(raknet_config)
        .await
        .expect("failed to bind RakNet server");

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let shutdown_tx = Arc::new(shutdown_tx);

    // Handle Ctrl+C
    let shutdown_tx_ctrlc = shutdown_tx.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("Shutdown signal received");
        let _ = shutdown_tx_ctrlc.send(true);
    });

    // Console REPL: read lines from stdin
    let (console_tx, mut console_rx) = tokio::sync::mpsc::channel::<String>(32);
    tokio::spawn(async move {
        let stdin = tokio::io::BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim().to_string();
            if !line.is_empty() && console_tx.send(line).await.is_err() {
                break;
            }
        }
    });

    // RCON server (if enabled)
    let (rcon_tx, mut rcon_rx) = tokio::sync::mpsc::channel::<rcon::RconCommand>(32);
    if config.rcon.enabled {
        rcon::start(config.rcon.port, config.rcon.password.clone(), rcon_tx);
    }

    // Query server (if enabled)
    let (query_stats_tx, query_stats_rx) =
        tokio::sync::watch::channel(query::ServerStats::default());
    if config.query.enabled {
        query::start(config.query.port, query_stats_rx);
    }

    // Process RakNet events through the connection handler
    let online_mode = config.server.online_mode;
    let server_config = config;
    let shutdown_tx_handler = shutdown_tx.clone();
    let mut shutdown_rx_handler = shutdown_rx.clone();
    let query_enabled = server_config.query.enabled;
    tokio::spawn(async move {
        let mut handler = ConnectionHandler::new(
            server_handle,
            online_mode,
            server_config,
            shutdown_tx_handler,
        );
        let mut tick_interval = tokio::time::interval(Duration::from_millis(50));
        loop {
            tokio::select! {
                event = events.recv() => {
                    match event {
                        Some(e) => handler.handle_event(e).await,
                        None => break, // channel closed
                    }
                }
                _ = tick_interval.tick() => {
                    handler.game_tick().await;

                    // Update query stats periodically (every 100 ticks / 5 seconds)
                    if query_enabled && handler.current_tick().is_multiple_of(100) {
                        let stats = handler.build_query_stats();
                        let _ = query_stats_tx.send(stats);
                    }
                }
                Some(line) = console_rx.recv() => {
                    handler.handle_console_command(&line).await;
                }
                Some(rcon_cmd) = rcon_rx.recv() => {
                    let response = handler.handle_console_command(&rcon_cmd.command).await;
                    let _ = rcon_cmd.response_tx.send(response);
                }
                _ = shutdown_rx_handler.changed() => {
                    if *shutdown_rx_handler.borrow() {
                        info!("Saving world before shutdown...");
                        handler.save_all();
                        break;
                    }
                }
            }
        }
    });

    server.run(shutdown_rx).await;
    info!("Server shut down.");
}
