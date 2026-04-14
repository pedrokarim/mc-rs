//! Creaking — 1.21.4 Pale Garden mob only moves when unseen.

#[derive(Debug, Clone)]
pub struct Creaking {
    pub heart_position: (i32, i32, i32), // Binding Creaking Heart
    pub attack_cooldown: u32,
    pub movement_frozen: bool, // true when any player sees it
    pub daytime_destroy: bool,
}

/// Attack damage (3).
pub const ATTACK_DAMAGE: f32 = 3.0;
/// Attack cooldown (20 ticks).
pub const ATTACK_COOLDOWN: u32 = 20;
/// Max distance from heart (32).
pub const MAX_DIST_FROM_HEART: f64 = 32.0;

impl Creaking {
    pub fn new(heart: (i32, i32, i32)) -> Self {
        Self {
            heart_position: heart,
            attack_cooldown: 0,
            movement_frozen: false,
            daytime_destroy: false,
        }
    }

    pub fn player_sight(&mut self, seen: bool) {
        self.movement_frozen = seen;
    }

    pub fn can_move(&self) -> bool {
        !self.movement_frozen && !self.daytime_destroy
    }

    pub fn tick(&mut self, in_daytime: bool) {
        if in_daytime {
            self.daytime_destroy = true;
        }
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }
    }

    /// Invulnerable to player damage (tied to heart).
    pub fn damage_goes_to_heart() -> bool { true }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cant_move_when_seen() {
        let mut c = Creaking::new((0, 0, 0));
        c.player_sight(true);
        assert!(!c.can_move());
    }

    #[test]
    fn daytime_destroys() {
        let mut c = Creaking::new((0, 0, 0));
        c.tick(true);
        assert!(c.daytime_destroy);
    }
}
