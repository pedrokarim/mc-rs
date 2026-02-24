use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use bytes::{Buf, BufMut};

use crate::error::RakNetError;

/// A RakNet network address as transmitted on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RakNetAddress(pub SocketAddr);

impl RakNetAddress {
    /// An empty IPv4 address (`0.0.0.0:0`), used to fill system address arrays.
    pub const EMPTY_V4: Self = Self(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)));

    pub fn encode(&self, buf: &mut impl BufMut) {
        match self.0 {
            SocketAddr::V4(addr) => {
                buf.put_u8(4);
                // IPv4 bytes are inverted (XOR 0xFF) in RakNet wire format
                for &b in &addr.ip().octets() {
                    buf.put_u8(!b);
                }
                buf.put_u16(addr.port());
            }
            SocketAddr::V6(addr) => {
                buf.put_u8(6);
                buf.put_u16_le(23); // AF_INET6
                buf.put_u16(addr.port());
                buf.put_u32(addr.flowinfo());
                buf.put_slice(&addr.ip().octets());
                buf.put_u32(addr.scope_id());
            }
        }
    }

    pub fn decode(buf: &mut impl Buf) -> Result<Self, RakNetError> {
        if buf.remaining() < 1 {
            return Err(RakNetError::PacketTooShort {
                expected: 1,
                actual: 0,
            });
        }
        let version = buf.get_u8();
        match version {
            4 => {
                if buf.remaining() < 6 {
                    return Err(RakNetError::PacketTooShort {
                        expected: 6,
                        actual: buf.remaining(),
                    });
                }
                let mut octets = [0u8; 4];
                for b in &mut octets {
                    *b = !buf.get_u8();
                }
                let port = buf.get_u16();
                Ok(Self(SocketAddr::V4(SocketAddrV4::new(
                    Ipv4Addr::from(octets),
                    port,
                ))))
            }
            6 => {
                if buf.remaining() < 26 {
                    return Err(RakNetError::PacketTooShort {
                        expected: 26,
                        actual: buf.remaining(),
                    });
                }
                let _family = buf.get_u16_le();
                let port = buf.get_u16();
                let flow_info = buf.get_u32();
                let mut octets = [0u8; 16];
                buf.copy_to_slice(&mut octets);
                let scope_id = buf.get_u32();
                Ok(Self(SocketAddr::V6(SocketAddrV6::new(
                    Ipv6Addr::from(octets),
                    port,
                    flow_info,
                    scope_id,
                ))))
            }
            v => Err(RakNetError::InvalidAddressVersion(v)),
        }
    }
}

impl From<SocketAddr> for RakNetAddress {
    fn from(addr: SocketAddr) -> Self {
        Self(addr)
    }
}

impl From<RakNetAddress> for SocketAddr {
    fn from(addr: RakNetAddress) -> Self {
        addr.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use std::io::Cursor;

    #[test]
    fn ipv4_roundtrip() {
        let addr = RakNetAddress(SocketAddr::new("127.0.0.1".parse().unwrap(), 19132));
        let mut buf = BytesMut::new();
        addr.encode(&mut buf);
        // version(1) + ip(4) + port(2) = 7 bytes
        assert_eq!(buf.len(), 7);
        // Check that IP bytes are inverted
        assert_eq!(buf[1], !127);
        assert_eq!(buf[2], !0);
        assert_eq!(buf[3], !0);
        assert_eq!(buf[4], !1);

        let mut cursor = Cursor::new(&buf[..]);
        let decoded = RakNetAddress::decode(&mut cursor).unwrap();
        assert_eq!(decoded, addr);
    }

    #[test]
    fn ipv6_roundtrip() {
        let addr = RakNetAddress(SocketAddr::new("::1".parse().unwrap(), 19132));
        let mut buf = BytesMut::new();
        addr.encode(&mut buf);
        let mut cursor = Cursor::new(&buf[..]);
        let decoded = RakNetAddress::decode(&mut cursor).unwrap();
        assert_eq!(decoded, addr);
    }

    #[test]
    fn empty_v4() {
        let addr = RakNetAddress::EMPTY_V4;
        let mut buf = BytesMut::new();
        addr.encode(&mut buf);
        let mut cursor = Cursor::new(&buf[..]);
        let decoded = RakNetAddress::decode(&mut cursor).unwrap();
        assert_eq!(decoded, addr);
    }
}
