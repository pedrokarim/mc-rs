//! Target block — emits redstone when projectile hit.

#[derive(Debug, Clone, Copy)]
pub struct TargetHit {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub face: u8,
}

/// Calculate signal strength based on hit offset from center (0-15).
pub fn signal_from_hit(hit: TargetHit, block_x: f64, block_y: f64, block_z: f64) -> u8 {
    let dx = (hit.x - (block_x + 0.5)).abs();
    let dy = (hit.y - (block_y + 0.5)).abs();
    let dz = (hit.z - (block_z + 0.5)).abs();
    let max = dx.max(dy).max(dz);
    let strength = ((1.0 - (max * 2.0).min(1.0)) * 15.0).round() as u8;
    strength.min(15)
}

/// Signal duration (8 ticks).
pub const SIGNAL_DURATION: u32 = 8;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_hit_max_signal() {
        let hit = TargetHit {
            x: 0.5,
            y: 0.5,
            z: 0.5,
            face: 1,
        };
        assert_eq!(signal_from_hit(hit, 0.0, 0.0, 0.0), 15);
    }

    #[test]
    fn edge_hit_zero_signal() {
        let hit = TargetHit {
            x: 1.0,
            y: 0.5,
            z: 0.5,
            face: 1,
        };
        let sig = signal_from_hit(hit, 0.0, 0.0, 0.0);
        assert_eq!(sig, 0);
    }
}
