//! Night/day logic — sleep, moon phase, lunar impact.

/// Sun sets at tick 13000 vanilla.
pub const SUN_SET_TICK: u64 = 13000;
/// Sun rises at tick 23000.
pub const SUN_RISE_TICK: u64 = 23000;
/// One full day = 24000 ticks.
pub const DAY_LENGTH: u64 = 24000;

pub fn is_daytime(time_of_day: u64) -> bool {
    let t = time_of_day % DAY_LENGTH;
    t >= SUN_RISE_TICK || t < SUN_SET_TICK
}

pub fn is_night(time_of_day: u64) -> bool {
    !is_daytime(time_of_day)
}

/// Night length for mob spawning logic (13000-23000 = 10000 ticks = ~8.3 min).
pub const NIGHT_DURATION: u64 = 10000;

/// Sleep-skip vote at least 50% of players (sleep_percentage gamerule).
pub const DEFAULT_SLEEP_PERCENTAGE: u8 = 50;

/// Moon phase indexing (0-7, repeats every 8 days).
pub fn moon_phase(day_count: u64) -> u8 {
    (day_count % 8) as u8
}

/// Moon full (phase 0) increases slime spawn chance.
pub fn is_full_moon(day_count: u64) -> bool {
    moon_phase(day_count) == 0
}

/// Slime spawn chance based on moon phase (0-100%).
pub fn slime_phase_multiplier(day_count: u64) -> f32 {
    let phase = moon_phase(day_count);
    match phase {
        0 => 1.0,      // full
        1 | 7 => 0.75, // waxing/waning gibbous
        2 | 6 => 0.5,  // first/last quarter
        3 | 5 => 0.25, // waxing/waning crescent
        _ => 0.0,      // new moon (4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midnight_is_night() {
        assert!(is_night(18000));
    }

    #[test]
    fn noon_is_day() {
        assert!(is_daytime(6000));
    }

    #[test]
    fn full_moon_every_8_days() {
        assert!(is_full_moon(0));
        assert!(is_full_moon(8));
        assert!(!is_full_moon(1));
    }
}
