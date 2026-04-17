//! Creeper — port PMMP. Explosion + charged variant (lightning).

#[derive(Debug, Clone)]
pub struct Creeper {
    pub fuse_ticks: u32,
    pub ignited: bool,
    pub charged: bool,
}

/// Fuse duration (30 ticks vanilla).
pub const FUSE_DURATION: u32 = 30;
/// Explosion power normal (3.0).
pub const POWER_NORMAL: f32 = 3.0;
/// Explosion power charged (6.0).
pub const POWER_CHARGED: f32 = 6.0;
/// Ignite range (3 blocs player proximity).
pub const IGNITE_RANGE: f64 = 3.0;

impl Creeper {
    pub fn new() -> Self {
        Self {
            fuse_ticks: 0,
            ignited: false,
            charged: false,
        }
    }

    /// Charge this creeper (lightning bolt).
    pub fn charge(&mut self) {
        self.charged = true;
    }

    pub fn ignite(&mut self) {
        if !self.ignited {
            self.ignited = true;
            self.fuse_ticks = FUSE_DURATION;
        }
    }

    /// Cancel ignition (player moves away).
    pub fn defuse(&mut self) {
        self.ignited = false;
        self.fuse_ticks = 0;
    }

    pub fn tick(&mut self) -> bool {
        if !self.ignited {
            return false;
        }
        if self.fuse_ticks > 0 {
            self.fuse_ticks -= 1;
        }
        self.fuse_ticks == 0
    }

    pub fn explosion_power(&self) -> f32 {
        if self.charged {
            POWER_CHARGED
        } else {
            POWER_NORMAL
        }
    }

    /// Head drop chance when killed by charged creeper (100%).
    pub fn head_drop_chance_charged() -> f32 {
        1.0
    }
}

impl Default for Creeper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explodes_after_fuse() {
        let mut c = Creeper::new();
        c.ignite();
        let mut exploded = false;
        for _ in 0..=FUSE_DURATION {
            if c.tick() {
                exploded = true;
                break;
            }
        }
        assert!(exploded);
    }

    #[test]
    fn charged_more_power() {
        let mut c = Creeper::new();
        c.charge();
        assert!(c.explosion_power() > POWER_NORMAL);
    }

    #[test]
    fn defusing_cancels() {
        let mut c = Creeper::new();
        c.ignite();
        c.defuse();
        assert_eq!(c.fuse_ticks, 0);
        assert!(!c.ignited);
    }
}
