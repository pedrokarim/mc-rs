//! TNT — port PMMP `src/entity/object/PrimedTNT.php` & `src/block/TNT.php`.

#[derive(Debug, Clone)]
pub struct PrimedTnt {
    pub fuse_ticks: u32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
    pub source: Option<u64>,
    pub power: f32,
}

/// Default fuse (80 ticks = 4s vanilla).
pub const DEFAULT_FUSE: u32 = 80;
/// Explosion power (4.0 vanilla).
pub const DEFAULT_POWER: f32 = 4.0;
/// Upward momentum at prime.
pub const INITIAL_UPWARD: f64 = 0.2;

impl PrimedTnt {
    pub fn new(x: f64, y: f64, z: f64, source: Option<u64>) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let angle = rng.gen::<f64>() * std::f64::consts::TAU;
        Self {
            fuse_ticks: DEFAULT_FUSE,
            x,
            y,
            z,
            motion_x: angle.cos() * 0.02,
            motion_y: INITIAL_UPWARD,
            motion_z: angle.sin() * 0.02,
            source,
            power: DEFAULT_POWER,
        }
    }

    pub fn tick(&mut self) -> bool {
        self.fuse_ticks = self.fuse_ticks.saturating_sub(1);
        self.motion_y -= 0.04;
        self.x += self.motion_x;
        self.y += self.motion_y;
        self.z += self.motion_z;
        self.motion_x *= 0.98;
        self.motion_y *= 0.98;
        self.motion_z *= 0.98;
        self.fuse_ticks == 0
    }
}

/// Ender TNT, etc — different variants.
pub fn variants() -> &'static [&'static str] {
    &["minecraft:tnt", "minecraft:underwater_tnt"]
}

/// Underwater TNT: same explosion but doesn't destroy blocks underwater.
pub fn destroys_blocks_underwater(variant: &str) -> bool {
    variant != "minecraft:underwater_tnt"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuse_counts_down() {
        let mut t = PrimedTnt::new(0.0, 0.0, 0.0, None);
        t.fuse_ticks = 2;
        assert!(!t.tick());
        assert!(t.tick());
    }
}
