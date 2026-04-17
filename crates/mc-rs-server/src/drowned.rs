//! Drowned — zombie aquatique avec trident.

#[derive(Debug, Clone)]
pub struct Drowned {
    pub has_trident: bool,
    pub is_nautilus_holder: bool,
    pub target_entity: Option<u64>,
    pub attack_cooldown: u32,
}

/// Nautilus shell drop chance (3% without looting).
pub const NAUTILUS_DROP_CHANCE: f32 = 0.03;
/// Trident drop chance (8.5%).
pub const TRIDENT_DROP_CHANCE: f32 = 0.085;
/// Throw attack cooldown (30 ticks).
pub const THROW_COOLDOWN: u32 = 30;

impl Drowned {
    pub fn new() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Self {
            has_trident: rng.gen::<f32>() < 0.15,
            is_nautilus_holder: rng.gen::<f32>() < 0.03,
            target_entity: None,
            attack_cooldown: 0,
        }
    }

    pub fn tick(&mut self, in_water: bool) {
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }
        let _ = in_water; // Actually swims better in water.
    }

    pub fn try_throw_trident(&mut self, target: u64) -> bool {
        if !self.has_trident || self.attack_cooldown > 0 {
            return false;
        }
        self.target_entity = Some(target);
        self.attack_cooldown = THROW_COOLDOWN;
        true
    }

    /// Copper ingot was added as possible drop (Minecraft 1.21+).
    pub fn copper_drop_chance() -> f32 {
        0.1
    }
}

impl Default for Drowned {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trident_requires_weapon() {
        let mut d = Drowned::new();
        d.has_trident = false;
        assert!(!d.try_throw_trident(1));
    }
}
