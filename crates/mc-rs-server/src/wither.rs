//! Wither — boss 3 heads, wither skulls projectiles.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitherHead {
    Center,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct Wither {
    pub hp: f32,
    pub invul_ticks: u32,  // spawn invulnerability + charge phase
    pub shield_ticks: u32, // when hp < half
    pub head_targets: [Option<u64>; 3],
}

/// Max HP normal (300).
pub const HP_MAX: f32 = 300.0;
/// Spawn charge invul duration (220 ticks).
pub const SPAWN_INVUL_TICKS: u32 = 220;
/// Explosion power on spawn (7.0).
pub const SPAWN_EXPLOSION_POWER: f32 = 7.0;
/// Wither skull damage.
pub const BLUE_SKULL_DAMAGE: f32 = 8.0;
pub const BLACK_SKULL_DAMAGE: f32 = 4.0;
/// Regeneration per 20 ticks (1 hp).
pub const REGEN_PER_SECOND: f32 = 1.0;
/// When HP < half, shield prevents arrows.
pub const SHIELD_HP_THRESHOLD: f32 = 150.0;

impl Wither {
    pub fn new_at_spawn() -> Self {
        Self {
            hp: HP_MAX,
            invul_ticks: SPAWN_INVUL_TICKS,
            shield_ticks: 0,
            head_targets: [None; 3],
        }
    }

    pub fn tick(&mut self) {
        if self.invul_ticks > 0 {
            self.invul_ticks -= 1;
        }
        // Regen 1 HP every 20 ticks (Bedrock scaled).
        if self.hp < HP_MAX && self.invul_ticks == 0 {
            self.hp = (self.hp + 1.0 / 20.0).min(HP_MAX);
        }
        if self.hp < SHIELD_HP_THRESHOLD {
            self.shield_ticks = 1;
        }
    }

    pub fn has_shield(&self) -> bool {
        self.hp < SHIELD_HP_THRESHOLD
    }

    pub fn is_invulnerable(&self) -> bool {
        self.invul_ticks > 0
    }

    pub fn take_damage(&mut self, amount: f32, arrow: bool) -> bool {
        if self.is_invulnerable() {
            return false;
        }
        if arrow && self.has_shield() {
            return false;
        }
        self.hp = (self.hp - amount).max(0.0);
        true
    }

    /// Shockwave when enough damage (Bedrock — explosion when ~1/2 HP).
    pub fn shockwave_hp_threshold() -> f32 {
        150.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invul_on_spawn() {
        let w = Wither::new_at_spawn();
        assert!(w.is_invulnerable());
    }

    #[test]
    fn arrow_blocked_with_shield() {
        let mut w = Wither::new_at_spawn();
        w.invul_ticks = 0;
        w.hp = 100.0;
        assert!(!w.take_damage(5.0, true));
    }

    #[test]
    fn melee_passes_shield() {
        let mut w = Wither::new_at_spawn();
        w.invul_ticks = 0;
        w.hp = 100.0;
        assert!(w.take_damage(5.0, false));
    }
}
