//! Villager reputation / gossip system.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GossipType {
    MajorPositive,  // Cure zombie villager
    MinorPositive,  // Trade
    MajorNegative,  // Kill villager
    MinorNegative,  // Hit villager
    Trading,
}

impl GossipType {
    pub fn weight(&self) -> i32 {
        match self {
            Self::MajorPositive => 5,
            Self::MinorPositive => 1,
            Self::MajorNegative => -25,
            Self::MinorNegative => -10,
            Self::Trading => 1,
        }
    }

    pub fn decay_per_day(&self) -> i32 {
        match self {
            Self::MajorPositive => 1,
            Self::MinorPositive => 1,
            Self::MajorNegative => 5,
            Self::MinorNegative => 2,
            Self::Trading => 1,
        }
    }

    pub fn max_value(&self) -> i32 {
        match self {
            Self::MajorPositive => 100,
            Self::MinorPositive => 200,
            Self::MajorNegative => 100,
            Self::MinorNegative => 200,
            Self::Trading => 20,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Gossip {
    pub entries: HashMap<GossipType, i32>,
}

impl Gossip {
    pub fn add(&mut self, kind: GossipType, amount: i32) {
        let current = self.entries.entry(kind).or_insert(0);
        *current = (*current + amount).min(kind.max_value());
    }

    pub fn reputation(&self) -> i32 {
        self.entries
            .iter()
            .map(|(k, v)| v * k.weight() / 5) // /5 factor because MajorPositive has weight 5
            .sum()
    }

    pub fn decay(&mut self) {
        for (kind, value) in self.entries.iter_mut() {
            let decay = kind.decay_per_day();
            *value = (*value - decay).max(0);
        }
    }
}

/// Per-player gossip per villager.
#[derive(Debug, Clone, Default)]
pub struct GossipBook {
    pub per_player: HashMap<u64, Gossip>,
}

impl GossipBook {
    pub fn add(&mut self, player: u64, kind: GossipType, amount: i32) {
        self.per_player.entry(player).or_default().add(kind, amount);
    }

    pub fn reputation(&self, player: u64) -> i32 {
        self.per_player.get(&player).map_or(0, |g| g.reputation())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_gossip_adds_rep() {
        let mut g = Gossip::default();
        g.add(GossipType::MajorPositive, 10);
        assert!(g.reputation() > 0);
    }

    #[test]
    fn negative_gossip_subtracts() {
        let mut g = Gossip::default();
        g.add(GossipType::MajorNegative, 10);
        assert!(g.reputation() < 0);
    }
}
