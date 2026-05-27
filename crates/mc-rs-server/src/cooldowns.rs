//! Cooldowns — shield use, ender pearl, chorus fruit, etc.

use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CooldownKind {
    EnderPearl,
    ChorusFruit,
    Shield,
    GoldenApple,
    Snowball,
    Egg,
    Trident,
    FishingRod,
}

impl CooldownKind {
    pub fn default_ticks(&self) -> u32 {
        match self {
            Self::EnderPearl => 20, // 1s
            Self::ChorusFruit => 20,
            Self::Shield => 100,    // 5s after hit
            Self::GoldenApple => 0, // no cooldown by itself
            Self::Snowball | Self::Egg => 4,
            Self::Trident => 20,
            Self::FishingRod => 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PlayerCooldowns {
    pub cooldowns: HashMap<CooldownKind, u32>,
}

impl PlayerCooldowns {
    pub fn set(&mut self, kind: CooldownKind, ticks: u32) {
        self.cooldowns.insert(kind, ticks);
    }

    pub fn is_on_cooldown(&self, kind: CooldownKind) -> bool {
        self.cooldowns.get(&kind).copied().unwrap_or(0) > 0
    }

    pub fn tick(&mut self) {
        self.cooldowns.retain(|_, v| {
            if *v > 0 {
                *v -= 1;
            }
            *v > 0
        });
    }
}

#[derive(Debug, Default)]
pub struct CooldownManager {
    pub per_player: HashMap<SocketAddr, PlayerCooldowns>,
}

impl CooldownManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trigger(&mut self, addr: SocketAddr, kind: CooldownKind) {
        let ticks = kind.default_ticks();
        if ticks > 0 {
            self.per_player.entry(addr).or_default().set(kind, ticks);
        }
    }

    pub fn tick_all(&mut self) {
        for p in self.per_player.values_mut() {
            p.tick();
        }
    }

    pub fn is_on_cooldown(&self, addr: &SocketAddr, kind: CooldownKind) -> bool {
        self.per_player
            .get(addr)
            .is_some_and(|p| p.is_on_cooldown(kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::str::FromStr;

    #[test]
    fn ender_pearl_cooldown_expires() {
        let mut mgr = CooldownManager::new();
        let a = SocketAddr::from_str("127.0.0.1:1001").unwrap();
        mgr.trigger(a, CooldownKind::EnderPearl);
        assert!(mgr.is_on_cooldown(&a, CooldownKind::EnderPearl));
        for _ in 0..21 {
            mgr.tick_all();
        }
        assert!(!mgr.is_on_cooldown(&a, CooldownKind::EnderPearl));
    }
}
