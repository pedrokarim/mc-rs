//! End islands — main + outer.

/// Main island: from (-128, -128) to (128, 128).
pub const MAIN_ISLAND_RADIUS: i32 = 128;
/// Void around main island (from 128 to 1024).
pub const VOID_RADIUS_MIN: i32 = 128;
pub const VOID_RADIUS_MAX: i32 = 1024;

/// Outer islands generate at radius > 1024.
pub fn is_outer_island(x: i32, z: i32) -> bool {
    let dist_sq = (x as i64).pow(2) + (z as i64).pow(2);
    dist_sq > (VOID_RADIUS_MAX as i64).pow(2)
}

/// Outer island blocks.
pub fn outer_blocks() -> &'static [&'static str] {
    &[
        "minecraft:end_stone",
        "minecraft:purpur_block",
        "minecraft:end_rod",
        "minecraft:chorus_flower",
        "minecraft:chorus_plant",
    ]
}

/// Shulker box drop in End cities.
pub fn city_contains_shulkers() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_island_not_outer() {
        assert!(!is_outer_island(0, 0));
    }

    #[test]
    fn far_position_is_outer() {
        assert!(is_outer_island(2000, 0));
    }
}
