//! Stronghold structure — contains end portal.

/// Strongholds are arranged in rings around world center.
pub const RING_COUNT: u32 = 8;
/// Strongholds per ring.
pub const STRONGHOLDS_PER_RING: &[u32] = &[3, 6, 10, 15, 21, 28, 36, 128];
/// Ring inner radii (blocks).
pub const RING_INNER_RADII: &[u32] = &[1408, 2688, 4480, 6784, 9600, 12928, 16768, 21120];

/// Stronghold chest loot.
pub fn corridor_chest_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:bread", 1, 3, 20),
        ("minecraft:iron_pickaxe", 1, 1, 5),
        ("minecraft:apple", 1, 3, 15),
        ("minecraft:iron_ingot", 1, 5, 10),
        ("minecraft:gold_ingot", 1, 3, 5),
        ("minecraft:redstone", 4, 9, 5),
        ("minecraft:iron_boots", 1, 1, 1),
        ("minecraft:saddle", 1, 1, 1),
        ("minecraft:book", 1, 1, 10),
        ("minecraft:enchanted_book", 1, 1, 1),
    ]
}

pub fn library_chest_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:book", 1, 3, 20),
        ("minecraft:paper", 2, 7, 20),
        ("minecraft:map", 1, 1, 1),
        ("minecraft:compass", 1, 1, 1),
        ("minecraft:enchanted_book", 1, 1, 10),
    ]
}

/// End portal frame count (12 total, some with eye).
pub const END_PORTAL_FRAME_COUNT: u32 = 12;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_sized() {
        assert_eq!(STRONGHOLDS_PER_RING.len(), RING_COUNT as usize);
    }
}
