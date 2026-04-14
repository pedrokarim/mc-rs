//! Beehive / Bee nest — holds up to 3 bees, honey level 0-5.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeeBlockKind {
    BeeNest,    // Natural, found in nature (oak/birch)
    Beehive,    // Crafted
}

#[derive(Debug, Clone)]
pub struct BeeBlock {
    pub kind: BeeBlockKind,
    pub bees_inside: Vec<u64>,
    pub honey_level: u8, // 0-5
    pub facing: u8,
}

/// Max bees.
pub const MAX_BEES: usize = 3;
/// Max honey.
pub const MAX_HONEY: u8 = 5;

impl BeeBlock {
    pub fn new(kind: BeeBlockKind) -> Self {
        Self {
            kind,
            bees_inside: Vec::with_capacity(MAX_BEES),
            honey_level: 0,
            facing: 0,
        }
    }

    pub fn add_bee(&mut self, bee_id: u64) -> bool {
        if self.bees_inside.len() >= MAX_BEES {
            return false;
        }
        self.bees_inside.push(bee_id);
        true
    }

    /// When bees leave, they deposit nectar — increases honey level.
    pub fn bee_leaves(&mut self, bee_id: u64, had_nectar: bool) {
        self.bees_inside.retain(|&id| id != bee_id);
        if had_nectar && self.honey_level < MAX_HONEY {
            self.honey_level += 1;
        }
    }

    /// Harvest honey — get 3 honey bottles when honey level is 5.
    pub fn harvest_with_bottle(&mut self) -> Option<&'static str> {
        if self.honey_level != MAX_HONEY {
            return None;
        }
        self.honey_level = 0;
        Some("minecraft:honey_bottle")
    }

    /// Shears turn into honeycomb (3 honeycombs at level 5).
    pub fn harvest_with_shears(&mut self) -> Option<u32> {
        if self.honey_level != MAX_HONEY {
            return None;
        }
        self.honey_level = 0;
        Some(3)
    }

    /// Angered bees if harvested without campfire below.
    pub fn angers_bees(campfire_below: bool) -> bool {
        !campfire_below
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_3_bees() {
        let mut b = BeeBlock::new(BeeBlockKind::Beehive);
        for i in 0..3 {
            assert!(b.add_bee(i));
        }
        assert!(!b.add_bee(3));
    }

    #[test]
    fn honey_full_harvestable() {
        let mut b = BeeBlock::new(BeeBlockKind::Beehive);
        b.honey_level = MAX_HONEY;
        assert!(b.harvest_with_bottle().is_some());
    }
}
