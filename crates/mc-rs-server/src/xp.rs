//! XP level system — port PMMP `src/entity/Human.php`.

/// XP required to go from level N to N+1.
pub fn xp_to_next_level(current: u32) -> u32 {
    if current < 16 {
        2 * current + 7
    } else if current < 32 {
        5 * current - 38
    } else {
        9 * current - 158
    }
}

/// Total XP from 0 to given level.
pub fn total_xp_to_level(level: u32) -> u32 {
    (0..level).map(xp_to_next_level).sum()
}

/// Convert total XP to current level + progress.
pub fn xp_to_level_and_progress(total: u32) -> (u32, f32) {
    let mut level = 0;
    let mut remaining = total;
    loop {
        let needed = xp_to_next_level(level);
        if remaining < needed {
            return (level, remaining as f32 / needed as f32);
        }
        remaining -= needed;
        level += 1;
    }
}

/// Min XP level to enchant with 30 (needs 30 levels).
pub const MAX_ENCHANT_LEVELS: u32 = 30;
/// Anvil max uses before prior work cost too high.
pub const MAX_ANVIL_PRIOR_WORK: u32 = 39;

/// XP orbs dropped by mobs.
pub const ZOMBIE_XP: u32 = 5;
pub const SKELETON_XP: u32 = 5;
pub const CREEPER_XP: u32 = 5;
pub const WITHER_SKELETON_XP: u32 = 5;
pub const BLAZE_XP: u32 = 10;
pub const ENDERMAN_XP: u32 = 5;
pub const WITHER_XP: u32 = 50;
pub const DRAGON_XP_FIRST_KILL: u32 = 12000;
pub const DRAGON_XP_RESPAWN: u32 = 500;
pub const VILLAGER_TRADE_XP_MIN: u32 = 3;
pub const VILLAGER_TRADE_XP_MAX: u32 = 6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_0_to_1_is_7() {
        assert_eq!(xp_to_next_level(0), 7);
    }

    #[test]
    fn level_16_transition() {
        assert_eq!(xp_to_next_level(16), 5 * 16 - 38);
    }

    #[test]
    fn progress_accurate() {
        let total = total_xp_to_level(5) + 3;
        let (lvl, prog) = xp_to_level_and_progress(total);
        assert_eq!(lvl, 5);
        assert!(prog > 0.0 && prog < 1.0);
    }
}
