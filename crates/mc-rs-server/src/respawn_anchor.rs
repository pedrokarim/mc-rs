//! Respawn anchor — port PMMP `src/block/RespawnAnchor.php` (nether spawn block).

#[derive(Debug, Clone, Copy)]
pub struct RespawnAnchorState {
    /// Charges restantes (0-4). 0 = inutilisable.
    pub charges: u8,
}

impl RespawnAnchorState {
    pub const MAX_CHARGES: u8 = 4;

    pub fn new() -> Self {
        Self { charges: 0 }
    }

    pub fn is_charged(&self) -> bool {
        self.charges > 0
    }

    pub fn charge(&mut self) -> bool {
        if self.charges >= Self::MAX_CHARGES {
            return false;
        }
        self.charges += 1;
        true
    }

    /// Consume one charge when used for respawn. Retourne true si utilisable.
    pub fn consume_charge(&mut self) -> bool {
        if self.charges == 0 {
            return false;
        }
        self.charges -= 1;
        true
    }

    /// Si le joueur tente d'utiliser l'anchor hors nether → explosion !
    /// PMMP : explosion force 5.0.
    pub fn explodes_outside_nether(&self, in_nether: bool) -> bool {
        !in_nether && self.charges > 0
    }
}

impl Default for RespawnAnchorState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_4_charges() {
        let mut a = RespawnAnchorState::new();
        for _ in 0..4 {
            assert!(a.charge());
        }
        assert!(!a.charge());
    }

    #[test]
    fn explodes_outside_nether_when_charged() {
        let mut a = RespawnAnchorState::new();
        a.charge();
        assert!(a.explodes_outside_nether(false));
        assert!(!a.explodes_outside_nether(true));
    }
}
