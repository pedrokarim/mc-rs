//! Axolotl — mob aquatique avec régénération + play dead.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxolotlVariant {
    LucyPink,
    Wild,
    Gold,
    Cyan,
    Blue,
}

impl AxolotlVariant {
    pub fn from_random() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let roll: u32 = rng.gen_range(0..1200);
        if roll < 1 {
            Self::Blue
        } else {
            let v: u8 = rng.gen_range(0..4);
            match v {
                0 => Self::LucyPink,
                1 => Self::Wild,
                2 => Self::Gold,
                _ => Self::Cyan,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Axolotl {
    pub variant: AxolotlVariant,
    pub age: i32,
    pub playing_dead: bool,
    pub play_dead_ticks: u32,
    pub time_out_of_water: u32,
}

/// Bucket of tropical fish = breeding item.
pub const BREEDING_ITEM: u16 = 374;
/// Play dead duration (10 seconds).
pub const PLAY_DEAD_DURATION: u32 = 200;
/// Play dead cooldown (5 minutes).
pub const PLAY_DEAD_COOLDOWN: u32 = 6000;
/// Time before axolotl starts dying out of water (5 min).
pub const OUT_OF_WATER_LIMIT: u32 = 6000;

impl Axolotl {
    pub fn new(variant: AxolotlVariant) -> Self {
        Self {
            variant,
            age: 0,
            playing_dead: false,
            play_dead_ticks: 0,
            time_out_of_water: 0,
        }
    }

    pub fn tick(&mut self, in_water: bool) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.playing_dead {
            self.play_dead_ticks += 1;
            if self.play_dead_ticks >= PLAY_DEAD_DURATION {
                self.playing_dead = false;
            }
        }
        if in_water {
            self.time_out_of_water = 0;
        } else {
            self.time_out_of_water += 1;
        }
    }

    pub fn should_take_damage_from_dry(&self) -> bool {
        self.time_out_of_water >= OUT_OF_WATER_LIMIT
    }

    pub fn start_play_dead(&mut self) {
        self.playing_dead = true;
        self.play_dead_ticks = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_dead_expires() {
        let mut a = Axolotl::new(AxolotlVariant::LucyPink);
        a.start_play_dead();
        for _ in 0..=PLAY_DEAD_DURATION {
            a.tick(true);
        }
        assert!(!a.playing_dead);
    }

    #[test]
    fn dies_out_of_water() {
        let mut a = Axolotl::new(AxolotlVariant::LucyPink);
        for _ in 0..OUT_OF_WATER_LIMIT {
            a.tick(false);
        }
        assert!(a.should_take_damage_from_dry());
    }
}
