//! EnderDragon — boss de l'End avec phases, beam, crystals healing.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragonPhase {
    Holding,      // Circles in air, idle
    Strafing,     // Attacking flybys
    Landing,      // Coming to perch
    Sitting,      // On end portal, breath attack
    Roaring,      // Before breath
    Breath,       // Breathing fire
    TakingOff,    // Leaving perch
    ChargePlayer, // Direct charge
    Dying,        // Death animation
}

#[derive(Debug, Clone)]
pub struct EnderDragon {
    pub hp: f32,
    pub phase: DragonPhase,
    pub phase_ticks: u32,
    pub connected_crystals: Vec<u64>, // Entity IDs of crystals still alive
    pub cur_target: Option<u64>,
}

/// Max HP (200).
pub const HP_MAX: f32 = 200.0;
/// Crystal heal per tick (when connected to crystal, ~1 HP/sec).
pub const CRYSTAL_HEAL_PER_20: f32 = 1.0;
/// Dying phase duration (200 ticks = 10s).
pub const DYING_DURATION: u32 = 200;
/// Breath attack damage per tick.
pub const BREATH_DAMAGE_PER_TICK: f32 = 0.5;
/// Charge damage.
pub const CHARGE_DAMAGE: f32 = 10.0;

impl EnderDragon {
    pub fn new() -> Self {
        Self {
            hp: HP_MAX,
            phase: DragonPhase::Holding,
            phase_ticks: 0,
            connected_crystals: Vec::new(),
            cur_target: None,
        }
    }

    pub fn tick(&mut self) {
        self.phase_ticks += 1;
        // Heal from crystals.
        if !self.connected_crystals.is_empty() && self.phase != DragonPhase::Dying {
            let heal = CRYSTAL_HEAL_PER_20 / 20.0;
            self.hp = (self.hp + heal).min(HP_MAX);
        }
    }

    pub fn set_phase(&mut self, phase: DragonPhase) {
        self.phase = phase;
        self.phase_ticks = 0;
    }

    pub fn take_damage(&mut self, amount: f32, to_head: bool) -> bool {
        if self.phase == DragonPhase::Dying {
            return false;
        }
        // Only head takes full damage — body takes less.
        let effective = if to_head { amount } else { amount * 0.25 };
        self.hp = (self.hp - effective).max(0.0);
        if self.hp == 0.0 {
            self.set_phase(DragonPhase::Dying);
        }
        true
    }

    pub fn is_dying(&self) -> bool {
        self.phase == DragonPhase::Dying
    }

    pub fn should_despawn(&self) -> bool {
        self.phase == DragonPhase::Dying && self.phase_ticks >= DYING_DURATION
    }

    pub fn disconnect_crystal(&mut self, crystal_id: u64) {
        self.connected_crystals.retain(|&c| c != crystal_id);
    }
}

impl Default for EnderDragon {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_max_hp() {
        let d = EnderDragon::new();
        assert_eq!(d.hp, HP_MAX);
    }

    #[test]
    fn dying_at_zero() {
        let mut d = EnderDragon::new();
        d.take_damage(1000.0, true);
        assert!(d.is_dying());
    }

    #[test]
    fn body_takes_reduced_damage() {
        let mut d = EnderDragon::new();
        d.take_damage(100.0, false);
        assert!(d.hp > HP_MAX - 100.0); // only 25% applied
    }
}
