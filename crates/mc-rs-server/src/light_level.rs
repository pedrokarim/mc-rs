//! Light level — block + sky light propagation.

/// Max light level.
pub const MAX_LIGHT: u8 = 15;

/// Block light sources (partial list).
pub fn block_emission(block_id: u16) -> u8 {
    match block_id {
        10 | 11 => 15,  // lava / flowing lava
        51 => 15,       // fire
        89 => 15,       // glowstone
        50 => 14,       // torch
        76 => 7,        // redstone torch on
        124 => 15,      // redstone lamp on
        91 => 15,       // jack o'lantern
        119 => 1,       // end portal
        120 => 1,       // end portal frame
        169 => 15,      // sea lantern
        213 => 3,       // magma block
        200 => 15,      // end rod
        252 => 15,      // glowstone?
        151 => 15,      // daylight sensor
        327 => 15,      // beacon beam
        // cave vines / berries
        734 => 14,      // cave vines lit
        _ => 0,
    }
}

/// Opacity (how much light loses per block).
pub fn opacity(block_id: u16) -> u8 {
    match block_id {
        0 | 166 => 0,    // air, barrier
        8 | 9 => 2,      // water (2 blocks per unit light)
        18 | 161 => 1,   // leaves
        79 | 212 => 1,   // ice
        20 | 95 => 0,    // glass
        _ => MAX_LIGHT,
    }
}

pub fn is_transparent(block_id: u16) -> bool {
    opacity(block_id) < MAX_LIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glowstone_full_light() {
        assert_eq!(block_emission(89), 15);
    }

    #[test]
    fn stone_no_light() {
        assert_eq!(block_emission(1), 0);
    }

    #[test]
    fn water_partial_opacity() {
        assert!(opacity(9) > 0);
        assert!(opacity(9) < MAX_LIGHT);
    }
}
