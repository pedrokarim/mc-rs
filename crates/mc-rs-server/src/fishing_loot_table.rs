//! Fishing loot tables (fish, treasure, junk).

use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishingOutcome {
    Fish,
    Treasure,
    Junk,
}

/// Base weights. Modified by Luck of the Sea enchant.
pub fn outcome_weight(outcome: FishingOutcome, luck_of_sea: u8) -> f32 {
    match outcome {
        FishingOutcome::Fish => 85.0 - 2.0 * luck_of_sea as f32,
        FishingOutcome::Treasure => 5.0 + 2.0 * luck_of_sea as f32,
        FishingOutcome::Junk => 10.0 - 2.5 * luck_of_sea as f32,
    }.max(0.0)
}

pub fn fish_loot() -> &'static [(&'static str, u32)] {
    &[
        ("minecraft:raw_cod", 60),
        ("minecraft:raw_salmon", 25),
        ("minecraft:tropical_fish", 2),
        ("minecraft:pufferfish", 13),
    ]
}

pub fn treasure_loot() -> &'static [(&'static str, u32)] {
    &[
        ("minecraft:bow", 17),
        ("minecraft:enchanted_book", 17),
        ("minecraft:fishing_rod", 17),
        ("minecraft:lily_pad", 17),
        ("minecraft:name_tag", 17),
        ("minecraft:nautilus_shell", 17),
        ("minecraft:saddle", 17),
    ]
}

pub fn junk_loot() -> &'static [(&'static str, u32)] {
    &[
        ("minecraft:bowl", 10),
        ("minecraft:leather_boots", 10),
        ("minecraft:leather", 10),
        ("minecraft:bone", 10),
        ("minecraft:stick", 5),
        ("minecraft:string", 5),
        ("minecraft:ink_sac", 1),
        ("minecraft:rotten_flesh", 10),
        ("minecraft:glass_bottle", 10),
        ("minecraft:water_bottle", 10),
        ("minecraft:fishing_rod", 2),
        ("minecraft:tripwire_hook", 10),
    ]
}

/// Roll a fishing outcome.
pub fn roll_outcome(luck: u8) -> FishingOutcome {
    let mut rng = rand::thread_rng();
    let fish = outcome_weight(FishingOutcome::Fish, luck);
    let treasure = outcome_weight(FishingOutcome::Treasure, luck);
    let junk = outcome_weight(FishingOutcome::Junk, luck);
    let total = fish + treasure + junk;
    let roll = rng.gen::<f32>() * total;
    if roll < fish {
        FishingOutcome::Fish
    } else if roll < fish + treasure {
        FishingOutcome::Treasure
    } else {
        FishingOutcome::Junk
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luck_increases_treasure() {
        assert!(outcome_weight(FishingOutcome::Treasure, 3) > outcome_weight(FishingOutcome::Treasure, 0));
    }

    #[test]
    fn junk_table_non_empty() {
        assert!(!junk_loot().is_empty());
    }
}
