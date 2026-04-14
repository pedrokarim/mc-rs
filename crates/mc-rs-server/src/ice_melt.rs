//! Ice melt — ice, packed ice, blue ice, frosted ice.

/// Ice melts when light level >= N (11 vanilla).
pub const MELT_LIGHT_THRESHOLD: u8 = 11;
/// Packed ice/blue ice don't melt (frozen deep).
pub fn can_melt(block_id: u16) -> bool {
    matches!(block_id,
        79  // ice
        | 212 // frosted ice
    )
}

/// Ice becomes water when melted.
pub fn melt_result(block_id: u16) -> u16 {
    match block_id {
        79 | 212 => 9, // flowing water
        _ => block_id,
    }
}

/// Snow layer melts to nothing (air).
pub fn snow_layer_melts() -> u16 { 0 }

/// Frost walker creates frosted ice on water (stage 0).
pub const FROST_WALKER_RANGE: u32 = 2;
/// Frost walker range per level.
pub fn frost_walker_range(level: u8) -> u32 {
    FROST_WALKER_RANGE + level as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ice_melts_to_water() {
        assert_eq!(melt_result(79), 9);
    }

    #[test]
    fn packed_ice_persistent() {
        assert!(!can_melt(174));
    }
}
