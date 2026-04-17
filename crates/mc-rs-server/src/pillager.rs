//! Pillager + Vindicator + Evoker + Illusioner — illagers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IllagerKind {
    Pillager,   // Crossbow
    Vindicator, // Iron axe
    Evoker,     // Summons vex + fangs
    Illusioner, // Bow + illusions
}

#[derive(Debug, Clone)]
pub struct Illager {
    pub kind: IllagerKind,
    pub is_patrol_leader: bool,
    pub raid_id: Option<u64>,
    pub attack_cooldown: u32,
    pub charge_ticks: u32,
    pub spell_ticks: u32,
}

/// Crossbow charge time (25 ticks).
pub const CROSSBOW_CHARGE: u32 = 25;
/// Axe swing cooldown (20 ticks).
pub const AXE_COOLDOWN: u32 = 20;
/// Evoker summon cooldown (100 ticks).
pub const EVOKER_SUMMON_COOLDOWN: u32 = 100;

impl Illager {
    pub fn new(kind: IllagerKind) -> Self {
        Self {
            kind,
            is_patrol_leader: false,
            raid_id: None,
            attack_cooldown: 0,
            charge_ticks: 0,
            spell_ticks: 0,
        }
    }

    pub fn tick(&mut self) {
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }
    }

    /// Leader gives nearby illagers bad omen on kill.
    pub fn grants_bad_omen(&self) -> bool {
        self.is_patrol_leader
    }

    /// Evoker summon fang or vex.
    pub fn can_summon(&self) -> bool {
        self.kind == IllagerKind::Evoker && self.attack_cooldown == 0
    }

    pub fn start_spell(&mut self) {
        self.spell_ticks = 40; // ~2s cast
        self.attack_cooldown = EVOKER_SUMMON_COOLDOWN;
    }

    /// Weakness to mob effects.
    pub fn is_weak_to_vex() -> bool {
        true
    }
    pub fn damage_by_axe_crit() -> f32 {
        13.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evoker_can_summon() {
        let i = Illager::new(IllagerKind::Evoker);
        assert!(i.can_summon());
    }

    #[test]
    fn pillager_cant_summon() {
        let i = Illager::new(IllagerKind::Pillager);
        assert!(!i.can_summon());
    }
}
