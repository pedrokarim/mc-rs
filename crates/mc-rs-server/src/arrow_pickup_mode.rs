//! Arrow pickup rules.

use crate::arrow::PickupMode;

/// Determine pickup mode based on shooter gamemode.
pub fn pickup_for_gamemode(gamemode: u8, infinity: bool) -> PickupMode {
    if infinity {
        return PickupMode::Disallowed;
    }
    match gamemode {
        1 => PickupMode::CreativeOnly,
        2 => PickupMode::Disallowed,
        _ => PickupMode::AllowedByAny,
    }
}

/// Duck when arrow has been in ground for N ticks.
pub const GROUND_TICK_THRESHOLD: u32 = 1200;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creative_arrow_creative_only() {
        matches!(pickup_for_gamemode(1, false), PickupMode::CreativeOnly);
    }
}
