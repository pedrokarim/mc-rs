//! Jungle temple structure.

pub const SIZE: u32 = 12;

/// Contains tripwire trap + lever puzzle + double chests.
pub fn traps() -> &'static [&'static str] {
    &["tripwire_hook_trap", "lever_combination_puzzle"]
}

/// Loot from jungle temple chest.
pub fn loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:diamond", 1, 3, 1),
        ("minecraft:iron_ingot", 1, 5, 10),
        ("minecraft:gold_ingot", 2, 7, 10),
        ("minecraft:emerald", 1, 3, 1),
        ("minecraft:bone", 4, 6, 25),
        ("minecraft:rotten_flesh", 3, 7, 25),
        ("minecraft:saddle", 1, 1, 3),
        ("minecraft:enchanted_book", 1, 1, 1),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traps_exist() {
        assert!(!traps().is_empty());
    }
}
