//! Suspicious sand/gravel — brushable for loot (archaeology).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspiciousKind {
    Sand,
    Gravel,
}

/// Brush ticks required to fully dig out.
pub const BRUSH_TICKS: u32 = 20;

/// Loot tables.
pub fn desert_well_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:brick", 1, 1, 60),
        ("minecraft:gunpowder", 1, 1, 60),
        ("minecraft:emerald", 1, 1, 30),
        ("minecraft:suspicious_stew", 1, 1, 10),
        ("minecraft:arms_up_pottery_sherd", 1, 1, 2),
        ("minecraft:brewer_pottery_sherd", 1, 1, 2),
    ]
}

pub fn trail_ruin_loot_common() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:wheat", 1, 1, 15),
        ("minecraft:carrot", 1, 1, 5),
        ("minecraft:brick", 1, 1, 20),
        ("minecraft:emerald", 1, 1, 5),
        ("minecraft:wheat_seeds", 1, 1, 10),
        ("minecraft:coal", 1, 1, 15),
        ("minecraft:iron_nugget", 1, 1, 10),
        ("minecraft:gold_nugget", 1, 1, 5),
    ]
}

pub fn trail_ruin_loot_rare() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:raiser_armor_trim_smithing_template", 1, 1, 1),
        ("minecraft:shaper_armor_trim_smithing_template", 1, 1, 1),
        ("minecraft:wayfinder_armor_trim_smithing_template", 1, 1, 1),
        ("minecraft:host_armor_trim_smithing_template", 1, 1, 1),
        ("minecraft:music_disc_relic", 1, 1, 1),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desert_well_has_sherds() {
        assert!(desert_well_loot().iter().any(|(i, _, _, _)| i.contains("sherd")));
    }
}
