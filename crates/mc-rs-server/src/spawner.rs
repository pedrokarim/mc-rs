//! Mob spawner block — timer, mob config.

#[derive(Debug, Clone)]
pub struct MobSpawner {
    pub entity_id: String,
    pub delay: u32,
    pub min_spawn_delay: u32,
    pub max_spawn_delay: u32,
    pub spawn_count: u32,
    pub max_nearby_entities: u32,
    pub required_player_range: u32,
    pub spawn_range: u32,
}

/// PMMP defaults.
pub const DEFAULT_MIN_DELAY: u32 = 200;
pub const DEFAULT_MAX_DELAY: u32 = 800;
pub const DEFAULT_SPAWN_COUNT: u32 = 4;
pub const DEFAULT_MAX_NEARBY: u32 = 6;
pub const DEFAULT_PLAYER_RANGE: u32 = 16;
pub const DEFAULT_SPAWN_RANGE: u32 = 4;

impl MobSpawner {
    pub fn new(entity_id: impl Into<String>) -> Self {
        Self {
            entity_id: entity_id.into(),
            delay: DEFAULT_MIN_DELAY,
            min_spawn_delay: DEFAULT_MIN_DELAY,
            max_spawn_delay: DEFAULT_MAX_DELAY,
            spawn_count: DEFAULT_SPAWN_COUNT,
            max_nearby_entities: DEFAULT_MAX_NEARBY,
            required_player_range: DEFAULT_PLAYER_RANGE,
            spawn_range: DEFAULT_SPAWN_RANGE,
        }
    }

    pub fn tick(&mut self) -> bool {
        if self.delay > 0 {
            self.delay -= 1;
            return false;
        }
        use rand::Rng;
        let mut rng = rand::thread_rng();
        self.delay = rng.gen_range(self.min_spawn_delay..=self.max_spawn_delay);
        true
    }

    /// Breaking a spawner drops XP.
    pub fn break_xp() -> (u32, u32) { (15, 43) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_down_delay() {
        let mut s = MobSpawner::new("zombie");
        let start = s.delay;
        s.tick();
        assert_eq!(s.delay, start - 1);
    }
}
