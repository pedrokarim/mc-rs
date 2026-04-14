//! Compass — points to spawn (overworld) or random in nether.

#[derive(Debug, Clone)]
pub struct Compass {
    pub target: CompassTarget,
}

#[derive(Debug, Clone)]
pub enum CompassTarget {
    WorldSpawn,
    LodestoneFor((i32, i32, i32), String), // pos + dimension
    None,
}

/// In nether/end, normal compass spins randomly.
pub fn spins_in_dimension(dimension: &str) -> bool {
    matches!(dimension, "nether" | "the_end")
}

impl Compass {
    pub fn new() -> Self {
        Self { target: CompassTarget::WorldSpawn }
    }

    pub fn link_to_lodestone(&mut self, pos: (i32, i32, i32), dim: String) {
        self.target = CompassTarget::LodestoneFor(pos, dim);
    }

    pub fn unlink(&mut self) {
        self.target = CompassTarget::WorldSpawn;
    }
}

impl Default for Compass {
    fn default() -> Self { Self::new() }
}

/// Recovery compass — added in 1.19, points to last death position.
#[derive(Debug, Clone)]
pub struct RecoveryCompass {
    pub last_death_pos: Option<(i32, i32, i32, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nether_spins() {
        assert!(spins_in_dimension("nether"));
    }

    #[test]
    fn overworld_works() {
        assert!(!spins_in_dimension("overworld"));
    }
}
