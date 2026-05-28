//! RCON protocol (remote console).
//!
//! Format Source RCON :
//!   length    : i32 LE — taille des champs suivants (request_id + type + body + 2 nul)
//!   request_id: i32 LE
//!   type      : i32 LE (3=Login, 2=Command, 0=ResponseValue)
//!   body      : ASCII null-terminated
//!   pad       : 1 nul byte
//!
//! Auth : client envoie d'abord Login(password). Si valide, server répond
//! ResponseValue(""). Sinon répond avec request_id=-1.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RconPacketType {
    Login = 3,
    Command = 2,
    ResponseValue = 0,
}

#[derive(Debug, Clone)]
pub struct RconPacket {
    pub length: i32,
    pub request_id: i32,
    pub packet_type: RconPacketType,
    pub payload: String,
}

/// Max packet size (4110 bytes).
pub const MAX_PACKET_SIZE: usize = 4110;

impl RconPacketType {
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            3 => Some(Self::Login),
            2 => Some(Self::Command),
            0 => Some(Self::ResponseValue),
            _ => None,
        }
    }
}

/// Encode un paquet RCON Source-format dans un buffer prêt à TCP write.
pub fn encode_packet(request_id: i32, ptype: i32, body: &str) -> Vec<u8> {
    let body_bytes = body.as_bytes();
    let length = (10 + body_bytes.len()) as i32; // 4 + 4 + body + 2 nul
    let mut out = Vec::with_capacity(4 + length as usize);
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&request_id.to_le_bytes());
    out.extend_from_slice(&ptype.to_le_bytes());
    out.extend_from_slice(body_bytes);
    out.push(0);
    out.push(0);
    out
}

/// Décode un paquet RCON depuis un slice. Retourne (consumed, packet) ou
/// None si pas assez de bytes pour un paquet complet.
pub fn decode_packet(buf: &[u8]) -> Option<(usize, i32, i32, String)> {
    if buf.len() < 4 {
        return None;
    }
    let length = i32::from_le_bytes(buf[0..4].try_into().unwrap());
    if length < 10 || length as usize > MAX_PACKET_SIZE {
        return None;
    }
    let total = 4 + length as usize;
    if buf.len() < total {
        return None;
    }
    let request_id = i32::from_le_bytes(buf[4..8].try_into().unwrap());
    let ptype = i32::from_le_bytes(buf[8..12].try_into().unwrap());
    let body_end = total.saturating_sub(2);
    let body = String::from_utf8_lossy(&buf[12..body_end]).to_string();
    Some((total, request_id, ptype, body))
}

/// Démarre le serveur RCON dans un thread dédié. Pour chaque commande reçue
/// après auth, envoie sur `cmd_tx` ; les réponses arrivent via le canal
/// retourné. Caller doit poller le receiver et envoyer les réponses.
pub struct RconCommand {
    pub addr: std::net::SocketAddr,
    pub request_id: i32,
    pub command: String,
    pub response_tx: mpsc::Sender<String>,
}

pub fn start(bind_addr: &str, password: String) -> std::io::Result<mpsc::Receiver<RconCommand>> {
    let listener = TcpListener::bind(bind_addr)?;
    listener.set_nonblocking(false)?;
    let (tx, rx) = mpsc::channel();
    tracing::info!("RCON listening on {}", bind_addr);

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let tx = tx.clone();
            let pw = password.clone();
            thread::spawn(move || {
                if let Err(e) = handle_client(stream, &pw, tx) {
                    tracing::debug!("RCON client error: {e}");
                }
            });
        }
    });
    Ok(rx)
}

fn handle_client(
    mut stream: std::net::TcpStream,
    password: &str,
    tx: mpsc::Sender<RconCommand>,
) -> std::io::Result<()> {
    let peer = stream.peer_addr()?;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    let mut authed = false;

    loop {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);

        while let Some((consumed, req_id, ptype, body)) = decode_packet(&buf) {
            buf.drain(..consumed);

            match RconPacketType::from_id(ptype) {
                Some(RconPacketType::Login) => {
                    if body == password {
                        authed = true;
                        // PMMP returns request_id back ; if password is wrong, returns -1.
                        let resp = encode_packet(req_id, 0, "");
                        stream.write_all(&resp)?;
                    } else {
                        let resp = encode_packet(-1, 0, "");
                        stream.write_all(&resp)?;
                        return Ok(());
                    }
                }
                Some(RconPacketType::Command) => {
                    if !authed {
                        return Ok(());
                    }
                    let (resp_tx, resp_rx) = mpsc::channel();
                    let _ = tx.send(RconCommand {
                        addr: peer,
                        request_id: req_id,
                        command: body.clone(),
                        response_tx: resp_tx,
                    });
                    // Attente max 2s pour la réponse.
                    let resp_text = resp_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "Command timed out".to_string());
                    let resp = encode_packet(req_id, 0, &resp_text);
                    stream.write_all(&resp)?;
                }
                _ => {}
            }
        }
    }
}

impl RconPacket {
    pub fn new_login(request_id: i32, password: &str) -> Self {
        Self {
            length: 10 + password.len() as i32,
            request_id,
            packet_type: RconPacketType::Login,
            payload: password.to_string(),
        }
    }

    pub fn new_command(request_id: i32, cmd: &str) -> Self {
        Self {
            length: 10 + cmd.len() as i32,
            request_id,
            packet_type: RconPacketType::Command,
            payload: cmd.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_packet_has_correct_length() {
        let p = RconPacket::new_login(1, "password");
        assert_eq!(p.length, 18);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let bytes = encode_packet(42, 2, "say hello");
        let (consumed, id, t, body) = decode_packet(&bytes).expect("decode");
        assert_eq!(consumed, bytes.len());
        assert_eq!(id, 42);
        assert_eq!(t, 2);
        assert_eq!(body, "say hello");
    }

    #[test]
    fn decode_returns_none_on_partial() {
        assert!(decode_packet(&[]).is_none());
        assert!(decode_packet(&[1, 2, 3]).is_none());
    }
}
