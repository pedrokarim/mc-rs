//! Block breaking logic — tool compat + hardness.

use crate::tool_types::{ToolType, ToolTier};

/// Material classification for block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockMaterial {
    Air,
    Stone,
    Wood,
    Metal,
    Dirt,
    Gravel,
    Sand,
    Web,
    Wool,
    Leaves,
    Plant,
    Water,
    Lava,
    Ice,
    Snow,
    Glass,
    Cobweb,
    Unbreakable,
    Pumpkin,
    Melon,
    Netherite,
}

impl BlockMaterial {
    /// What tool is effective for this material.
    pub fn effective_tool(&self) -> Option<ToolType> {
        match self {
            Self::Stone | Self::Metal | Self::Ice | Self::Netherite => Some(ToolType::Pickaxe),
            Self::Wood | Self::Pumpkin | Self::Melon => Some(ToolType::Axe),
            Self::Dirt | Self::Gravel | Self::Sand | Self::Snow => Some(ToolType::Shovel),
            Self::Plant => Some(ToolType::Hoe),
            Self::Web | Self::Cobweb => Some(ToolType::Shears),
            _ => None,
        }
    }

    /// Required tool tier.
    pub fn required_tier(&self, block_id: u16) -> Option<ToolTier> {
        match self {
            Self::Stone => Some(match block_id {
                // Iron ore = stone tier
                // Diamond ore = iron tier
                // Obsidian = diamond tier
                15 | 73 => Some(ToolTier::Stone),
                49 => Some(ToolTier::Diamond),
                56 | 129 | 153 => Some(ToolTier::Iron),
                _ => Some(ToolTier::Wood),
            })?,
            Self::Metal => Some(ToolTier::Stone),
            _ => None,
        }
    }

    /// Drops items when broken without correct tool?
    pub fn drops_with_hand(&self) -> bool {
        matches!(self,
            Self::Air | Self::Wood | Self::Dirt | Self::Gravel | Self::Sand
            | Self::Wool | Self::Leaves | Self::Plant | Self::Pumpkin | Self::Melon
            | Self::Snow | Self::Glass | Self::Cobweb
        )
    }
}

/// Break time formula (vanilla).
pub fn break_time_ticks(hardness: f32, tool_speed: f32, can_harvest: bool) -> u32 {
    let base_factor = if can_harvest { 1.5 } else { 5.0 };
    ((hardness * base_factor) / tool_speed * 20.0).ceil() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wood_is_axe_tool() {
        assert_eq!(BlockMaterial::Wood.effective_tool(), Some(ToolType::Axe));
    }

    #[test]
    fn faster_with_correct_tool() {
        let correct = break_time_ticks(1.5, 6.0, true);
        let incorrect = break_time_ticks(1.5, 1.0, false);
        assert!(correct < incorrect);
    }
}
