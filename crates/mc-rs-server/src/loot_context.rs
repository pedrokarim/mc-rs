//! Loot context — params de LootContext pour drop calculation.

#[derive(Debug, Clone)]
pub struct LootContext {
    pub killed_by_player: bool,
    pub looting_level: u8,
    pub fortune_level: u8,
    pub silk_touch: bool,
    pub explosion: bool,
    pub position: [f32; 3],
    pub luck: f32,
}

impl Default for LootContext {
    fn default() -> Self {
        Self {
            killed_by_player: false,
            looting_level: 0,
            fortune_level: 0,
            silk_touch: false,
            explosion: false,
            position: [0.0, 0.0, 0.0],
            luck: 0.0,
        }
    }
}

impl LootContext {
    pub fn with_player_killer(mut self) -> Self {
        self.killed_by_player = true;
        self
    }

    pub fn with_looting(mut self, level: u8) -> Self {
        self.looting_level = level;
        self
    }

    pub fn with_fortune(mut self, level: u8) -> Self {
        self.fortune_level = level;
        self
    }

    pub fn with_silk_touch(mut self) -> Self {
        self.silk_touch = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_chain() {
        let ctx = LootContext::default()
            .with_player_killer()
            .with_looting(3)
            .with_fortune(2);
        assert!(ctx.killed_by_player);
        assert_eq!(ctx.looting_level, 3);
        assert_eq!(ctx.fortune_level, 2);
    }
}
