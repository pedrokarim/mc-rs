//! World time utilities.

/// Game tick (1/20 second).
pub const TICKS_PER_SECOND: u64 = 20;
pub const TICKS_PER_MINUTE: u64 = TICKS_PER_SECOND * 60;
pub const TICKS_PER_HOUR: u64 = TICKS_PER_MINUTE * 60;
pub const TICKS_PER_DAY: u64 = 24000;

/// Convert ticks to human readable time string (mm:ss).
pub fn ticks_to_string(ticks: u64) -> String {
    let seconds = ticks / TICKS_PER_SECOND;
    let m = seconds / 60;
    let s = seconds % 60;
    format!("{:02}:{:02}", m, s)
}

/// Absolute game time (total ticks world has existed).
pub fn day_count(game_time: u64) -> u64 {
    game_time / TICKS_PER_DAY
}

/// Time of day (0-24000).
pub fn day_time(game_time: u64) -> u64 {
    game_time % TICKS_PER_DAY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_to_sec_conversion() {
        assert_eq!(ticks_to_string(TICKS_PER_SECOND * 3), "00:03");
    }

    #[test]
    fn day_count_accurate() {
        assert_eq!(day_count(TICKS_PER_DAY * 2 + 100), 2);
    }
}
