//! Glow berry — food + light source via cave vines.

/// Hunger.
pub const HUNGER: u8 = 2;
/// Saturation.
pub const SATURATION: f32 = 0.4;

/// Feeding fox trusts food.
pub fn fox_breed_item() -> bool {
    false
} // not used for breeding

/// Mobs eat glow berries (none specific).
pub fn mobs_that_eat_glow_berries() -> &'static [&'static str] {
    &[] // not used
}

#[cfg(test)]
mod tests {
    #[test]
    fn constant_hunger() {
        assert_eq!(super::HUNGER, 2);
    }
}
