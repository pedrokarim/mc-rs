//! Cave vines — grow down from ceiling, have berries.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VineState {
    Empty,
    Berries,
}

#[derive(Debug, Clone)]
pub struct CaveVine {
    pub state: VineState,
    pub berry_growth_chance: f32,
}

/// Berry growth chance per random tick (0.11).
pub const BERRY_GROWTH: f32 = 0.11;
/// Light emission with berries (14).
pub const BERRY_LIGHT: u8 = 14;

impl CaveVine {
    pub fn new() -> Self {
        Self { state: VineState::Empty, berry_growth_chance: BERRY_GROWTH }
    }

    pub fn light_emission(&self) -> u8 {
        if self.state == VineState::Berries {
            BERRY_LIGHT
        } else {
            0
        }
    }

    pub fn harvest_berries(&mut self) -> u32 {
        if self.state == VineState::Berries {
            self.state = VineState::Empty;
            2 // 1-2 berries vanilla
        } else {
            0
        }
    }
}

impl Default for CaveVine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn berries_emit_light() {
        let mut v = CaveVine::new();
        v.state = VineState::Berries;
        assert_eq!(v.light_emission(), BERRY_LIGHT);
    }
}
