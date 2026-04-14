//! Piglin — gold attraction + barter.

#[derive(Debug, Clone)]
pub struct Piglin {
    pub age: i32,
    pub is_admiring_item: bool,
    pub admire_ticks: u32,
    pub held_item: Option<u16>,
    pub target_entity: Option<u64>,
    pub zombification_ticks: u32,
}

/// Admiring duration when they pick up a gold item (100 ticks = 5s).
pub const ADMIRE_DURATION: u32 = 100;
/// Zombification ticks (300 ticks in Overworld).
pub const ZOMBIFY_TICKS: u32 = 300;
/// Damage in hard diff.
pub const DAMAGE_HARD: f32 = 8.0;

impl Piglin {
    pub fn new_adult() -> Self {
        Self {
            age: 0,
            is_admiring_item: false,
            admire_ticks: 0,
            held_item: None,
            target_entity: None,
            zombification_ticks: 0,
        }
    }

    pub fn new_baby() -> Self {
        Self {
            age: -24000,
            is_admiring_item: false,
            admire_ticks: 0,
            held_item: None,
            target_entity: None,
            zombification_ticks: 0,
        }
    }

    pub fn is_baby(&self) -> bool {
        self.age < 0
    }

    pub fn tick(&mut self, in_overworld: bool) -> PiglinEvent {
        if self.age < 0 {
            self.age += 1;
        }
        if self.admire_ticks > 0 {
            self.admire_ticks -= 1;
            if self.admire_ticks == 0 {
                self.is_admiring_item = false;
                return PiglinEvent::FinishedAdmiring;
            }
        }
        if in_overworld {
            self.zombification_ticks += 1;
            if self.zombification_ticks >= ZOMBIFY_TICKS {
                return PiglinEvent::Zombify;
            }
        } else {
            self.zombification_ticks = 0;
        }
        PiglinEvent::Tick
    }

    pub fn admire(&mut self, item: u16) {
        self.is_admiring_item = true;
        self.admire_ticks = ADMIRE_DURATION;
        self.held_item = Some(item);
    }

    /// Baby piglins don't zombify.
    pub fn baby_never_zombifies() -> bool { false } // Actually they do zombify
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiglinEvent {
    Tick,
    FinishedAdmiring,
    Zombify,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zombifies_in_overworld() {
        let mut p = Piglin::new_adult();
        for _ in 0..ZOMBIFY_TICKS {
            p.tick(true);
        }
        assert_eq!(p.tick(true), PiglinEvent::Zombify);
    }
}
