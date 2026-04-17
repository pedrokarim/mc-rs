//! Cartography table — map duplication, expansion, locking.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartographyOperation {
    Duplicate,     // Map + empty map → 2 copies
    Expand,        // Map + paper → larger scale
    Lock,          // Map + glass pane → locked (stops auto-update)
    MapWithBanner, // Map + named banner → marker
}

impl CartographyOperation {
    pub fn inputs(&self) -> (&'static str, &'static str) {
        match self {
            Self::Duplicate => ("map", "empty_map"),
            Self::Expand => ("map", "paper"),
            Self::Lock => ("map", "glass_pane"),
            Self::MapWithBanner => ("map", "banner"),
        }
    }

    /// Expansion max scale (4 = 1:16).
    pub fn max_scale() -> u8 {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_takes_map_and_empty_map() {
        let (a, b) = CartographyOperation::Duplicate.inputs();
        assert_eq!(a, "map");
        assert_eq!(b, "empty_map");
    }
}
