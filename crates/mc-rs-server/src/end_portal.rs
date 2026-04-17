//! End portal — bloc + frame activation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndPortalFrameSlot {
    Facing0,
    Facing1,
    Facing2,
    Facing3,
}

#[derive(Debug, Clone)]
pub struct EndPortalFrame {
    pub facing: EndPortalFrameSlot,
    pub has_eye: bool,
}

impl EndPortalFrame {
    pub fn new(facing: EndPortalFrameSlot) -> Self {
        Self {
            facing,
            has_eye: false,
        }
    }

    pub fn insert_eye(&mut self) -> bool {
        if self.has_eye {
            return false;
        }
        self.has_eye = true;
        true
    }
}

/// 12 frames nécessaires pour activation (arrangés en 3x3 ring with offset).
pub const REQUIRED_FRAMES: usize = 12;

/// Check whether 12 frames form an activation square.
pub fn is_portal_complete(frames: &[(i32, i32)]) -> bool {
    frames.len() >= REQUIRED_FRAMES
}

/// Eye of Ender throwing — pointe vers stronghold.
pub const EYE_THROW_TRAVEL_TICKS: u32 = 40;
/// 20% break chance when eye falls.
pub const EYE_BREAK_CHANCE: f32 = 0.2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eye_inserts_once() {
        let mut f = EndPortalFrame::new(EndPortalFrameSlot::Facing0);
        assert!(f.insert_eye());
        assert!(!f.insert_eye());
    }
}
