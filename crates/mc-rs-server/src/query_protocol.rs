//! Query protocol (external server stat query) — Gamespy v4.
//!
//! Format wire :
//!   client → server : magic(2) + type(1) + session_id(i32 BE) + [challenge token]
//!   server → client : type(1) + session_id(i32 BE) + payload
//!
//! Type 9 = Handshake : server répond avec un challenge token (string ASCII).
//! Type 0 = Stat      : client envoie token + (optional padding for full stat).
//!                       Server répond avec basic ou full stat selon padding.

use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub const MAGIC_HEADER: [u8; 2] = [0xfe, 0xfd];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryPacketType {
    Handshake = 9,
    Stat = 0,
}

/// Handshake + challenge tokens (sessionless).
#[derive(Debug, Clone)]
pub struct QuerySession {
    pub session_id: i32,
    pub challenge_token: i32,
    pub expires_at: u64,
}

/// Token TTL (30s).
pub const TOKEN_TTL: u64 = 30;

#[derive(Clone)]
pub struct QueryStatus {
    pub motd: String,
    pub gametype: String,
    pub map: String,
    pub num_players: u32,
    pub max_players: u32,
    pub host_port: u16,
    pub host_ip: String,
    pub player_names: Vec<String>,
    pub plugins: String,
    pub version: String,
}

/// Démarre le serveur Query UDP. Le caller passe une closure qui retourne
/// le QueryStatus actuel (lu depuis le state du serveur).
pub fn start<F>(bind_addr: &str, status_fn: F) -> std::io::Result<()>
where
    F: Fn() -> QueryStatus + Send + Sync + 'static,
{
    let socket = UdpSocket::bind(bind_addr)?;
    tracing::info!("Query listening on {}", bind_addr);
    let tokens: Arc<Mutex<HashMap<SocketAddr, (i32, Instant)>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let status_fn = Arc::new(status_fn);
    thread::spawn(move || {
        let mut buf = [0u8; 1500];
        loop {
            let Ok((n, peer)) = socket.recv_from(&mut buf) else {
                continue;
            };
            if n < 7 || buf[0..2] != MAGIC_HEADER {
                continue;
            }
            let ptype = buf[2];
            let session_id = i32::from_be_bytes([buf[3], buf[4], buf[5], buf[6]]) & 0x0F0F0F0F;
            match ptype {
                9 => {
                    // Handshake → renvoie un token (ASCII int).
                    let token = (rand::random::<i32>() & 0x7FFFFFFF).max(1);
                    if let Ok(mut t) = tokens.lock() {
                        // Nettoie les expirés.
                        let now = Instant::now();
                        t.retain(|_, (_, exp)| now.duration_since(*exp) < Duration::from_secs(TOKEN_TTL));
                        t.insert(peer, (token, now));
                    }
                    let mut resp = Vec::with_capacity(16);
                    resp.push(9);
                    resp.extend_from_slice(&session_id.to_be_bytes());
                    let token_str = format!("{}\0", token);
                    resp.extend_from_slice(token_str.as_bytes());
                    let _ = socket.send_to(&resp, peer);
                }
                0 => {
                    if n < 11 {
                        continue;
                    }
                    let token = i32::from_be_bytes([buf[7], buf[8], buf[9], buf[10]]);
                    let valid = tokens
                        .lock()
                        .ok()
                        .and_then(|t| t.get(&peer).map(|(tk, _)| *tk == token))
                        .unwrap_or(false);
                    if !valid {
                        continue;
                    }
                    let st = status_fn();
                    let full = n >= 15;
                    let mut resp = Vec::with_capacity(256);
                    resp.push(0);
                    resp.extend_from_slice(&session_id.to_be_bytes());
                    if full {
                        resp.extend_from_slice(b"splitnum\0\x80\0");
                        let pairs: &[(&str, String)] = &[
                            ("hostname", st.motd.clone()),
                            ("gametype", st.gametype.clone()),
                            ("game_id", "MINECRAFTPE".into()),
                            ("version", st.version.clone()),
                            ("plugins", st.plugins.clone()),
                            ("map", st.map.clone()),
                            ("numplayers", st.num_players.to_string()),
                            ("maxplayers", st.max_players.to_string()),
                            ("hostport", st.host_port.to_string()),
                            ("hostip", st.host_ip.clone()),
                        ];
                        for (k, v) in pairs {
                            resp.extend_from_slice(k.as_bytes());
                            resp.push(0);
                            resp.extend_from_slice(v.as_bytes());
                            resp.push(0);
                        }
                        resp.push(0);
                        resp.extend_from_slice(b"\x01player_\0\0");
                        for name in &st.player_names {
                            resp.extend_from_slice(name.as_bytes());
                            resp.push(0);
                        }
                        resp.push(0);
                    } else {
                        // Basic stat
                        resp.extend_from_slice(st.motd.as_bytes());
                        resp.push(0);
                        resp.extend_from_slice(st.gametype.as_bytes());
                        resp.push(0);
                        resp.extend_from_slice(st.map.as_bytes());
                        resp.push(0);
                        resp.extend_from_slice(st.num_players.to_string().as_bytes());
                        resp.push(0);
                        resp.extend_from_slice(st.max_players.to_string().as_bytes());
                        resp.push(0);
                        resp.extend_from_slice(&st.host_port.to_le_bytes());
                        resp.extend_from_slice(st.host_ip.as_bytes());
                        resp.push(0);
                    }
                    let _ = socket.send_to(&resp, peer);
                }
                _ => {}
            }
        }
    });
    let _ = mpsc::channel::<()>(); // silence
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn magic_header_correct() {
        assert_eq!(super::MAGIC_HEADER, [0xfe, 0xfd]);
    }
}
