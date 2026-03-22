use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicI64, Ordering};

/// Global runtime entity ID counter. Each player gets a unique ID.
static NEXT_ENTITY_ID: AtomicI64 = AtomicI64::new(1);

pub fn next_entity_id() -> i64 {
    NEXT_ENTITY_ID.fetch_add(1, Ordering::Relaxed)
}

/// Info about a connected player, shared across the server.
#[derive(Clone)]
pub struct PlayerInfo {
    pub addr: SocketAddr,
    pub name: String,
    pub uuid: [u8; 16],
    pub xuid: String,
    pub entity_id: i64,
    pub position: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
}

/// Tracks all connected players.
pub struct PlayerRegistry {
    pub players: HashMap<SocketAddr, PlayerInfo>,
}

impl Default for PlayerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerRegistry {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
        }
    }

    /// Register a new player. Returns the assigned entity ID.
    pub fn add(
        &mut self,
        addr: SocketAddr,
        name: String,
        uuid: [u8; 16],
        xuid: String,
        position: [f32; 3],
    ) -> i64 {
        let entity_id = next_entity_id();
        self.players.insert(
            addr,
            PlayerInfo {
                addr,
                name,
                uuid,
                xuid,
                entity_id,
                position,
                pitch: 0.0,
                yaw: 0.0,
                head_yaw: 0.0,
            },
        );
        entity_id
    }

    /// Remove a player. Returns the PlayerInfo if found.
    pub fn remove(&mut self, addr: &SocketAddr) -> Option<PlayerInfo> {
        self.players.remove(addr)
    }

    /// Update a player's position.
    pub fn update_position(
        &mut self,
        addr: &SocketAddr,
        pos: [f32; 3],
        pitch: f32,
        yaw: f32,
        head_yaw: f32,
    ) {
        if let Some(info) = self.players.get_mut(addr) {
            info.position = pos;
            info.pitch = pitch;
            info.yaw = yaw;
            info.head_yaw = head_yaw;
        }
    }

    /// Get all players except the given addr.
    pub fn others(&self, except: &SocketAddr) -> Vec<&PlayerInfo> {
        self.players
            .values()
            .filter(|p| &p.addr != except)
            .collect()
    }

    /// Get all players.
    pub fn all(&self) -> Vec<&PlayerInfo> {
        self.players.values().collect()
    }

    pub fn get(&self, addr: &SocketAddr) -> Option<&PlayerInfo> {
        self.players.get(addr)
    }

    pub fn count(&self) -> usize {
        self.players.len()
    }
}
