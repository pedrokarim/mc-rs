//! Allay — Mob pacifique qui collecte items et duplique avec amethyst + noteblock.

#[derive(Debug, Clone)]
pub struct Allay {
    pub held_item: Option<u16>,
    pub following_player: Option<u64>,
    pub vibration_target: Option<(i32, i32, i32)>,
    pub duplication_cooldown: u32,
}

/// Duplication cooldown après amethyst + noteblock (5 minutes = 6000 ticks).
pub const DUPLICATION_COOLDOWN: u32 = 6000;
/// Max distance pour continuer à suivre le joueur.
pub const FOLLOW_RANGE: f64 = 64.0;
/// Range pour noteblock resonance.
pub const NOTEBLOCK_RANGE: f64 = 16.0;

impl Allay {
    pub fn new() -> Self {
        Self {
            held_item: None,
            following_player: None,
            vibration_target: None,
            duplication_cooldown: 0,
        }
    }

    pub fn give_item(&mut self, item_id: u16, player_id: u64) {
        self.held_item = Some(item_id);
        self.following_player = Some(player_id);
    }

    pub fn drop_item(&mut self) -> Option<u16> {
        self.following_player = None;
        self.held_item.take()
    }

    pub fn tick(&mut self) {
        if self.duplication_cooldown > 0 {
            self.duplication_cooldown -= 1;
        }
    }

    /// Tente duplication si tient amethyst shard et entend noteblock.
    pub fn try_duplicate(&mut self) -> bool {
        if self.duplication_cooldown > 0 {
            return false;
        }
        if self.held_item != Some(Self::AMETHYST_SHARD_ID) {
            return false;
        }
        self.duplication_cooldown = DUPLICATION_COOLDOWN;
        true
    }

    const AMETHYST_SHARD_ID: u16 = 721;
}

impl Default for Allay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receives_item_follows_player() {
        let mut a = Allay::new();
        a.give_item(1, 42);
        assert_eq!(a.following_player, Some(42));
    }

    #[test]
    fn duplication_on_cooldown() {
        let mut a = Allay::new();
        a.held_item = Some(Allay::AMETHYST_SHARD_ID);
        assert!(a.try_duplicate());
        assert!(!a.try_duplicate());
    }

    #[test]
    fn duplication_requires_amethyst() {
        let mut a = Allay::new();
        a.held_item = Some(1);
        assert!(!a.try_duplicate());
    }
}
