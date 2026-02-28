//! Experience (XP) system: formulas, level calculation, mob/ore XP rewards.

use rand::Rng;

// ---------------------------------------------------------------------------
// XP formulas
// ---------------------------------------------------------------------------

/// XP needed to advance from `level` to `level + 1`.
pub fn xp_for_next_level(level: i32) -> i32 {
    if level < 0 {
        0
    } else if level < 16 {
        2 * level + 7
    } else if level < 31 {
        5 * level - 38
    } else {
        9 * level - 158
    }
}

/// Total XP needed from 0 to reach the START of `level`.
pub fn total_xp_for_level(level: i32) -> i32 {
    if level <= 0 {
        return 0;
    }
    let mut total = 0;
    for l in 0..level {
        total += xp_for_next_level(l);
    }
    total
}

/// Compute the level from a total XP amount (inverse of `total_xp_for_level`).
pub fn level_from_total_xp(total_xp: i32) -> i32 {
    if total_xp <= 0 {
        return 0;
    }
    let mut level = 0;
    let mut remaining = total_xp;
    loop {
        let needed = xp_for_next_level(level);
        if remaining < needed {
            break;
        }
        remaining -= needed;
        level += 1;
    }
    level
}

/// XP progress within the current level (0.0 .. 1.0).
pub fn xp_progress(level: i32, total_xp: i32) -> f32 {
    let level_start = total_xp_for_level(level);
    let needed = xp_for_next_level(level);
    if needed <= 0 {
        return 0.0;
    }
    let within = total_xp - level_start;
    (within as f32 / needed as f32).clamp(0.0, 1.0)
}

/// Add XP and return `(new_level, new_total)`.
pub fn add_xp(_current_level: i32, current_total: i32, amount: i32) -> (i32, i32) {
    let new_total = (current_total + amount).max(0);
    let new_level = level_from_total_xp(new_total);
    (new_level, new_total)
}

/// XP lost on death: `level * 7`, capped at the player's total.
pub fn xp_lost_on_death(level: i32, total_xp: i32) -> i32 {
    let loss = level * 7;
    loss.min(total_xp).max(0)
}

/// Compute new `(level, total)` after dying.
pub fn after_death(level: i32, total_xp: i32) -> (i32, i32) {
    let loss = xp_lost_on_death(level, total_xp);
    let new_total = (total_xp - loss).max(0);
    let new_level = level_from_total_xp(new_total);
    (new_level, new_total)
}

// ---------------------------------------------------------------------------
// Mob XP rewards
// ---------------------------------------------------------------------------

/// XP dropped by a mob on death.
pub fn mob_xp(mob_type: &str) -> i32 {
    match mob_type {
        "minecraft:zombie" | "minecraft:skeleton" => 5,
        "minecraft:cow" | "minecraft:pig" | "minecraft:chicken" => 2,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Ore XP rewards
// ---------------------------------------------------------------------------

/// XP range `(min, max)` from mining an ore. Returns `None` if the block is not an ore.
pub fn ore_xp_range(block_name: &str) -> Option<(i32, i32)> {
    match block_name {
        "minecraft:coal_ore" | "minecraft:deepslate_coal_ore" => Some((0, 2)),
        "minecraft:diamond_ore" | "minecraft:deepslate_diamond_ore" => Some((3, 7)),
        "minecraft:emerald_ore" | "minecraft:deepslate_emerald_ore" => Some((3, 7)),
        "minecraft:lapis_ore" | "minecraft:deepslate_lapis_ore" => Some((2, 5)),
        "minecraft:redstone_ore" | "minecraft:deepslate_redstone_ore" => Some((1, 5)),
        "minecraft:nether_quartz_ore" => Some((2, 5)),
        _ => None,
    }
}

/// Random XP from mining a block. Returns 0 if the block is not an ore.
pub fn ore_xp_random(block_name: &str) -> i32 {
    match ore_xp_range(block_name) {
        Some((min, max)) => {
            if min >= max {
                return min;
            }
            let mut rng = rand::thread_rng();
            rng.gen_range(min..=max)
        }
        None => 0,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xp_for_next_level_boundaries() {
        assert_eq!(xp_for_next_level(0), 7);
        assert_eq!(xp_for_next_level(1), 9);
        assert_eq!(xp_for_next_level(15), 37);
        assert_eq!(xp_for_next_level(16), 42);
        assert_eq!(xp_for_next_level(30), 112);
        assert_eq!(xp_for_next_level(31), 121);
        assert_eq!(xp_for_next_level(50), 292);
    }

    #[test]
    fn total_xp_for_level_coherence() {
        assert_eq!(total_xp_for_level(0), 0);
        assert_eq!(total_xp_for_level(1), 7);
        // Level 2 = 7 + 9 = 16
        assert_eq!(total_xp_for_level(2), 16);
        // Just check it's increasing
        let mut prev = 0;
        for l in 1..=50 {
            let total = total_xp_for_level(l);
            assert!(total > prev, "total_xp should increase: level {l}");
            prev = total;
        }
    }

    #[test]
    fn level_from_total_xp_roundtrip() {
        for l in 0..=50 {
            let total = total_xp_for_level(l);
            assert_eq!(
                level_from_total_xp(total),
                l,
                "roundtrip failed for level {l}"
            );
        }
        // Mid-level should still return the same level
        let total_10 = total_xp_for_level(10);
        let mid = total_10 + xp_for_next_level(10) / 2;
        assert_eq!(level_from_total_xp(mid), 10);
    }

    #[test]
    fn xp_progress_values() {
        // At level start, progress = 0
        let total_5 = total_xp_for_level(5);
        assert!((xp_progress(5, total_5) - 0.0).abs() < 0.001);

        // At midpoint
        let needed = xp_for_next_level(5);
        let mid = total_5 + needed / 2;
        let p = xp_progress(5, mid);
        assert!(p > 0.4 && p < 0.6, "progress should be ~0.5, got {p}");

        // Just before next level
        let almost = total_5 + needed - 1;
        let p2 = xp_progress(5, almost);
        assert!(p2 > 0.9, "progress should be ~1.0, got {p2}");
    }

    #[test]
    fn add_xp_simple() {
        let (level, total) = add_xp(0, 0, 5);
        assert_eq!(level, 0);
        assert_eq!(total, 5);
    }

    #[test]
    fn add_xp_level_up() {
        // Level 0 needs 7 XP to reach level 1
        let (level, total) = add_xp(0, 0, 7);
        assert_eq!(level, 1);
        assert_eq!(total, 7);

        // Adding more to go to level 2 (needs 9 more = 16 total)
        let (level, total) = add_xp(1, 7, 9);
        assert_eq!(level, 2);
        assert_eq!(total, 16);
    }

    #[test]
    fn add_xp_multi_level() {
        // Give enough for level 5
        let total_5 = total_xp_for_level(5);
        let (level, total) = add_xp(0, 0, total_5);
        assert_eq!(level, 5);
        assert_eq!(total, total_5);
    }

    #[test]
    fn xp_lost_on_death_values() {
        // Level 10: lose 70 XP
        let total_10 = total_xp_for_level(10);
        assert_eq!(xp_lost_on_death(10, total_10), 70);

        // Level 0: lose nothing
        assert_eq!(xp_lost_on_death(0, 0), 0);

        // Cap at total
        assert_eq!(xp_lost_on_death(10, 30), 30);
    }

    #[test]
    fn after_death_values() {
        let total_10 = total_xp_for_level(10);
        let (new_level, new_total) = after_death(10, total_10);
        assert!(new_level < 10);
        assert_eq!(new_total, total_10 - 70);
        assert_eq!(new_level, level_from_total_xp(new_total));
    }

    #[test]
    fn mob_xp_values() {
        assert_eq!(mob_xp("minecraft:zombie"), 5);
        assert_eq!(mob_xp("minecraft:skeleton"), 5);
        assert_eq!(mob_xp("minecraft:cow"), 2);
        assert_eq!(mob_xp("minecraft:pig"), 2);
        assert_eq!(mob_xp("minecraft:chicken"), 2);
        assert_eq!(mob_xp("minecraft:unknown"), 0);
    }

    #[test]
    fn ore_xp_range_values() {
        assert_eq!(ore_xp_range("minecraft:coal_ore"), Some((0, 2)));
        assert_eq!(ore_xp_range("minecraft:diamond_ore"), Some((3, 7)));
        assert_eq!(ore_xp_range("minecraft:emerald_ore"), Some((3, 7)));
        assert_eq!(ore_xp_range("minecraft:lapis_ore"), Some((2, 5)));
        assert_eq!(ore_xp_range("minecraft:redstone_ore"), Some((1, 5)));
        assert_eq!(
            ore_xp_range("minecraft:deepslate_diamond_ore"),
            Some((3, 7))
        );
        assert_eq!(ore_xp_range("minecraft:nether_quartz_ore"), Some((2, 5)));
        assert_eq!(ore_xp_range("minecraft:stone"), None);
        assert_eq!(ore_xp_range("minecraft:iron_ore"), None);
    }

    #[test]
    fn ore_xp_random_in_range() {
        for _ in 0..20 {
            let xp = ore_xp_random("minecraft:diamond_ore");
            assert!((3..=7).contains(&xp), "diamond xp {xp} out of range");
        }
        assert_eq!(ore_xp_random("minecraft:stone"), 0);
    }
}
