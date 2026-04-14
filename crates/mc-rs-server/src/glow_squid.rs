//! Glow squid — lumineux, ink glow.

#[derive(Debug, Clone)]
pub struct GlowSquid {
    pub glow_ticks: u32,
    pub dark_ticks: u32,
}

/// Goes dark when hit for 100 ticks.
pub const DARK_DURATION: u32 = 100;
/// Light emission when glowing.
pub const LIGHT_EMISSION: u8 = 5;

impl GlowSquid {
    pub fn new() -> Self {
        Self { glow_ticks: 0, dark_ticks: 0 }
    }

    pub fn tick(&mut self) {
        if self.dark_ticks > 0 {
            self.dark_ticks -= 1;
        } else {
            self.glow_ticks += 1;
        }
    }

    /// Hit causes 5s darkness.
    pub fn darken(&mut self) {
        self.dark_ticks = DARK_DURATION;
    }

    pub fn is_glowing(&self) -> bool {
        self.dark_ticks == 0
    }

    pub fn drops() -> &'static str { "minecraft:glow_ink_sac" }
}

impl Default for GlowSquid {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_turns_off_glow() {
        let mut g = GlowSquid::new();
        g.darken();
        assert!(!g.is_glowing());
    }
}
