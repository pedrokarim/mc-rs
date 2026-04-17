//! Wind charge — 1.21 projectile (pushes targets, no damage).

#[derive(Debug, Clone)]
pub struct WindCharge {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
    pub owner: u64,
    pub age: u32,
}

/// Wind charge knockback radius (2 blocks).
pub const KNOCKBACK_RADIUS: f64 = 2.0;
/// Explosion power (0.5).
pub const EXPLOSION_POWER: f32 = 0.5;
/// Damage on hit (1).
pub const DIRECT_DAMAGE: f32 = 1.0;

impl WindCharge {
    pub fn new(x: f64, y: f64, z: f64, mx: f64, my: f64, mz: f64, owner: u64) -> Self {
        Self {
            x,
            y,
            z,
            motion_x: mx,
            motion_y: my,
            motion_z: mz,
            owner,
            age: 0,
        }
    }

    pub fn tick(&mut self) {
        // No gravity for wind charges.
        self.x += self.motion_x;
        self.y += self.motion_y;
        self.z += self.motion_z;
        self.motion_x *= 0.99;
        self.motion_y *= 0.99;
        self.motion_z *= 0.99;
        self.age += 1;
    }

    /// Wind charges activate certain blocks (bell, button, etc.)
    pub fn activates_block(block_id: u16) -> bool {
        matches!(
            block_id,
            85 | 107    // fence gate
            | 64 | 193 | 194 | 195 | 196 | 197  // doors
            | 77 | 143  // buttons
            | 96  // trapdoor
            | 304 // lantern (swing)
            | 461 // bell
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bell_activated() {
        assert!(WindCharge::activates_block(461));
    }

    #[test]
    fn stone_not_activated() {
        assert!(!WindCharge::activates_block(1));
    }
}
