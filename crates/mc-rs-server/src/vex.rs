//! Vex — mob volant invocable par evoker. Attaque + disparaît.

#[derive(Debug, Clone)]
pub struct Vex {
    pub life_ticks: u32,
    pub max_life_ticks: u32,
    pub owner_entity: Option<u64>,
    pub charging: bool,
    pub charge_cooldown: u32,
    pub target_entity: Option<u64>,
}

/// Vex lifetime (parfois aucune — bound avec summoner).
pub const MAX_LIFE_TICKS: u32 = 2400; // 2 min
/// Charge cooldown.
pub const CHARGE_COOLDOWN: u32 = 20;
/// Damage (9 on charge).
pub const CHARGE_DAMAGE: f32 = 9.0;

impl Vex {
    pub fn new(owner: u64) -> Self {
        Self {
            life_ticks: 0,
            max_life_ticks: MAX_LIFE_TICKS,
            owner_entity: Some(owner),
            charging: false,
            charge_cooldown: 0,
            target_entity: None,
        }
    }

    pub fn tick(&mut self) {
        self.life_ticks += 1;
        if self.charge_cooldown > 0 {
            self.charge_cooldown -= 1;
        }
    }

    pub fn is_expired(&self) -> bool {
        self.life_ticks >= self.max_life_ticks
    }

    pub fn try_charge(&mut self) -> bool {
        if self.charge_cooldown > 0 {
            return false;
        }
        self.charging = true;
        self.charge_cooldown = CHARGE_COOLDOWN;
        true
    }

    /// Passes through blocks.
    pub fn noclip() -> bool { true }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expires_after_lifetime() {
        let mut v = Vex::new(1);
        v.life_ticks = MAX_LIFE_TICKS;
        assert!(v.is_expired());
    }

    #[test]
    fn charging_cooldown() {
        let mut v = Vex::new(1);
        assert!(v.try_charge());
        assert!(!v.try_charge());
    }
}
