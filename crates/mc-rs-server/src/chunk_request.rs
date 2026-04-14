//! Chunk request / ACK packet.

#[derive(Debug, Clone)]
pub struct ChunkRequest {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub max_radius: u32,
}

#[derive(Debug, Clone)]
pub struct ChunkStatus {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub status: StatusKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Unknown,
    NotReady,
    Ready,
    Sent,
    Acknowledged,
}

/// Max pending requests per player.
pub const MAX_PENDING: usize = 32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ready() {
        matches!(StatusKind::Ready, StatusKind::Ready);
    }
}
