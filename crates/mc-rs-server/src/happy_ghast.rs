//! Happy Ghast — 1.22 mob pacifique que l'on peut monter.

#[derive(Debug, Clone)]
pub struct HappyGhast {
    pub age: i32,
    pub saddled: bool,
    pub harness_type: Option<&'static str>, // color variant
    pub rider: Option<u64>,
}

/// Ghastling grows into happy ghast.
pub const GHASTLING_GROW_TICKS: u32 = 24000 * 5; // 5 days
/// Breeding item = snowball to wet dried ghast.
pub fn hydration_item() -> &'static str { "minecraft:water_bucket" }

impl HappyGhast {
    pub fn new_adult() -> Self {
        Self {
            age: 0,
            saddled: false,
            harness_type: None,
            rider: None,
        }
    }

    pub fn new_ghastling() -> Self {
        Self {
            age: -24000 * 5,
            saddled: false,
            harness_type: None,
            rider: None,
        }
    }

    pub fn is_baby(&self) -> bool { self.age < 0 }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
    }

    pub fn equip_harness(&mut self, color: &'static str) -> bool {
        if self.is_baby() {
            return false;
        }
        self.harness_type = Some(color);
        self.saddled = true;
        true
    }

    pub fn mount(&mut self, player: u64) -> bool {
        if !self.saddled || self.rider.is_some() {
            return false;
        }
        self.rider = Some(player);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghastling_cant_wear_harness() {
        let mut g = HappyGhast::new_ghastling();
        assert!(!g.equip_harness("white"));
    }

    #[test]
    fn mount_requires_saddle() {
        let mut g = HappyGhast::new_adult();
        assert!(!g.mount(1));
        g.equip_harness("white");
        assert!(g.mount(1));
    }
}
