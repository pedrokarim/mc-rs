//! Mob cap limits.

/// Per player-radius category caps.
pub const MOB_CAP_HOSTILE: u32 = 70;
pub const MOB_CAP_PASSIVE: u32 = 10;
pub const MOB_CAP_AMBIENT: u32 = 15;
pub const MOB_CAP_WATER_CREATURE: u32 = 5;
pub const MOB_CAP_WATER_AMBIENT: u32 = 20;
pub const MOB_CAP_UNDERGROUND_WATER: u32 = 5;
pub const MOB_CAP_MISC: u32 = 0;

pub fn hostile_cap_per_player() -> u32 { MOB_CAP_HOSTILE }
pub fn passive_cap_per_player() -> u32 { MOB_CAP_PASSIVE }

/// Radius within which to count per player (128 blocks).
pub const COUNTING_RADIUS: u32 = 128;

/// Delay between spawn attempts (400 ticks).
pub const SPAWN_ATTEMPT_INTERVAL: u32 = 400;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostile_more_than_passive() {
        assert!(MOB_CAP_HOSTILE > MOB_CAP_PASSIVE);
    }
}
