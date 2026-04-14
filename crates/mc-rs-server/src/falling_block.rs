//! FallingBlock — port PMMP `src/entity/object/FallingBlock.php`.
//! Sand/gravel/anvil qui tombent quand pas de support.

#[derive(Debug, Clone)]
pub struct FallingBlock {
    pub block_id: u16,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub motion_y: f64,
    pub ticks_alive: u32,
    pub damages_entities: bool,
    pub fall_distance: f32,
}

impl FallingBlock {
    pub fn new(block_id: u16, x: f64, y: f64, z: f64) -> Self {
        Self {
            block_id,
            x,
            y,
            z,
            motion_y: 0.0,
            ticks_alive: 0,
            damages_entities: Self::block_damages_entities(block_id),
            fall_distance: 0.0,
        }
    }

    fn block_damages_entities(id: u16) -> bool {
        matches!(id,
            145 // anvil
            | 146 // damaged anvil
            | 147 // very damaged anvil
        )
    }

    /// Gravity: 0.04, drag: 0.02 (vanilla).
    pub fn tick(&mut self) {
        self.motion_y -= 0.04;
        self.motion_y *= 0.98;
        self.y += self.motion_y;
        self.fall_distance += (-self.motion_y) as f32;
        self.ticks_alive += 1;
    }

    pub fn has_landed(&self, ground_y: f64) -> bool {
        self.y <= ground_y
    }

    /// Damages applied on landing (anvil = 2 * fall_distance, capped).
    pub fn landing_damage(&self) -> f32 {
        if !self.damages_entities {
            return 0.0;
        }
        (self.fall_distance * 2.0).min(40.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sand_no_damage() {
        let fb = FallingBlock::new(12, 0.0, 5.0, 0.0);
        assert!(!fb.damages_entities);
    }

    #[test]
    fn anvil_damages() {
        let fb = FallingBlock::new(145, 0.0, 5.0, 0.0);
        assert!(fb.damages_entities);
    }

    #[test]
    fn falling_accumulates_distance() {
        let mut fb = FallingBlock::new(12, 0.0, 100.0, 0.0);
        for _ in 0..10 {
            fb.tick();
        }
        assert!(fb.fall_distance > 0.0);
    }
}
