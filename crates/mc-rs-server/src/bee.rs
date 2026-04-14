//! Bee — pollinisation, angry quand nest cassé.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeeActivity {
    Idle,
    SeekingFlower,
    Pollinating,
    ReturningToNest,
    AngryAtPlayer,
}

#[derive(Debug, Clone)]
pub struct Bee {
    pub activity: BeeActivity,
    pub has_nectar: bool,
    pub pollination_ticks: u32,
    pub anger_ticks: u32,
    pub target_entity: Option<u64>,
    pub target_flower: Option<(i32, i32, i32)>,
    pub target_hive: Option<(i32, i32, i32)>,
    pub cannot_enter_hive_ticks: u32,
}

/// Pollination duration (5 seconds = 100 ticks).
pub const POLLINATION_DURATION: u32 = 100;
/// Sting cooldown / anger.
pub const ANGER_DURATION: u32 = 500;
/// Can't find hive cooldown.
pub const NO_HIVE_COOLDOWN: u32 = 600;

impl Bee {
    pub fn new() -> Self {
        Self {
            activity: BeeActivity::Idle,
            has_nectar: false,
            pollination_ticks: 0,
            anger_ticks: 0,
            target_entity: None,
            target_flower: None,
            target_hive: None,
            cannot_enter_hive_ticks: 0,
        }
    }

    pub fn tick(&mut self) {
        if self.anger_ticks > 0 {
            self.anger_ticks -= 1;
            if self.anger_ticks == 0 {
                self.activity = BeeActivity::Idle;
                self.target_entity = None;
            }
        }
        if self.cannot_enter_hive_ticks > 0 {
            self.cannot_enter_hive_ticks -= 1;
        }
        if self.activity == BeeActivity::Pollinating {
            self.pollination_ticks += 1;
            if self.pollination_ticks >= POLLINATION_DURATION {
                self.has_nectar = true;
                self.pollination_ticks = 0;
                self.activity = BeeActivity::ReturningToNest;
            }
        }
    }

    pub fn anger_at(&mut self, target: u64) {
        self.anger_ticks = ANGER_DURATION;
        self.target_entity = Some(target);
        self.activity = BeeActivity::AngryAtPlayer;
    }

    pub fn enter_hive(&mut self) -> bool {
        if self.cannot_enter_hive_ticks > 0 {
            return false;
        }
        self.has_nectar = false;
        self.activity = BeeActivity::Idle;
        true
    }

    pub fn is_angry(&self) -> bool {
        self.anger_ticks > 0
    }

    /// Bees sting and lose their life (vanilla).
    pub fn dies_after_sting() -> bool {
        true
    }
}

impl Default for Bee {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pollination_fills_nectar() {
        let mut b = Bee::new();
        b.activity = BeeActivity::Pollinating;
        for _ in 0..=POLLINATION_DURATION {
            b.tick();
        }
        assert!(b.has_nectar);
    }

    #[test]
    fn anger_expires() {
        let mut b = Bee::new();
        b.anger_at(1);
        for _ in 0..ANGER_DURATION {
            b.tick();
        }
        assert!(!b.is_angry());
    }
}
