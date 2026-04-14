//! Breeze rod — drop from Breeze (1.21).

pub const ITEM_ID: &str = "minecraft:breeze_rod";

/// Used in smithing for new enchants (Wind Burst).
pub fn used_for_wind_burst_template() -> bool { true }

/// Used to make mace.
pub fn used_for_mace() -> bool { true }

#[cfg(test)]
mod tests {
    #[test]
    fn item_id() {
        assert!(!super::ITEM_ID.is_empty());
    }
}
