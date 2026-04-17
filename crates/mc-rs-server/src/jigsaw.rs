//! Jigsaw block — structure placement + pattern targeting.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JigsawOrientation {
    DownEast,
    DownNorth,
    DownSouth,
    DownWest,
    UpEast,
    UpNorth,
    UpSouth,
    UpWest,
    WestUp,
    EastUp,
    NorthUp,
    SouthUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JigsawJointType {
    Rollable,
    Aligned,
}

#[derive(Debug, Clone)]
pub struct JigsawBlock {
    pub name: String,
    pub target: String,      // Pattern to search for
    pub pool: String,        // Template pool
    pub final_state: String, // Block to replace with
    pub joint: JigsawJointType,
    pub orientation: JigsawOrientation,
    pub selection_priority: i32,
    pub placement_priority: i32,
}

impl JigsawBlock {
    pub fn new() -> Self {
        Self {
            name: "minecraft:empty".into(),
            target: "minecraft:empty".into(),
            pool: "minecraft:empty".into(),
            final_state: "minecraft:air".into(),
            joint: JigsawJointType::Aligned,
            orientation: JigsawOrientation::UpNorth,
            selection_priority: 0,
            placement_priority: 0,
        }
    }
}

impl Default for JigsawBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_final_state_air() {
        assert_eq!(JigsawBlock::new().final_state, "minecraft:air");
    }
}
