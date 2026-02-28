//! Query protocol (GameSpy4) server implementation.
//!
//! Responds to basic and full stat queries over UDP.

use std::collections::HashMap;
use std::net::SocketAddr;

use tokio::net::UdpSocket;
use tokio::sync::watch;
use tracing::{debug, info, warn};

/// Server statistics exposed via the Query protocol.
#[derive(Clone, Debug)]
pub struct ServerStats {
    pub motd: String,
    pub game_type: String,
    pub map_name: String,
    pub num_players: u32,
    pub max_players: u32,
    pub host_port: u16,
    pub host_ip: String,
    pub player_names: Vec<String>,
    pub version: String,
}

impl Default for ServerStats {
    fn default() -> Self {
        Self {
            motd: "MC-RS Server".into(),
            game_type: "SMP".into(),
            map_name: "world".into(),
            num_players: 0,
            max_players: 20,
            host_port: 19132,
            host_ip: "0.0.0.0".into(),
            player_names: Vec::new(),
            version: "1.26.0".into(),
        }
    }
}

const QUERY_HANDSHAKE: u8 = 0x09;
const QUERY_STAT: u8 = 0x00;

/// Start the Query server on the given port.
pub fn start(port: u16, stats_rx: watch::Receiver<ServerStats>) {
    tokio::spawn(async move {
        let addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();
        let socket = match UdpSocket::bind(addr).await {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to bind Query server on port {port}: {e}");
                return;
            }
        };
        info!("Query server listening on port {port}");

        let mut challenge_tokens: HashMap<SocketAddr, i32> = HashMap::new();
        let mut next_token: i32 = 1;
        let mut buf = [0u8; 1024];

        loop {
            let (len, peer) = match socket.recv_from(&mut buf).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Query recv error: {e}");
                    continue;
                }
            };

            if len < 7 {
                continue;
            }

            // Query packet format: u16_be(magic=0xFEFD) + u8(type) + i32_be(session_id)
            if buf[0] != 0xFE || buf[1] != 0xFD {
                continue;
            }

            let packet_type = buf[2];
            let session_id = i32::from_be_bytes([buf[3], buf[4], buf[5], buf[6]]);

            match packet_type {
                QUERY_HANDSHAKE => {
                    let token = next_token;
                    next_token = next_token.wrapping_add(1);
                    challenge_tokens.insert(peer, token);

                    // Response: u8(type=0x09) + i32_be(session_id) + token_string + \0
                    let token_str = token.to_string();
                    let mut resp = Vec::with_capacity(6 + token_str.len());
                    resp.push(QUERY_HANDSHAKE);
                    resp.extend_from_slice(&session_id.to_be_bytes());
                    resp.extend_from_slice(token_str.as_bytes());
                    resp.push(0);

                    if let Err(e) = socket.send_to(&resp, peer).await {
                        debug!("Query send error to {peer}: {e}");
                    }
                }
                QUERY_STAT => {
                    if len < 11 {
                        continue;
                    }

                    let client_token = i32::from_be_bytes([buf[7], buf[8], buf[9], buf[10]]);
                    if challenge_tokens.get(&peer) != Some(&client_token) {
                        continue;
                    }

                    let stats = stats_rx.borrow().clone();
                    let is_full = len > 11; // Full stat if padding bytes present

                    if is_full {
                        let resp = build_full_stat(session_id, &stats);
                        if let Err(e) = socket.send_to(&resp, peer).await {
                            debug!("Query send error to {peer}: {e}");
                        }
                    } else {
                        let resp = build_basic_stat(session_id, &stats);
                        if let Err(e) = socket.send_to(&resp, peer).await {
                            debug!("Query send error to {peer}: {e}");
                        }
                    }
                }
                _ => {}
            }
        }
    });
}

fn build_basic_stat(session_id: i32, stats: &ServerStats) -> Vec<u8> {
    let mut resp = Vec::new();
    resp.push(QUERY_STAT);
    resp.extend_from_slice(&session_id.to_be_bytes());

    // Basic stat fields as null-terminated strings
    push_cstring(&mut resp, &stats.motd);
    push_cstring(&mut resp, &stats.game_type);
    push_cstring(&mut resp, &stats.map_name);
    push_cstring(&mut resp, &stats.num_players.to_string());
    push_cstring(&mut resp, &stats.max_players.to_string());

    // Host port (little-endian u16) + host IP string
    resp.extend_from_slice(&stats.host_port.to_le_bytes());
    push_cstring(&mut resp, &stats.host_ip);

    resp
}

fn build_full_stat(session_id: i32, stats: &ServerStats) -> Vec<u8> {
    let mut resp = Vec::new();
    resp.push(QUERY_STAT);
    resp.extend_from_slice(&session_id.to_be_bytes());

    // Padding (11 bytes)
    resp.extend_from_slice(b"splitnum\0\x80\0");

    // Key-value section
    let kvs = [
        ("hostname", stats.motd.as_str()),
        ("gametype", stats.game_type.as_str()),
        ("game_id", "MINECRAFTBE"),
        ("version", stats.version.as_str()),
        ("plugins", ""),
        ("map", stats.map_name.as_str()),
        ("numplayers", &stats.num_players.to_string()),
        ("maxplayers", &stats.max_players.to_string()),
        ("hostport", &stats.host_port.to_string()),
        ("hostip", stats.host_ip.as_str()),
    ];
    for (key, val) in &kvs {
        push_cstring(&mut resp, key);
        push_cstring(&mut resp, val);
    }
    resp.push(0); // End of key-value

    // Player section
    resp.push(1);
    push_cstring(&mut resp, "player_");
    resp.push(0);
    for name in &stats.player_names {
        push_cstring(&mut resp, name);
    }
    resp.push(0); // End of player list

    resp
}

fn push_cstring(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_stat_response() {
        let stats = ServerStats {
            motd: "Test".into(),
            game_type: "SMP".into(),
            map_name: "world".into(),
            num_players: 3,
            max_players: 20,
            host_port: 19132,
            host_ip: "127.0.0.1".into(),
            player_names: vec![],
            version: "1.26.0".into(),
        };
        let resp = build_basic_stat(1, &stats);
        assert!(resp.len() > 5);
        assert_eq!(resp[0], QUERY_STAT);
    }

    #[test]
    fn full_stat_response() {
        let stats = ServerStats {
            motd: "Test".into(),
            game_type: "SMP".into(),
            map_name: "world".into(),
            num_players: 1,
            max_players: 20,
            host_port: 19132,
            host_ip: "127.0.0.1".into(),
            player_names: vec!["Alice".into()],
            version: "1.26.0".into(),
        };
        let resp = build_full_stat(1, &stats);
        assert!(resp.len() > 20);
        assert_eq!(resp[0], QUERY_STAT);
        // Should contain "Alice" somewhere
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.contains("Alice"));
    }

    #[test]
    fn default_stats() {
        let stats = ServerStats::default();
        assert_eq!(stats.motd, "MC-RS Server");
        assert_eq!(stats.max_players, 20);
        assert_eq!(stats.host_port, 19132);
    }
}
