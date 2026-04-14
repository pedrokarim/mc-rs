//! Sleep / Bed system — port PMMP `src/block/Bed.php` + `src/player/Player.php::sleepOn`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct SleepState {
    pub bed_position: [i32; 3],
    pub started_at: Instant,
}

#[derive(Debug, Clone, Default)]
pub struct SleepManager {
    pub sleepers: HashMap<SocketAddr, SleepState>,
}

impl SleepManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Un joueur se couche. Retourne true si tous les joueurs dorment
    /// (condition pour faire passer la nuit).
    pub fn player_sleep(
        &mut self,
        addr: SocketAddr,
        bed_position: [i32; 3],
        total_players: usize,
    ) -> bool {
        self.sleepers.insert(
            addr,
            SleepState {
                bed_position,
                started_at: Instant::now(),
            },
        );
        self.sleepers.len() >= total_players && total_players > 0
    }

    pub fn player_wake(&mut self, addr: SocketAddr) {
        self.sleepers.remove(&addr);
    }

    pub fn is_sleeping(&self, addr: &SocketAddr) -> bool {
        self.sleepers.contains_key(addr)
    }

    pub fn clear(&mut self) {
        self.sleepers.clear();
    }
}

/// Condition pour dormir : il doit faire nuit (time >= 12541 && time <= 23458).
pub fn can_sleep(game_time: i32) -> bool {
    let t = game_time.rem_euclid(24000);
    t >= 12541 && t <= 23458
}

/// Passer au matin (PMMP `sleep_until_morning`). Retourne le nouveau game_time.
pub fn skip_night_to(game_time: i32) -> i32 {
    let day = game_time / 24000;
    (day + 1) * 24000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_detection() {
        assert!(!can_sleep(6000));
        assert!(can_sleep(13000));
        assert!(can_sleep(20000));
        assert!(!can_sleep(1000));
    }

    #[test]
    fn skip_night() {
        assert_eq!(skip_night_to(15000), 24000);
        assert_eq!(skip_night_to(38000), 48000);
    }

    #[test]
    fn all_players_sleeping_triggers_skip() {
        use std::net::SocketAddr;
        use std::str::FromStr;
        let mut mgr = SleepManager::new();
        let addr1 = SocketAddr::from_str("127.0.0.1:1001").unwrap();
        let addr2 = SocketAddr::from_str("127.0.0.1:1002").unwrap();
        assert!(!mgr.player_sleep(addr1, [0, 64, 0], 2));
        assert!(mgr.player_sleep(addr2, [5, 64, 0], 2));
    }
}
