//! Lightning bolt spawning and effects.

#[derive(Debug, Clone)]
pub struct LightningBolt {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub lifetime_ticks: u32,
    pub target_entity: Option<u64>,
    pub visual_only: bool, // triggered by trident channeling (no blocks catching fire)
}

/// Lightning animation duration.
pub const LIGHTNING_DURATION: u32 = 4;
/// Damage when hit directly (5 in survival).
pub const DIRECT_DAMAGE: f32 = 5.0;
/// Fire duration for struck entities (8s).
pub const FIRE_DURATION: u32 = 160;
/// Lightning range for chaining.
pub const CHAIN_RANGE: f64 = 16.0;

impl LightningBolt {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x, y, z,
            lifetime_ticks: LIGHTNING_DURATION,
            target_entity: None,
            visual_only: false,
        }
    }

    pub fn tick(&mut self) -> bool {
        self.lifetime_ticks = self.lifetime_ticks.saturating_sub(1);
        self.lifetime_ticks == 0
    }

    /// Chance to set nearby blocks on fire (if not visual).
    pub fn sets_blocks_on_fire(&self) -> bool {
        !self.visual_only
    }

    /// Lightning strikes can charge creeper, turn pig into pigman, villager into witch.
    pub fn lightning_conversions() -> &'static [(&'static str, &'static str)] {
        &[
            ("minecraft:creeper", "minecraft:charged_creeper"),
            ("minecraft:pig", "minecraft:zombie_piglin"),
            ("minecraft:villager", "minecraft:witch"),
            ("minecraft:turtle", "minecraft:baby_turtle"),
            ("minecraft:red_mooshroom", "minecraft:brown_mooshroom"),
            ("minecraft:brown_mooshroom", "minecraft:red_mooshroom"),
            ("minecraft:skeleton_horse", "minecraft:skeleton_horse"), // spawns trap horde
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lightning_expires() {
        let mut l = LightningBolt::new(0.0, 0.0, 0.0);
        for _ in 0..LIGHTNING_DURATION {
            l.tick();
        }
        assert_eq!(l.lifetime_ticks, 0);
    }
}
