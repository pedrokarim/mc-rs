//! Glass panes / iron bars — thin blocks that connect.

#[derive(Debug, Clone)]
pub struct GlassPane {
    pub connects_north: bool,
    pub connects_south: bool,
    pub connects_east: bool,
    pub connects_west: bool,
    pub waterlogged: bool,
}

impl GlassPane {
    pub fn new() -> Self {
        Self {
            connects_north: false,
            connects_south: false,
            connects_east: false,
            connects_west: false,
            waterlogged: false,
        }
    }

    pub fn update_connections(&mut self, n: bool, s: bool, e: bool, w: bool) {
        self.connects_north = n;
        self.connects_south = s;
        self.connects_east = e;
        self.connects_west = w;
    }

    /// Breaks silently when hit (no XP).
    pub fn drops_self(tool: &str) -> bool {
        tool == "minecraft:shears" || tool.contains("silk_touch")
    }
}

impl Default for GlassPane {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shears_drop_pane() {
        assert!(GlassPane::drops_self("minecraft:shears"));
    }
}
