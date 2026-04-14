//! Workbench (crafting table) — 3x3 grid.

/// 3x3 grid.
pub const GRID_WIDTH: usize = 3;
pub const GRID_HEIGHT: usize = 3;
pub const GRID_SIZE: usize = 9;

#[derive(Debug, Clone)]
pub struct WorkbenchGrid {
    pub slots: [Option<(u16, u16)>; GRID_SIZE],
    pub output_slot: Option<(u16, u16)>,
}

impl WorkbenchGrid {
    pub fn new() -> Self {
        Self {
            slots: [None; GRID_SIZE],
            output_slot: None,
        }
    }

    /// Shift a pattern so it starts at a normalized position.
    pub fn normalize_shape(items: &[(Option<u16>, u16)]) -> Vec<(usize, usize, u16)> {
        let mut min_x = GRID_WIDTH;
        let mut min_y = GRID_HEIGHT;
        let mut positions = Vec::new();
        for (i, entry) in items.iter().enumerate() {
            if let (Some(id), _) = entry {
                let x = i % GRID_WIDTH;
                let y = i / GRID_WIDTH;
                positions.push((x, y, *id));
                min_x = min_x.min(x);
                min_y = min_y.min(y);
            }
        }
        positions
            .into_iter()
            .map(|(x, y, id)| (x - min_x, y - min_y, id))
            .collect()
    }
}

impl Default for WorkbenchGrid {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_offset_pattern() {
        let items = vec![
            (None, 0), (None, 0), (None, 0),
            (None, 0), (Some(1u16), 1), (None, 0),
            (None, 0), (None, 0), (None, 0),
        ];
        let norm = WorkbenchGrid::normalize_shape(&items);
        assert_eq!(norm, vec![(0, 0, 1u16)]);
    }
}
