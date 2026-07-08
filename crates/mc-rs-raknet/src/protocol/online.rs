use std::net::SocketAddr;

use crate::consts::SYSTEM_ADDRESS_COUNT;
use crate::protocol::types::*;

// ── ConnectionRequest (0x09) ──

/// Decode a ConnectionRequest.
/// Layout: id(1) + client_guid(8) + send_time(8) + security(1)
pub fn decode_connection_request(buf: &[u8]) -> Option<(i64, i64, bool)> {
    if buf.len() < 18 {
        return None;
    }
    let client_guid = read_i64_be(&buf[1..9]);
    let send_time = read_i64_be(&buf[9..17]);
    let security = buf[17] != 0;
    Some((client_guid, send_time, security))
}

/// Encode a ConnectionRequestAccepted.
/// Layout: id(1) + client_addr(7) + system_index(2) + system_addresses(20*7) + ping_time(8) + pong_time(8)
pub fn encode_connection_request_accepted(
    client_addr: &SocketAddr,
    request_time: i64,
    pong_time: i64,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 7 + 2 + SYSTEM_ADDRESS_COUNT * 7 + 16);
    buf.push(0x10); // ID_CONNECTION_REQUEST_ACCEPTED

    // Client address
    write_address(&mut buf, client_addr);

    // System index (i16 BE)
    buf.push(0x00);
    buf.push(0x00);

    // 20 system addresses (all dummy)
    for _ in 0..SYSTEM_ADDRESS_COUNT {
        write_dummy_address(&mut buf);
    }

    // Request time (ping time from client)
    write_i64_be(&mut buf, request_time);

    // Pong time (our current time)
    write_i64_be(&mut buf, pong_time);

    buf
}

// ── NewIncomingConnection (0x13) ──

/// Decode a NewIncomingConnection. We just need to know it arrived.
/// Layout: id(1) + server_addr(7) + system_addresses(20*7) + ping_time(8) + pong_time(8)
pub fn decode_new_incoming_connection(buf: &[u8]) -> Option<(SocketAddr, i64, i64)> {
    if buf.len() < 8 {
        return None;
    }
    let (server_addr, addr_len) = read_address(&buf[1..]);
    // Skip system addresses
    let offset = 1 + addr_len + SYSTEM_ADDRESS_COUNT * 7;
    if buf.len() < offset + 16 {
        return None;
    }
    let ping_time = read_i64_be(&buf[offset..]);
    let pong_time = read_i64_be(&buf[offset + 8..]);
    Some((server_addr, ping_time, pong_time))
}

// ── ConnectedPing (0x00) ──

/// Decode ConnectedPing. Layout: id(1) + time(8)
pub fn decode_connected_ping(buf: &[u8]) -> Option<i64> {
    if buf.len() < 9 {
        return None;
    }
    Some(read_i64_be(&buf[1..9]))
}

/// Encode ConnectedPong. Layout: id(1) + ping_time(8) + pong_time(8)
pub fn encode_connected_pong(ping_time: i64, pong_time: i64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(17);
    buf.push(0x03); // ID_CONNECTED_PONG
    write_i64_be(&mut buf, ping_time);
    write_i64_be(&mut buf, pong_time);
    buf
}

/// Encode ConnectedPing. Layout: id(1) + time(8)
pub fn encode_connected_ping(time: i64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(9);
    buf.push(0x00); // ID_CONNECTED_PING
    write_i64_be(&mut buf, time);
    buf
}

/// Encode DisconnectionNotification. Layout: id(1)
pub fn encode_disconnection_notification() -> Vec<u8> {
    vec![0x15]
}

// ══════════════════════════════════════════════════════════════════════════
// Client → serveur : encode des requêtes / decode des réponses.
// ══════════════════════════════════════════════════════════════════════════

/// Encode un ConnectionRequest.
/// Layout : id(1) + client_guid(8) + send_time(8) + security(1)
pub fn encode_connection_request(client_guid: i64, send_time: i64, security: bool) -> Vec<u8> {
    let mut buf = Vec::with_capacity(18);
    buf.push(0x09); // ID_CONNECTION_REQUEST
    write_i64_be(&mut buf, client_guid);
    write_i64_be(&mut buf, send_time);
    buf.push(if security { 1 } else { 0 });
    buf
}

/// Parse un ConnectionRequestAccepted. On a seulement besoin des timestamps
/// (request_time / pong_time) à ré-écho dans le NewIncomingConnection.
/// Layout : id(1) + client_addr(7) + system_index(2) + system_addresses(20*7)
///          + request_time(8) + pong_time(8)
pub fn decode_connection_request_accepted(buf: &[u8]) -> Option<(i64, i64)> {
    // Le client_addr peut être IPv4 (7) ou IPv6 (29). On lit sa longueur.
    if buf.len() < 8 {
        return None;
    }
    let (_client_addr, addr_len) = read_address(&buf[1..]);
    let offset = 1 + addr_len + 2 + SYSTEM_ADDRESS_COUNT * 7;
    if buf.len() < offset + 16 {
        return None;
    }
    let request_time = read_i64_be(&buf[offset..]);
    let pong_time = read_i64_be(&buf[offset + 8..]);
    Some((request_time, pong_time))
}

/// Encode un NewIncomingConnection.
/// Layout : id(1) + server_addr(7) + system_addresses(20*7) + ping_time(8) + pong_time(8)
pub fn encode_new_incoming_connection(
    server_addr: &SocketAddr,
    ping_time: i64,
    pong_time: i64,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 7 + SYSTEM_ADDRESS_COUNT * 7 + 16);
    buf.push(0x13); // ID_NEW_INCOMING_CONNECTION
    write_address(&mut buf, server_addr);
    for _ in 0..SYSTEM_ADDRESS_COUNT {
        write_dummy_address(&mut buf);
    }
    write_i64_be(&mut buf, ping_time);
    write_i64_be(&mut buf, pong_time);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_request_accepted_size() {
        let addr: SocketAddr = "127.0.0.1:19132".parse().unwrap();
        let pkt = encode_connection_request_accepted(&addr, 12345, 12346);
        // 1 + 7 + 2 + 20*7 + 8 + 8 = 166
        assert_eq!(pkt.len(), 166);
        assert_eq!(pkt[0], 0x10);
    }

    #[test]
    fn test_connected_pong_roundtrip() {
        let pong = encode_connected_pong(1000, 2000);
        assert_eq!(pong[0], 0x03);
        let ping_time = read_i64_be(&pong[1..9]);
        let pong_time = read_i64_be(&pong[9..17]);
        assert_eq!(ping_time, 1000);
        assert_eq!(pong_time, 2000);
    }

    // ── Client-side roundtrips ──

    #[test]
    fn test_client_connection_request_decoded_by_server() {
        let req = encode_connection_request(0x1234, 5555, false);
        let (guid, send_time, security) = decode_connection_request(&req).unwrap();
        assert_eq!(guid, 0x1234);
        assert_eq!(send_time, 5555);
        assert!(!security);
    }

    #[test]
    fn test_client_decodes_accepted() {
        let addr: SocketAddr = "127.0.0.1:19132".parse().unwrap();
        let accepted = encode_connection_request_accepted(&addr, 111, 222);
        let (request_time, pong_time) = decode_connection_request_accepted(&accepted).unwrap();
        assert_eq!(request_time, 111);
        assert_eq!(pong_time, 222);
    }

    #[test]
    fn test_client_new_incoming_decoded_by_server() {
        let addr: SocketAddr = "127.0.0.1:19132".parse().unwrap();
        let nic = encode_new_incoming_connection(&addr, 111, 222);
        let (_srv, ping, pong) = decode_new_incoming_connection(&nic).unwrap();
        assert_eq!(ping, 111);
        assert_eq!(pong, 222);
    }
}
