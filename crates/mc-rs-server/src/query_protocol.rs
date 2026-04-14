//! Query protocol (external server stat query).

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

#[cfg(test)]
mod tests {
    #[test]
    fn magic_header_correct() {
        assert_eq!(super::MAGIC_HEADER, [0xfe, 0xfd]);
    }
}
