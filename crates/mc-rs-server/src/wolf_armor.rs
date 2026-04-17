//! Wolf armor — armadillo scute crafted.

#[derive(Debug, Clone)]
pub struct WolfArmor {
    pub durability: u16,
    pub custom_color: Option<[u8; 3]>,
    pub pattern: Option<u8>,
}

pub const MAX_DURABILITY: u16 = 64;
/// Armor points given.
pub const DEFENSE: u8 = 2;

impl WolfArmor {
    pub fn new() -> Self {
        Self {
            durability: MAX_DURABILITY,
            custom_color: None,
            pattern: None,
        }
    }

    /// Crafted from 6 armadillo scutes.
    pub fn recipe() -> (&'static [&'static str], &'static str) {
        (&["minecraft:armadillo_scute"; 6], "minecraft:wolf_armor")
    }

    /// Dye wolf armor (leather dyeing).
    pub fn dye(&mut self, color: [u8; 3]) {
        self.custom_color = Some(color);
    }
}

impl Default for WolfArmor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_durability_64() {
        assert_eq!(WolfArmor::new().durability, MAX_DURABILITY);
    }
}
