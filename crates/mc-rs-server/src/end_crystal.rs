//! EndCrystal — port PMMP `src/entity/object/EndCrystal.php`.
//! Crystal de l'End : explosion massive quand détruit, heal le dragon.

#[derive(Debug, Clone)]
pub struct EndCrystal {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub show_base: bool,
    pub beam_target: Option<(i32, i32, i32)>,
    pub health: u16,
}

/// Explosion power lors de destruction (6.0 vanilla).
pub const EXPLOSION_POWER: f32 = 6.0;

impl EndCrystal {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x,
            y,
            z,
            show_base: true,
            beam_target: None,
            health: 1,
        }
    }

    pub fn take_damage(&mut self, amount: u16) -> bool {
        self.health = self.health.saturating_sub(amount);
        self.health == 0
    }

    pub fn heals_dragon(&self) -> bool {
        self.beam_target.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crystal_destroyed_by_any_hit() {
        let mut c = EndCrystal::new(0.0, 0.0, 0.0);
        assert!(c.take_damage(1));
    }
}
