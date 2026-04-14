//! Torchflower — grown from torchflower seeds (Sniffer drop).

/// Growth stages (0-1).
pub const MAX_STAGE: u8 = 1;
/// Growth chance.
pub const GROWTH_CHANCE: f32 = 0.10;
/// Valid ground: farmland (like crops).
pub fn valid_ground() -> &'static [u16] {
    &[60] // farmland
}

/// Drop: 1 torchflower seed + chance for 2.
pub const DROP_BASE: u32 = 1;

#[cfg(test)]
mod tests {
    #[test]
    fn farmland_valid() {
        assert!(super::valid_ground().contains(&60));
    }
}
