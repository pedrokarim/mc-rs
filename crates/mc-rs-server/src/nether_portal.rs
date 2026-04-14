//! Nether portal — port PMMP `src/block/utils/NetherPortalHelper.php`.

use crate::dimensions::DimensionId;

pub const PORTAL_COOLDOWN_TICKS: u32 = 80; // 4 sec entre teleports
pub const PORTAL_TRIGGER_TICKS: u32 = 80; // 4 sec in portal avant TP

#[derive(Debug, Clone)]
pub struct PortalState {
    pub in_portal_ticks: u32,
    pub cooldown_ticks: u32,
    pub last_dimension: DimensionId,
}

impl PortalState {
    pub fn new(dim: DimensionId) -> Self {
        Self {
            in_portal_ticks: 0,
            cooldown_ticks: 0,
            last_dimension: dim,
        }
    }

    /// Tick : entity in portal block.
    /// Retourne Some(target_dim) si doit TP ce tick.
    pub fn tick_in_portal(&mut self) -> Option<DimensionId> {
        if self.cooldown_ticks > 0 {
            self.cooldown_ticks -= 1;
            return None;
        }
        self.in_portal_ticks += 1;
        if self.in_portal_ticks >= PORTAL_TRIGGER_TICKS {
            self.in_portal_ticks = 0;
            self.cooldown_ticks = PORTAL_COOLDOWN_TICKS;
            let target = match self.last_dimension {
                DimensionId::Overworld => DimensionId::Nether,
                DimensionId::Nether => DimensionId::Overworld,
                DimensionId::End => DimensionId::Overworld,
            };
            self.last_dimension = target;
            return Some(target);
        }
        None
    }

    /// Called when entity leaves portal block without teleport.
    pub fn leave_portal(&mut self) {
        self.in_portal_ticks = 0;
    }
}

/// Compute nether-to-overworld or vice-versa position using vanilla 1:8 ratio.
pub fn translate_position(from: DimensionId, to: DimensionId, pos: [f32; 3]) -> [f32; 3] {
    match (from, to) {
        (DimensionId::Overworld, DimensionId::Nether) => [pos[0] / 8.0, pos[1], pos[2] / 8.0],
        (DimensionId::Nether, DimensionId::Overworld) => [pos[0] * 8.0, pos[1], pos[2] * 8.0],
        _ => pos,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_teleport_after_80_ticks() {
        let mut p = PortalState::new(DimensionId::Overworld);
        for _ in 0..79 {
            assert!(p.tick_in_portal().is_none());
        }
        let t = p.tick_in_portal();
        assert_eq!(t, Some(DimensionId::Nether));
    }

    #[test]
    fn leave_resets_timer() {
        let mut p = PortalState::new(DimensionId::Overworld);
        for _ in 0..50 {
            p.tick_in_portal();
        }
        p.leave_portal();
        assert_eq!(p.in_portal_ticks, 0);
    }

    #[test]
    fn position_ratio_1_to_8() {
        let p = translate_position(
            DimensionId::Overworld,
            DimensionId::Nether,
            [800.0, 64.0, 0.0],
        );
        assert_eq!(p[0], 100.0);
    }
}
