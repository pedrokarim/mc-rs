//! Block light propagation.

/// Max light level (15).
pub const MAX_LIGHT: u8 = 15;

/// Update block light when placing/removing light source.
pub fn propagate_from_source(source_light: u8, distance: u8) -> u8 {
    source_light.saturating_sub(distance).min(MAX_LIGHT)
}

/// Priority queue of blocks to update.
#[derive(Debug, Clone, Default)]
pub struct LightUpdateContext {
    pub dirty_blocks: Vec<(i32, i32, i32)>,
    pub max_updates_per_tick: usize,
}

impl LightUpdateContext {
    pub fn new(max: usize) -> Self {
        Self {
            dirty_blocks: Vec::new(),
            max_updates_per_tick: max,
        }
    }

    pub fn enqueue(&mut self, pos: (i32, i32, i32)) {
        self.dirty_blocks.push(pos);
    }

    pub fn take_batch(&mut self) -> Vec<(i32, i32, i32)> {
        let n = self.max_updates_per_tick.min(self.dirty_blocks.len());
        self.dirty_blocks.drain(..n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn propagates_decay() {
        assert_eq!(propagate_from_source(15, 3), 12);
    }
}
