//! Stats — tracking per-player : distance walked, jumps, mobs killed, etc.
//! Port conceptuel (PMMP n'a pas d'équivalent standard).

use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatKind {
    DistanceWalked, // cm
    DistanceSprinted,
    DistanceSwum,
    DistanceFlown,
    DistanceByMinecart,
    DistanceByBoat,
    DistanceFallen,
    Jumps,
    Deaths,
    MobKills,
    PlayerKills,
    TimePlayed,    // ticks
    TimeSinceDeath,
    BlocksBroken,
    BlocksPlaced,
    ItemsCrafted,
    ItemsUsed,
    DamageDealt,
    DamageTaken,
    FishCaught,
}

#[derive(Debug, Default, Clone)]
pub struct PlayerStats {
    pub values: HashMap<StatKind, u64>,
}

impl PlayerStats {
    pub fn add(&mut self, kind: StatKind, amount: u64) {
        *self.values.entry(kind).or_insert(0) += amount;
    }

    pub fn get(&self, kind: StatKind) -> u64 {
        self.values.get(&kind).copied().unwrap_or(0)
    }

    pub fn set(&mut self, kind: StatKind, value: u64) {
        self.values.insert(kind, value);
    }
}

#[derive(Debug, Default)]
pub struct StatsManager {
    pub per_player: HashMap<SocketAddr, PlayerStats>,
}

impl StatsManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, addr: SocketAddr, kind: StatKind, amount: u64) {
        self.per_player.entry(addr).or_default().add(kind, amount);
    }

    pub fn get(&self, addr: &SocketAddr, kind: StatKind) -> u64 {
        self.per_player.get(addr).map_or(0, |p| p.get(kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::str::FromStr;

    #[test]
    fn stats_accumulate() {
        let mut mgr = StatsManager::new();
        let a = SocketAddr::from_str("127.0.0.1:1001").unwrap();
        mgr.add(a, StatKind::BlocksBroken, 5);
        mgr.add(a, StatKind::BlocksBroken, 3);
        assert_eq!(mgr.get(&a, StatKind::BlocksBroken), 8);
    }
}
