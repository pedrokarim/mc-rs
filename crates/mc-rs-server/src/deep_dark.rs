//! Deep Dark — underground biome with sculk + warden.

pub const MIN_Y: i32 = -64;
pub const MAX_Y: i32 = -8;

/// Ancient City generates here.
pub fn generates_ancient_city() -> bool {
    true
}

/// Deep Dark biome ID.
pub const BIOME_ID: u8 = 190;

/// Warden spawn conditions.
pub fn can_spawn_warden() -> bool {
    true
}

/// Sculk catalyst converts dead blocks into sculk on death.
pub fn sculk_catalyst_range() -> u32 {
    8
}

/// Sculk shrieker warning count before warden summon.
pub const SHRIEKS_NEEDED: u8 = 4;

#[cfg(test)]
mod tests {
    #[test]
    fn can_have_warden() {
        assert!(super::can_spawn_warden());
    }
}
