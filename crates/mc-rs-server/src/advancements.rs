//! Advancements / Achievements — port conceptuel (PMMP n'a pas d'équivalent).
//! Bedrock utilise un système "achievement" plus simple, avec des badges
//! débloqués par événements.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AchievementKind {
    OpenInventory,      // Taking Inventory
    MineWood,           // Getting Wood
    BuildWorkbench,     // Benchmarking
    BuildPickaxe,       // Time to Mine!
    BuildFurnace,       // Hot Topic
    AcquireIron,        // Acquire Hardware
    BuildHoe,           // Time to Farm!
    MakeBread,          // Bake Bread
    BakeCake,           // The Lie
    BuildBetterPickaxe, // Getting an Upgrade
    CookFish,           // Delicious Fish
    OnARail,            // On A Rail (travel cart 1km)
    BuildSword,         // Time to Strike!
    KillEnemy,          // Monster Hunter
    KillCow,            // Cow Tipper
    DiamondsToYou,      // Sniper Duel
    MakeMap,            // Map Room
    PortalMaker,        // Portal Maker
    PotionEffect,       // Local Brewery
    SpawnWither,        // The Beginning?
    KillWither,         // The Beginning.
    FullBeacon,         // Beaconator
    Overkill,
    BookcaseBuilder,
    ReturnToSender,
    Overpowered,
}

/// Manager per-player achievements. PMMP-like `AchievementManager`.
#[derive(Debug, Default)]
pub struct AchievementManager {
    pub unlocked: HashMap<SocketAddr, HashSet<AchievementKind>>,
}

impl AchievementManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn unlock(&mut self, addr: SocketAddr, kind: AchievementKind) -> bool {
        let set = self.unlocked.entry(addr).or_default();
        set.insert(kind)
    }

    pub fn has(&self, addr: &SocketAddr, kind: AchievementKind) -> bool {
        self.unlocked.get(addr).is_some_and(|s| s.contains(&kind))
    }

    pub fn count(&self, addr: &SocketAddr) -> usize {
        self.unlocked.get(addr).map_or(0, |s| s.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::str::FromStr;

    #[test]
    fn unlock_once() {
        let mut mgr = AchievementManager::new();
        let addr = SocketAddr::from_str("127.0.0.1:1001").unwrap();
        assert!(mgr.unlock(addr, AchievementKind::OpenInventory));
        assert!(!mgr.unlock(addr, AchievementKind::OpenInventory));
        assert!(mgr.has(&addr, AchievementKind::OpenInventory));
    }
}
