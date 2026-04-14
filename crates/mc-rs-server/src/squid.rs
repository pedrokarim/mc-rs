//! Squid — mob aquatique qui drop ink sacs.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquidVariant {
    Normal,  // Drops ink sac
    Glow,    // Drops glow ink sac
}

#[derive(Debug, Clone)]
pub struct Squid {
    pub variant: SquidVariant,
    pub ink_cooldown: u32,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
}

/// Ink cloud cooldown (100 ticks vanilla).
pub const INK_COOLDOWN: u32 = 100;

impl Squid {
    pub fn new(variant: SquidVariant) -> Self {
        Self {
            variant,
            ink_cooldown: 0,
            motion_x: 0.0,
            motion_y: 0.0,
            motion_z: 0.0,
        }
    }

    pub fn tick(&mut self) {
        if self.ink_cooldown > 0 {
            self.ink_cooldown -= 1;
        }
    }

    /// Release ink cloud when attacked.
    pub fn release_ink(&mut self) -> Option<&'static str> {
        if self.ink_cooldown > 0 {
            return None;
        }
        self.ink_cooldown = INK_COOLDOWN;
        match self.variant {
            SquidVariant::Normal => Some("minecraft:ink_sac"),
            SquidVariant::Glow => Some("minecraft:glow_ink_sac"),
        }
    }

    pub fn drop_item(&self) -> &'static str {
        match self.variant {
            SquidVariant::Normal => "minecraft:ink_sac",
            SquidVariant::Glow => "minecraft:glow_ink_sac",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ink_cloud_on_first_hit() {
        let mut s = Squid::new(SquidVariant::Normal);
        assert!(s.release_ink().is_some());
    }

    #[test]
    fn ink_cloud_cooldown() {
        let mut s = Squid::new(SquidVariant::Normal);
        s.release_ink();
        assert!(s.release_ink().is_none());
    }
}
