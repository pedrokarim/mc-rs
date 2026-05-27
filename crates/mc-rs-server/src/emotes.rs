//! Emotes — port du système emote Bedrock. Emotes par UUID, stockés en
//! inventaire joueur slots (6-10 default in Bedrock).

use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct PlayerEmotes {
    pub emote_ids: Vec<uuid::Uuid>,
}

#[derive(Debug, Default)]
pub struct EmoteRegistry {
    pub per_player: HashMap<SocketAddr, PlayerEmotes>,
}

impl EmoteRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, addr: SocketAddr, emotes: Vec<uuid::Uuid>) {
        self.per_player
            .insert(addr, PlayerEmotes { emote_ids: emotes });
    }

    pub fn remove(&mut self, addr: &SocketAddr) {
        self.per_player.remove(addr);
    }

    /// Vérifie qu'un emote est possédé par le joueur avant broadcast.
    pub fn has(&self, addr: &SocketAddr, emote_id: &uuid::Uuid) -> bool {
        self.per_player
            .get(addr)
            .is_some_and(|p| p.emote_ids.contains(emote_id))
    }
}

/// Types d'emotes standards Bedrock (UUIDs fixés par Mojang).
pub mod standard_emotes {
    pub const HELLO: &str = "a5ac1d4a-13c0-5b9d-abc1-2b7f5a74d567";
    pub const WAVE: &str = "b7a1a9b8-c3d2-41c7-8d49-f7e25a1e9b22";
    pub const CLAP: &str = "fa3d7a85-9e84-442e-ab61-8c3a28d1a7b2";
    pub const HEART: &str = "54e7c80d-9a3a-4b0d-bc11-e6a43b5a91c2";
    pub const HI: &str = "a5ac1d4a-13c0-5b9d-abc1-2b7f5a74d567";
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::str::FromStr;

    #[test]
    fn register_and_check_emote() {
        let mut r = EmoteRegistry::new();
        let addr = SocketAddr::from_str("127.0.0.1:1001").unwrap();
        let id = uuid::Uuid::new_v4();
        r.register(addr, vec![id]);
        assert!(r.has(&addr, &id));
    }
}
