//! Panda — mob avec genes (traits).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PandaGene {
    Normal,
    Lazy,
    Worried,
    Playful,
    Weak,
    Aggressive,
    Brown,
}

impl PandaGene {
    /// Weight for random rolls when panda is born.
    pub fn roll_random() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let roll = rng.gen_range(0..100);
        match roll {
            0..=4 => Self::Brown,
            5..=9 => Self::Weak,
            10..=19 => Self::Aggressive,
            20..=34 => Self::Worried,
            35..=49 => Self::Playful,
            50..=69 => Self::Lazy,
            _ => Self::Normal,
        }
    }

    pub fn is_recessive(&self) -> bool {
        matches!(self, Self::Brown | Self::Weak)
    }
}

#[derive(Debug, Clone)]
pub struct Panda {
    pub age: i32,
    pub main_gene: PandaGene,
    pub hidden_gene: PandaGene,
    pub eating: bool,
    pub sneezing_ticks: u32,
    pub unhappy_ticks: u32,
}

/// Breeding item = bamboo.
pub const BREEDING_ITEM: u16 = 353;
/// Sneezing duration (1s).
pub const SNEEZE_DURATION: u32 = 20;

impl Panda {
    pub fn new() -> Self {
        let a = PandaGene::roll_random();
        let b = PandaGene::roll_random();
        Self {
            age: 0,
            main_gene: a,
            hidden_gene: b,
            eating: false,
            sneezing_ticks: 0,
            unhappy_ticks: 0,
        }
    }

    /// Expressed gene = main if dominant, else hidden ou fallback normal.
    pub fn expressed(&self) -> PandaGene {
        if !self.main_gene.is_recessive() {
            return self.main_gene;
        }
        if self.main_gene == self.hidden_gene {
            return self.main_gene;
        }
        if !self.hidden_gene.is_recessive() {
            return self.hidden_gene;
        }
        PandaGene::Normal
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.sneezing_ticks > 0 {
            self.sneezing_ticks -= 1;
        }
        if self.unhappy_ticks > 0 {
            self.unhappy_ticks -= 1;
        }
    }

    pub fn sneeze(&mut self) {
        self.sneezing_ticks = SNEEZE_DURATION;
    }
}

impl Default for Panda {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brown_is_recessive() {
        assert!(PandaGene::Brown.is_recessive());
    }

    #[test]
    fn dominant_main_expressed() {
        let mut p = Panda::new();
        p.main_gene = PandaGene::Playful;
        p.hidden_gene = PandaGene::Brown;
        assert_eq!(p.expressed(), PandaGene::Playful);
    }
}
