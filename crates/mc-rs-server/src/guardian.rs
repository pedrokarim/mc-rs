//! Guardian + ElderGuardian — ocean monument boss.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardianKind {
    Normal,
    Elder,
}

#[derive(Debug, Clone)]
pub struct Guardian {
    pub kind: GuardianKind,
    pub laser_charge: u32,
    pub spike_active: bool,
    pub target_entity: Option<u64>,
}

/// Laser charge time (60-80 ticks).
pub const LASER_CHARGE: u32 = 80;
/// Laser range.
pub const LASER_RANGE: f64 = 15.0;
/// Normal guardian HP (30), Elder (80).
pub fn hp(kind: GuardianKind) -> f32 {
    match kind {
        GuardianKind::Normal => 30.0,
        GuardianKind::Elder => 80.0,
    }
}

impl Guardian {
    pub fn new(kind: GuardianKind) -> Self {
        Self {
            kind,
            laser_charge: 0,
            spike_active: false,
            target_entity: None,
        }
    }

    pub fn start_laser(&mut self, target: u64) {
        self.laser_charge = 0;
        self.target_entity = Some(target);
    }

    pub fn tick(&mut self) {
        if self.target_entity.is_some() {
            self.laser_charge += 1;
        }
    }

    pub fn laser_ready(&self) -> bool {
        self.laser_charge >= LASER_CHARGE
    }

    /// Laser damage scales with charge time.
    pub fn laser_damage(&self) -> f32 {
        match self.kind {
            GuardianKind::Normal => 6.0,
            GuardianKind::Elder => 8.0,
        }
    }

    /// Touching guardian deals spike damage.
    pub fn spike_damage() -> f32 {
        2.0
    }

    /// Elder guardian gives Mining Fatigue to nearby players.
    pub fn elder_range() -> f64 {
        50.0
    }
    pub fn elder_mining_fatigue_duration() -> u32 {
        6000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_hp_less_than_elder() {
        assert!(hp(GuardianKind::Normal) < hp(GuardianKind::Elder));
    }

    #[test]
    fn laser_charges_over_time() {
        let mut g = Guardian::new(GuardianKind::Normal);
        g.start_laser(1);
        for _ in 0..LASER_CHARGE {
            g.tick();
        }
        assert!(g.laser_ready());
    }
}
