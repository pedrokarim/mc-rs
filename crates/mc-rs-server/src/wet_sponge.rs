//! Sponge / wet sponge — water absorption.

/// Sponge absorption range (6 blocks in all directions).
pub const ABSORPTION_RANGE: u32 = 6;
/// Max blocks absorbed.
pub const MAX_ABSORBED: u32 = 64;

/// Smelt wet sponge in furnace to get dry sponge.
pub fn smelt_result() -> &'static str {
    "minecraft:sponge"
}

/// Wet sponge turns dry when placed in nether.
pub fn becomes_dry_in_nether() -> bool {
    true
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    #[test]
    fn constants_sane() {
        assert!(super::ABSORPTION_RANGE > 0);
    }
}
