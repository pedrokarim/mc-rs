//! Structure block — save/load structures.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureBlockMode {
    Save,
    Load,
    Corner,
    Data,
    Invalid,
}

#[derive(Debug, Clone)]
pub struct StructureBlock {
    pub mode: StructureBlockMode,
    pub name: String,
    pub author: String,
    pub position_offset: (i32, i32, i32),
    pub size: (u32, u32, u32),
    pub rotation: u8, // 0,1,2,3 = 0, 90, 180, 270
    pub mirror: u8,   // 0=none, 1=LR, 2=FB
    pub ignore_entities: bool,
    pub show_bounding_box: bool,
    pub integrity: f32,
    pub seed: u64,
}

/// Max structure size (48^3).
pub const MAX_SIZE: u32 = 48;

impl StructureBlock {
    pub fn new(mode: StructureBlockMode) -> Self {
        Self {
            mode,
            name: String::new(),
            author: String::new(),
            position_offset: (0, 0, 0),
            size: (1, 1, 1),
            rotation: 0,
            mirror: 0,
            ignore_entities: false,
            show_bounding_box: true,
            integrity: 1.0,
            seed: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_size() {
        let s = StructureBlock::new(StructureBlockMode::Save);
        assert_eq!(s.size, (1, 1, 1));
    }
}
