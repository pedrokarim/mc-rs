//! Vines — climbable, spread.

#[derive(Debug, Clone)]
pub struct Vine {
    pub north: bool,
    pub south: bool,
    pub east: bool,
    pub west: bool,
    pub up: bool,
}

/// Growth chance per random tick.
pub const GROWTH_CHANCE: f32 = 0.25;
/// Max distance from supporting block before removed.
pub const MAX_DISTANCE: u8 = 5;

impl Vine {
    pub fn new() -> Self {
        Self {
            north: false,
            south: false,
            east: false,
            west: false,
            up: false,
        }
    }

    pub fn has_any_face(&self) -> bool {
        self.north || self.south || self.east || self.west || self.up
    }

    pub fn face_count(&self) -> u8 {
        let mut count = 0;
        if self.north {
            count += 1;
        }
        if self.south {
            count += 1;
        }
        if self.east {
            count += 1;
        }
        if self.west {
            count += 1;
        }
        if self.up {
            count += 1;
        }
        count
    }
}

impl Default for Vine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_no_faces() {
        assert!(!Vine::new().has_any_face());
    }
}
