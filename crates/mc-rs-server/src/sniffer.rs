//! Sniffer — mob qui sniffe des ancient seeds.

use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnifferState {
    Idle,
    Sniffing,
    Digging,
    Happy,
}

#[derive(Debug, Clone)]
pub struct Sniffer {
    pub state: SnifferState,
    pub sniffing_cooldown: u32,
    pub digging_progress: u32,
    pub last_dig_pos: Option<(i32, i32, i32)>,
}

/// Durée du digging (80 ticks = 4s vanilla).
pub const DIGGING_DURATION: u32 = 80;
/// Cooldown entre sniff (randomized 240-600 ticks = 12-30s).
pub const SNIFF_COOLDOWN_MIN: u32 = 240;
pub const SNIFF_COOLDOWN_MAX: u32 = 600;

/// Loot possible (weighted).
pub fn sniffer_loot() -> &'static [(&'static str, u32)] {
    &[
        ("minecraft:torchflower_seeds", 6),
        ("minecraft:pitcher_pod", 2),
    ]
}

impl Sniffer {
    pub fn new() -> Self {
        Self {
            state: SnifferState::Idle,
            sniffing_cooldown: 0,
            digging_progress: 0,
            last_dig_pos: None,
        }
    }

    pub fn tick(&mut self) {
        if self.sniffing_cooldown > 0 {
            self.sniffing_cooldown -= 1;
        }
        if self.state == SnifferState::Digging {
            self.digging_progress += 1;
            if self.digging_progress >= DIGGING_DURATION {
                self.state = SnifferState::Happy;
                self.digging_progress = 0;
            }
        }
    }

    pub fn start_digging(&mut self, pos: (i32, i32, i32)) {
        let mut rng = rand::thread_rng();
        self.state = SnifferState::Digging;
        self.digging_progress = 0;
        self.last_dig_pos = Some(pos);
        self.sniffing_cooldown = rng.gen_range(SNIFF_COOLDOWN_MIN..=SNIFF_COOLDOWN_MAX);
    }

    /// Roll loot quand sniffer termine dig.
    pub fn roll_loot() -> Option<&'static str> {
        let mut rng = rand::thread_rng();
        let loot = sniffer_loot();
        let total: u32 = loot.iter().map(|(_, w)| *w).sum();
        let mut r = rng.gen_range(0..total);
        for (name, w) in loot {
            if r < *w {
                return Some(name);
            }
            r -= *w;
        }
        None
    }

    pub fn can_sniff(&self) -> bool {
        self.sniffing_cooldown == 0 && self.state == SnifferState::Idle
    }
}

impl Default for Sniffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digging_completes() {
        let mut s = Sniffer::new();
        s.start_digging((0, 0, 0));
        for _ in 0..=DIGGING_DURATION {
            s.tick();
        }
        assert_eq!(s.state, SnifferState::Happy);
    }

    #[test]
    fn loot_returns_something() {
        let item = Sniffer::roll_loot();
        assert!(item.is_some());
    }
}
