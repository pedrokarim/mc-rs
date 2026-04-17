//! Spyglass — zoom in (1.17+).

#[derive(Debug, Clone, Copy)]
pub struct SpyglassUse {
    pub fov_multiplier: f32, // Default 0.1 (10x zoom)
    pub usage_ticks: u32,
}

impl SpyglassUse {
    pub fn new() -> Self {
        Self {
            fov_multiplier: 0.1,
            usage_ticks: 0,
        }
    }

    pub fn tick(&mut self) {
        self.usage_ticks += 1;
    }

    pub fn stop(&mut self) {
        self.usage_ticks = 0;
    }

    /// Slows player movement while using.
    pub fn movement_multiplier() -> f32 {
        0.1
    }
}

impl Default for SpyglassUse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn slows_movement() {
        assert!(super::SpyglassUse::movement_multiplier() < 1.0);
    }
}
