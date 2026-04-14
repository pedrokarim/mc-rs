//! ExperienceOrb — port PMMP `src/entity/object/ExperienceOrb.php`.

#[derive(Debug, Clone)]
pub struct ExperienceOrb {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub xp_value: u32,
    pub ticks_alive: u32,
    pub pickup_delay: u32,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
}

/// PMMP max age before despawn (5 minutes = 6000 ticks).
pub const MAX_AGE_TICKS: u32 = 6000;
/// XP range dans laquelle attirée vers joueur.
pub const ATTRACT_RANGE: f64 = 7.0;
/// Speed d'attraction.
pub const ATTRACT_SPEED: f64 = 0.125;

impl ExperienceOrb {
    pub fn new(x: f64, y: f64, z: f64, xp_value: u32) -> Self {
        Self {
            x,
            y,
            z,
            xp_value,
            ticks_alive: 0,
            pickup_delay: 10,
            motion_x: 0.0,
            motion_y: 0.0,
            motion_z: 0.0,
        }
    }

    /// Divise xp en orbes de taille vanilla (1,3,7,17,37,73,149,307,616,1237,2477).
    pub fn split_value(total: u32) -> Vec<u32> {
        const SIZES: &[u32] = &[2477, 1237, 616, 307, 149, 73, 37, 17, 7, 3, 1];
        let mut remaining = total;
        let mut orbs = Vec::new();
        for &sz in SIZES {
            while remaining >= sz {
                orbs.push(sz);
                remaining -= sz;
            }
        }
        orbs
    }

    pub fn tick(&mut self) {
        self.ticks_alive += 1;
        if self.pickup_delay > 0 {
            self.pickup_delay -= 1;
        }
        self.motion_y -= 0.04;
        self.x += self.motion_x;
        self.y += self.motion_y;
        self.z += self.motion_z;
        self.motion_x *= 0.98;
        self.motion_y *= 0.98;
        self.motion_z *= 0.98;
    }

    pub fn is_expired(&self) -> bool {
        self.ticks_alive >= MAX_AGE_TICKS
    }

    pub fn can_be_collected(&self) -> bool {
        self.pickup_delay == 0
    }

    /// Attire l'orbe vers un joueur (si dans range).
    pub fn attract_to(&mut self, px: f64, py: f64, pz: f64) {
        let dx = px - self.x;
        let dy = py - self.y;
        let dz = pz - self.z;
        let dist_sq = dx * dx + dy * dy + dz * dz;
        if dist_sq < ATTRACT_RANGE * ATTRACT_RANGE {
            let dist = dist_sq.sqrt();
            if dist > 0.01 {
                self.motion_x += dx / dist * ATTRACT_SPEED;
                self.motion_y += dy / dist * ATTRACT_SPEED;
                self.motion_z += dz / dist * ATTRACT_SPEED;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_small_values() {
        assert_eq!(ExperienceOrb::split_value(1), vec![1]);
        assert_eq!(ExperienceOrb::split_value(5), vec![3, 1, 1]);
    }

    #[test]
    fn split_large_conserves_total() {
        let orbs = ExperienceOrb::split_value(1000);
        let sum: u32 = orbs.iter().sum();
        assert_eq!(sum, 1000);
    }

    #[test]
    fn orb_pickup_delay_ticks_down() {
        let mut o = ExperienceOrb::new(0.0, 0.0, 0.0, 5);
        assert!(!o.can_be_collected());
        for _ in 0..11 {
            o.tick();
        }
        assert!(o.can_be_collected());
    }
}
