//! Fletching table — 1.22+ feature (currently inactive in vanilla beyond workstation).

/// Fletching table is a job site for fletcher villager.
pub const VILLAGER_JOB: &str = "fletcher";

/// Fletcher trades (level 1): emeralds ↔ arrows/bows.
pub fn fletcher_level_1_trades() -> &'static [(&'static str, u32, &'static str, u32)] {
    &[
        ("minecraft:stick", 32, "minecraft:emerald", 1),
        ("minecraft:emerald", 1, "minecraft:arrow", 16),
        ("minecraft:gravel", 10, "minecraft:flint", 10),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_1_not_empty() {
        assert!(!fletcher_level_1_trades().is_empty());
    }
}
