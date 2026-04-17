//! Piston / Sticky Piston — port PMMP `src/block/Piston.php`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PistonKind {
    Normal,
    Sticky,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

#[derive(Debug, Clone)]
pub struct Piston {
    pub kind: PistonKind,
    pub facing: Facing,
    pub extended: bool,
    pub blocks_pushed: Vec<(i32, i32, i32, u16)>,
}

/// Max blocks pushable (12 vanilla).
pub const MAX_PUSHED_BLOCKS: usize = 12;
/// Extend/retract animation duration (2 ticks).
pub const EXTEND_DURATION: u32 = 2;

impl Piston {
    pub fn new(kind: PistonKind, facing: Facing) -> Self {
        Self {
            kind,
            facing,
            extended: false,
            blocks_pushed: Vec::new(),
        }
    }

    /// Check if a block can be pushed by piston.
    /// PMMP: PistonBlockStateHelper::cannotBePushed
    pub fn can_push(block_id: u16) -> bool {
        match block_id {
            0   // air
            | 7 // bedrock
            | 51 // fire
            | 54 // chest
            | 52 // spawner
            | 130 // ender chest
            | 146 // trapped chest
            | 119 // end portal
            | 120 // end frame
            | 49 // obsidian
            | 145 // anvil
            => false,
            _ => true,
        }
    }

    pub fn can_pull(block_id: u16) -> bool {
        // Sticky piston pulls — mostly same rules as push for immovable blocks,
        // plus slime/honey blocks transfer.
        Self::can_push(block_id)
            && !matches!(block_id,
            0  // air
        )
    }

    pub fn extend(&mut self, blocks: Vec<(i32, i32, i32, u16)>) -> bool {
        if blocks.len() > MAX_PUSHED_BLOCKS {
            return false;
        }
        self.extended = true;
        self.blocks_pushed = blocks;
        true
    }

    pub fn retract(&mut self) -> Option<Vec<(i32, i32, i32, u16)>> {
        if !self.extended {
            return None;
        }
        self.extended = false;
        Some(std::mem::take(&mut self.blocks_pushed))
    }

    /// Sticky pulls a block.
    pub fn sticky_pulls(&self) -> bool {
        self.kind == PistonKind::Sticky
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obsidian_not_pushable() {
        assert!(!Piston::can_push(49));
    }

    #[test]
    fn stone_pushable() {
        assert!(Piston::can_push(1));
    }

    #[test]
    fn max_12_blocks() {
        let mut p = Piston::new(PistonKind::Normal, Facing::Up);
        let v: Vec<_> = (0..13).map(|i| (i, 0, 0, 1)).collect();
        assert!(!p.extend(v));
    }
}
