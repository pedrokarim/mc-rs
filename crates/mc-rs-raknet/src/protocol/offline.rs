use std::net::SocketAddr;

use crate::consts::{MAGIC, RAKNET_PROTOCOL_VERSION};
use crate::protocol::types::*;

/// Validate that magic bytes at the given offset match.
fn validate_magic(buf: &[u8], offset: usize) -> bool {
    buf.len() >= offset + 16 && buf[offset..offset + 16] == MAGIC
}

// ── UnconnectedPing (0x01 or 0x02) ──

/// Parse an UnconnectedPing packet.
/// Layout: id(1) + send_time(8) + magic(16) + client_guid(8) = 33 bytes
pub fn decode_unconnected_ping(buf: &[u8]) -> Option<(i64, i64)> {
    if buf.len() < 33 {
        return None;
    }
    if !validate_magic(buf, 9) {
        return None;
    }
    let send_time = read_i64_be(&buf[1..9]);
    let client_guid = read_i64_be(&buf[25..33]);
    Some((send_time, client_guid))
}

/// Encode an UnconnectedPong packet.
/// Layout: id(1) + send_time(8) + server_guid(8) + magic(16) + server_name_str(2+N)
pub fn encode_unconnected_pong(send_time: i64, server_guid: i64, server_name: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(35 + server_name.len());
    buf.push(0x1C); // ID
    write_i64_be(&mut buf, send_time);
    write_i64_be(&mut buf, server_guid);
    buf.extend_from_slice(&MAGIC);
    write_raknet_string(&mut buf, server_name);
    buf
}

// ── OpenConnectionRequest1 (0x05) ──

/// Parse OpenConnectionRequest1.
/// Layout: id(1) + magic(16) + protocol(1) + padding(N)
/// Returns (protocol_version, mtu_size). MTU = total packet length.
pub fn decode_open_connection_request_1(buf: &[u8]) -> Option<(u8, u16)> {
    if buf.len() < 18 {
        return None;
    }
    if !validate_magic(buf, 1) {
        return None;
    }
    let protocol = buf[17];
    let mtu_size = buf.len() as u16;
    Some((protocol, mtu_size))
}

/// Encode OpenConnectionReply1.
/// Layout: id(1) + magic(16) + server_guid(8) + security(1) + mtu_size(2) = 28 bytes
pub fn encode_open_connection_reply_1(server_guid: i64, mtu_size: u16) -> Vec<u8> {
    let mut buf = Vec::with_capacity(28);
    buf.push(0x06); // ID
    buf.extend_from_slice(&MAGIC);
    write_i64_be(&mut buf, server_guid);
    buf.push(0x00); // security = false
    write_u16_be(&mut buf, mtu_size);
    buf
}

/// Encode IncompatibleProtocolVersion.
/// Layout: id(1) + protocol(1) + magic(16) + server_guid(8) = 26 bytes
pub fn encode_incompatible_protocol(server_guid: i64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(26);
    buf.push(0x19); // ID
    buf.push(RAKNET_PROTOCOL_VERSION);
    buf.extend_from_slice(&MAGIC);
    write_i64_be(&mut buf, server_guid);
    buf
}

// ── OpenConnectionRequest2 (0x07) ──

/// Parse OpenConnectionRequest2.
/// Layout: id(1) + magic(16) + address(7 for IPv4) + mtu(2) + client_guid(8)
/// Returns (server_address, mtu_size, client_guid).
pub fn decode_open_connection_request_2(buf: &[u8]) -> Option<(SocketAddr, u16, i64)> {
    if buf.len() < 17 + 7 + 2 + 8 {
        return None;
    }
    if !validate_magic(buf, 1) {
        return None;
    }
    let (addr, addr_len) = read_address(&buf[17..]);
    let offset = 17 + addr_len;
    if buf.len() < offset + 10 {
        return None;
    }
    let mtu_size = read_u16_be(&buf[offset..]);
    let client_guid = read_i64_be(&buf[offset + 2..]);
    Some((addr, mtu_size, client_guid))
}

/// Encode OpenConnectionReply2.
/// Layout: id(1) + magic(16) + server_guid(8) + client_addr(7) + mtu(2) + security(1) = 35 bytes
pub fn encode_open_connection_reply_2(
    server_guid: i64,
    client_addr: &SocketAddr,
    mtu_size: u16,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(35);
    buf.push(0x08); // ID
    buf.extend_from_slice(&MAGIC);
    write_i64_be(&mut buf, server_guid);
    write_address(&mut buf, client_addr);
    write_u16_be(&mut buf, mtu_size);
    buf.push(0x00); // security = false
    buf
}

// ══════════════════════════════════════════════════════════════════════════
// Client → serveur : encode des requêtes / decode des réponses.
//
// Miroir exact des fonctions serveur ci-dessus, pour qu'un client (mc-rs-sim)
// puisse ouvrir une connexion. Layouts identiques, sens inversé.
// ══════════════════════════════════════════════════════════════════════════

/// Encode un UnconnectedPing.
/// Layout : id(1) + send_time(8) + magic(16) + client_guid(8) = 33 bytes
pub fn encode_unconnected_ping(send_time: i64, client_guid: i64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(33);
    buf.push(0x01); // ID_UNCONNECTED_PING
    write_i64_be(&mut buf, send_time);
    buf.extend_from_slice(&MAGIC);
    write_i64_be(&mut buf, client_guid);
    buf
}

/// Parse un UnconnectedPong. Layout : id(1) + send_time(8) + server_guid(8) +
/// magic(16) + server_name_str(2+N). Retourne (send_time, server_guid, motd).
pub fn decode_unconnected_pong(buf: &[u8]) -> Option<(i64, i64, String)> {
    if buf.len() < 35 {
        return None;
    }
    if !validate_magic(buf, 17) {
        return None;
    }
    let send_time = read_i64_be(&buf[1..9]);
    let server_guid = read_i64_be(&buf[9..17]);
    let (motd, _) = read_raknet_string(&buf[33..]);
    Some((send_time, server_guid, motd))
}

/// Encode un OpenConnectionRequest1 rembourré jusqu'à `mtu` octets au total.
/// Layout : id(1) + magic(16) + protocol(1) + padding(mtu-18). Le serveur lit
/// la MTU depuis la longueur du paquet (`buf.len()`), d'où le padding.
pub fn encode_open_connection_request_1(protocol: u8, mtu: u16) -> Vec<u8> {
    let mtu = (mtu as usize).max(18);
    let mut buf = Vec::with_capacity(mtu);
    buf.push(0x05); // ID_OPEN_CONNECTION_REQUEST_1
    buf.extend_from_slice(&MAGIC);
    buf.push(protocol);
    buf.resize(mtu, 0x00); // padding
    buf
}

/// Parse OpenConnectionReply1.
/// Layout : id(1) + magic(16) + server_guid(8) + security(1) + mtu_size(2)
/// Retourne (server_guid, mtu_size).
pub fn decode_open_connection_reply_1(buf: &[u8]) -> Option<(i64, u16)> {
    if buf.len() < 28 {
        return None;
    }
    if !validate_magic(buf, 1) {
        return None;
    }
    let server_guid = read_i64_be(&buf[17..25]);
    // buf[25] = security
    let mtu_size = read_u16_be(&buf[26..28]);
    Some((server_guid, mtu_size))
}

/// Encode OpenConnectionRequest2.
/// Layout : id(1) + magic(16) + server_addr(7) + mtu(2) + client_guid(8)
pub fn encode_open_connection_request_2(
    server_addr: &SocketAddr,
    mtu: u16,
    client_guid: i64,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(34);
    buf.push(0x07); // ID_OPEN_CONNECTION_REQUEST_2
    buf.extend_from_slice(&MAGIC);
    write_address(&mut buf, server_addr);
    write_u16_be(&mut buf, mtu);
    write_i64_be(&mut buf, client_guid);
    buf
}

/// Parse OpenConnectionReply2.
/// Layout : id(1) + magic(16) + server_guid(8) + client_addr(7) + mtu(2) + security(1)
/// Retourne (server_guid, mtu_size).
pub fn decode_open_connection_reply_2(buf: &[u8]) -> Option<(i64, u16)> {
    if buf.len() < 17 + 8 + 7 + 2 + 1 {
        return None;
    }
    if !validate_magic(buf, 1) {
        return None;
    }
    let server_guid = read_i64_be(&buf[17..25]);
    let (_client_addr, addr_len) = read_address(&buf[25..]);
    let offset = 25 + addr_len;
    if buf.len() < offset + 2 {
        return None;
    }
    let mtu_size = read_u16_be(&buf[offset..]);
    Some((server_guid, mtu_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unconnected_pong_encode() {
        let pong = encode_unconnected_pong(
            12345,
            67890,
            "MCPE;Test;2193;1.26.51;0;20;67890;world;Survival;19132;19133;",
        );
        assert_eq!(pong[0], 0x1C);
        // send_time at offset 1
        assert_eq!(read_i64_be(&pong[1..9]), 12345);
        // server_guid at offset 9
        assert_eq!(read_i64_be(&pong[9..17]), 67890);
        // magic at offset 17
        assert_eq!(&pong[17..33], &MAGIC);
    }

    #[test]
    fn test_open_connection_reply_1() {
        let reply = encode_open_connection_reply_1(12345, 1400);
        assert_eq!(reply[0], 0x06);
        assert_eq!(reply.len(), 28);
        assert_eq!(reply[25], 0x00); // security = false
        assert_eq!(read_u16_be(&reply[26..28]), 1400);
    }

    #[test]
    fn test_incompatible_protocol() {
        let pkt = encode_incompatible_protocol(12345);
        assert_eq!(pkt[0], 0x19);
        assert_eq!(pkt[1], RAKNET_PROTOCOL_VERSION);
        assert_eq!(pkt.len(), 26);
    }

    #[test]
    fn test_open_connection_reply_2() {
        let addr: SocketAddr = "127.0.0.1:19132".parse().unwrap();
        let reply = encode_open_connection_reply_2(12345, &addr, 1400);
        assert_eq!(reply[0], 0x08);
        assert_eq!(reply.last(), Some(&0x00)); // security = false
    }

    // ── Client-side roundtrips (encode requête ↔ decode par le serveur) ──

    #[test]
    fn test_client_request_1_decoded_by_server() {
        let req = encode_open_connection_request_1(RAKNET_PROTOCOL_VERSION, 1400);
        assert_eq!(req[0], 0x05);
        assert_eq!(req.len(), 1400);
        let (protocol, mtu) = decode_open_connection_request_1(&req).unwrap();
        assert_eq!(protocol, RAKNET_PROTOCOL_VERSION);
        assert_eq!(mtu, 1400);
    }

    #[test]
    fn test_client_decodes_server_reply_1() {
        let reply = encode_open_connection_reply_1(0xABCD, 1400);
        let (guid, mtu) = decode_open_connection_reply_1(&reply).unwrap();
        assert_eq!(guid, 0xABCD);
        assert_eq!(mtu, 1400);
    }

    #[test]
    fn test_client_request_2_decoded_by_server() {
        let addr: SocketAddr = "127.0.0.1:19132".parse().unwrap();
        let req = encode_open_connection_request_2(&addr, 1400, 0x1122);
        let (_srv_addr, mtu, client_guid) = decode_open_connection_request_2(&req).unwrap();
        assert_eq!(mtu, 1400);
        assert_eq!(client_guid, 0x1122);
    }

    #[test]
    fn test_client_decodes_server_reply_2() {
        let addr: SocketAddr = "127.0.0.1:19132".parse().unwrap();
        let reply = encode_open_connection_reply_2(0xABCD, &addr, 1400);
        let (guid, mtu) = decode_open_connection_reply_2(&reply).unwrap();
        assert_eq!(guid, 0xABCD);
        assert_eq!(mtu, 1400);
    }

    #[test]
    fn test_client_pong_roundtrip() {
        let pong = encode_unconnected_pong(999, 0xBEEF, "MCPE;Hello;2193;1.26.51;0;5;");
        let (send_time, guid, motd) = decode_unconnected_pong(&pong).unwrap();
        assert_eq!(send_time, 999);
        assert_eq!(guid, 0xBEEF);
        assert!(motd.starts_with("MCPE;Hello"));
    }
}
