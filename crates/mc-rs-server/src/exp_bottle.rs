//! Bottle o' Enchanting — thrown, drops 3-11 XP.

#[derive(Debug, Clone)]
pub struct ExperienceBottle {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
    pub thrower: u64,
    pub age: u32,
}

/// XP range per bottle.
pub const XP_MIN: u32 = 3;
pub const XP_MAX: u32 = 11;

impl ExperienceBottle {
    pub fn new(x: f64, y: f64, z: f64, mx: f64, my: f64, mz: f64, thrower: u64) -> Self {
        Self {
            x,
            y,
            z,
            motion_x: mx,
            motion_y: my,
            motion_z: mz,
            thrower,
            age: 0,
        }
    }

    pub fn tick(&mut self) {
        self.motion_y -= 0.07;
        self.motion_x *= 0.99;
        self.motion_y *= 0.99;
        self.motion_z *= 0.99;
        self.x += self.motion_x;
        self.y += self.motion_y;
        self.z += self.motion_z;
        self.age += 1;
    }

    pub fn roll_xp() -> u32 {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_range(XP_MIN..=XP_MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xp_in_range() {
        for _ in 0..100 {
            let xp = ExperienceBottle::roll_xp();
            assert!((XP_MIN..=XP_MAX).contains(&xp));
        }
    }
}
