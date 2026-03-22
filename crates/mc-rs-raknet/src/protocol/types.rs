use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Read a 3-byte little-endian unsigned integer (LTriad).
pub fn read_u24_le(buf: &[u8]) -> u32 {
    u32::from_le_bytes([buf[0], buf[1], buf[2], 0])
}

/// Write a 3-byte little-endian unsigned integer (LTriad).
pub fn write_u24_le(buf: &mut Vec<u8>, v: u32) {
    let bytes = v.to_le_bytes();
    buf.extend_from_slice(&bytes[..3]);
}

/// Read a big-endian i64.
pub fn read_i64_be(buf: &[u8]) -> i64 {
    i64::from_be_bytes([
        buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
    ])
}

/// Write a big-endian i64.
pub fn write_i64_be(buf: &mut Vec<u8>, v: i64) {
    buf.extend_from_slice(&v.to_be_bytes());
}

/// Read a big-endian u16.
pub fn read_u16_be(buf: &[u8]) -> u16 {
    u16::from_be_bytes([buf[0], buf[1]])
}

/// Write a big-endian u16.
pub fn write_u16_be(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_be_bytes());
}

/// Read a big-endian i32.
pub fn read_i32_be(buf: &[u8]) -> i32 {
    i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]])
}

/// Read a RakNet-encoded socket address.
/// IPv4: version(1) + ~ip_octets(4) + port_be(2) = 7 bytes
/// IPv6: version(1) + family_le(2) + port_be(2) + flow(4) + addr(16) + scope(4) = 29 bytes
pub fn read_address(buf: &[u8]) -> (SocketAddr, usize) {
    let version = buf[0];
    if version == 4 {
        let ip = Ipv4Addr::new(!buf[1], !buf[2], !buf[3], !buf[4]);
        let port = read_u16_be(&buf[5..7]);
        (SocketAddr::new(IpAddr::V4(ip), port), 7)
    } else {
        // IPv6 — for now return a dummy, we only need IPv4 for Phase 1
        let port = read_u16_be(&buf[3..5]);
        (SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port), 29)
    }
}

/// Write a RakNet-encoded socket address (IPv4).
pub fn write_address(buf: &mut Vec<u8>, addr: &SocketAddr) {
    match addr.ip() {
        IpAddr::V4(ip) => {
            buf.push(4); // version
            for octet in ip.octets() {
                buf.push(!octet); // bitwise NOT
            }
            write_u16_be(buf, addr.port());
        }
        IpAddr::V6(_) => {
            // Simplified: write as dummy IPv4 for Phase 1
            buf.push(4);
            buf.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // 0.0.0.0 inverted
            write_u16_be(buf, addr.port());
        }
    }
}

/// Write a dummy address (0.0.0.0:0)
pub fn write_dummy_address(buf: &mut Vec<u8>) {
    buf.push(4); // IPv4
    buf.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // ~0.0.0.0
    buf.push(0);
    buf.push(0); // port 0
}

/// Write a BE-prefixed string (u16 BE length + UTF-8 bytes).
/// Used in offline RakNet packets (NOT the same as MCPE VarUInt32 strings).
pub fn write_raknet_string(buf: &mut Vec<u8>, s: &str) {
    write_u16_be(buf, s.len() as u16);
    buf.extend_from_slice(s.as_bytes());
}

/// Read a BE-prefixed string (u16 BE length + UTF-8 bytes).
pub fn read_raknet_string(buf: &[u8]) -> (String, usize) {
    let len = read_u16_be(buf) as usize;
    let s = String::from_utf8_lossy(&buf[2..2 + len]).to_string();
    (s, 2 + len)
}
