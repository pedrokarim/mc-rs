//! Shield decoration with banner.

#[derive(Debug, Clone)]
pub struct ShieldDecoration {
    pub base_color: u8,
    pub patterns: Vec<(u8, u8)>, // (pattern_id, color_id)
}

/// Max patterns on shield.
pub const MAX_PATTERNS: usize = 6;

impl ShieldDecoration {
    pub fn new(base: u8) -> Self {
        Self { base_color: base, patterns: Vec::new() }
    }

    pub fn apply_banner(&mut self, banner_color: u8, banner_patterns: Vec<(u8, u8)>) {
        self.base_color = banner_color;
        self.patterns = banner_patterns;
    }

    /// Wash shield to remove patterns (cauldron).
    pub fn wash(&mut self) -> bool {
        self.patterns.pop().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wash_removes_pattern() {
        let mut s = ShieldDecoration::new(0);
        s.patterns.push((1, 0));
        assert!(s.wash());
        assert!(s.patterns.is_empty());
    }
}
