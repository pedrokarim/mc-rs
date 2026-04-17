//! End gateway — spawns after dragon death, teleports to outer islands.

#[derive(Debug, Clone)]
pub struct EndGateway {
    pub exit_position: (i32, i32, i32),
    pub age: u64, // Used for beam color animation
}

/// Gateway outer island min distance.
pub const OUTER_ISLAND_MIN_DIST: f64 = 1024.0;
/// Teleport range (ender pearl / player contact).
pub const TELEPORT_TRIGGER_RANGE: f64 = 0.5;

/// Generate next gateway exit (spiral outward).
pub fn next_exit_position(previous: (i32, i32, i32), gateway_index: u32) -> (i32, i32, i32) {
    let angle = (gateway_index as f64) * std::f64::consts::TAU / 20.0;
    let radius = OUTER_ISLAND_MIN_DIST + (gateway_index as f64) * 100.0;
    (
        previous.0 + (angle.cos() * radius) as i32,
        previous.1,
        previous.2 + (angle.sin() * radius) as i32,
    )
}

impl EndGateway {
    pub fn new(exit: (i32, i32, i32)) -> Self {
        Self {
            exit_position: exit,
            age: 0,
        }
    }

    pub fn tick(&mut self) {
        self.age += 1;
    }
}

/// Number of gateways around central island (20 vanilla).
pub const GATEWAY_COUNT: u32 = 20;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_exits_far_apart() {
        let a = next_exit_position((0, 0, 0), 0);
        let b = next_exit_position((0, 0, 0), 1);
        let dx = (a.0 - b.0) as f64;
        let dz = (a.2 - b.2) as f64;
        let dist = (dx * dx + dz * dz).sqrt();
        assert!(dist > 100.0);
    }
}
