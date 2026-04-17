//! Bat — passive nocturnal mob.

#[derive(Debug, Clone)]
pub struct Bat {
    pub hanging: bool, // Sleeping on block
    pub hang_position: Option<(i32, i32, i32)>,
}

impl Bat {
    pub fn new() -> Self {
        Self {
            hanging: false,
            hang_position: None,
        }
    }

    pub fn hang(&mut self, pos: (i32, i32, i32)) {
        self.hanging = true;
        self.hang_position = Some(pos);
    }

    pub fn unhang(&mut self) {
        self.hanging = false;
        self.hang_position = None;
    }

    /// Bats don't spawn in bright areas.
    pub fn max_spawn_light() -> u8 {
        3
    }

    /// Spawns in caves.
    pub fn spawn_biomes() -> &'static [&'static str] {
        &["overworld_caves", "lush_caves"]
    }
}

impl Default for Bat {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hang_and_unhang() {
        let mut b = Bat::new();
        b.hang((0, 64, 0));
        assert!(b.hanging);
        b.unhang();
        assert!(!b.hanging);
    }
}
