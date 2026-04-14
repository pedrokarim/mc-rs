//! Chunk ticket system (force-loading).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkTicketType {
    Player,        // Near player
    ForceLoaded,   // Admin command
    Spawn,         // World spawn
    Portal,        // Nether/end portal
    Dragon,        // Ender dragon spawning
    PostTeleport,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ChunkTicket {
    pub ticket_type: ChunkTicketType,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub level: u8, // lower = stronger
    pub ttl_ticks: u32,
}

/// Max level (0=tick, 29=border).
pub const TICKING_LEVEL: u8 = 31;
pub const BORDER_LEVEL: u8 = 33;
pub const UNLOAD_LEVEL: u8 = 44;

impl ChunkTicket {
    pub fn new_player(chunk_x: i32, chunk_z: i32) -> Self {
        Self {
            ticket_type: ChunkTicketType::Player,
            chunk_x, chunk_z,
            level: 31,
            ttl_ticks: u32::MAX,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_ticket_has_ticking_level() {
        let t = ChunkTicket::new_player(0, 0);
        assert_eq!(t.level, 31);
    }
}
