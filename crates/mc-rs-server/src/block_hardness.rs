//! Block hardness + tool class requirements — port PMMP `src/block/Block::getHardness`.

use crate::durability::{ToolTier, ToolType};

/// Hardness vanilla (seconds requis pour casser avec main nue).
pub fn hardness(block_name: &str) -> f32 {
    match block_name {
        "minecraft:air" => 0.0,
        "minecraft:bedrock" => -1.0, // unbreakable
        "minecraft:obsidian" => 50.0,
        "minecraft:crying_obsidian" | "minecraft:respawn_anchor" => 50.0,
        "minecraft:netherite_block" | "minecraft:ancient_debris" => 50.0,
        "minecraft:enchanting_table" => 5.0,
        "minecraft:stone" | "minecraft:cobblestone" | "minecraft:deepslate" => 1.5,
        "minecraft:iron_ore" | "minecraft:coal_ore" | "minecraft:copper_ore"
        | "minecraft:gold_ore" | "minecraft:redstone_ore" | "minecraft:lapis_ore" => 3.0,
        "minecraft:diamond_ore" | "minecraft:emerald_ore" => 3.0,
        "minecraft:iron_block" | "minecraft:diamond_block" | "minecraft:emerald_block"
        | "minecraft:gold_block" => 5.0,
        "minecraft:dirt" | "minecraft:grass_block" | "minecraft:sand" | "minecraft:gravel" => 0.5,
        "minecraft:snow_block" => 0.2,
        "minecraft:ice" | "minecraft:packed_ice" => 0.5,
        "minecraft:glass" | "minecraft:glowstone" | "minecraft:sea_lantern" => 0.3,
        "minecraft:oak_log" | "minecraft:birch_log" | "minecraft:spruce_log"
        | "minecraft:jungle_log" | "minecraft:acacia_log" | "minecraft:dark_oak_log" => 2.0,
        "minecraft:oak_planks" | "minecraft:birch_planks" | "minecraft:spruce_planks"
        | "minecraft:jungle_planks" | "minecraft:acacia_planks" | "minecraft:dark_oak_planks" => 2.0,
        "minecraft:leaves" | "minecraft:oak_leaves" | "minecraft:birch_leaves"
        | "minecraft:spruce_leaves" | "minecraft:jungle_leaves" | "minecraft:acacia_leaves"
        | "minecraft:dark_oak_leaves" => 0.2,
        "minecraft:wool" => 0.8,
        _ => 1.0,
    }
}

/// Tool type requis pour drop l'item normal (sinon cassé mais pas de drop).
pub fn required_tool_type(block_name: &str) -> Option<ToolType> {
    match block_name {
        "minecraft:stone" | "minecraft:cobblestone" | "minecraft:deepslate"
        | "minecraft:iron_ore" | "minecraft:coal_ore" | "minecraft:gold_ore"
        | "minecraft:diamond_ore" | "minecraft:emerald_ore" | "minecraft:redstone_ore"
        | "minecraft:lapis_ore" | "minecraft:obsidian" | "minecraft:netherite_block"
        | "minecraft:ancient_debris" => Some(ToolType::Pickaxe),
        "minecraft:oak_log" | "minecraft:birch_log" | "minecraft:spruce_log"
        | "minecraft:jungle_log" | "minecraft:acacia_log" | "minecraft:dark_oak_log"
        | "minecraft:oak_planks" | "minecraft:birch_planks" | "minecraft:spruce_planks"
        | "minecraft:jungle_planks" | "minecraft:acacia_planks" | "minecraft:dark_oak_planks" => {
            Some(ToolType::Axe)
        }
        "minecraft:dirt" | "minecraft:grass_block" | "minecraft:sand" | "minecraft:gravel"
        | "minecraft:clay" | "minecraft:podzol" | "minecraft:mycelium" | "minecraft:snow_layer"
        | "minecraft:snow_block" => Some(ToolType::Shovel),
        _ => None,
    }
}

/// Tier minimum de pickaxe pour drop l'ore.
pub fn min_tool_tier_for_drop(block_name: &str) -> Option<ToolTier> {
    match block_name {
        "minecraft:stone" | "minecraft:cobblestone" | "minecraft:coal_ore" => Some(ToolTier::Wood),
        "minecraft:iron_ore" | "minecraft:lapis_ore" | "minecraft:copper_ore" => Some(ToolTier::Stone),
        "minecraft:diamond_ore" | "minecraft:gold_ore" | "minecraft:redstone_ore"
        | "minecraft:emerald_ore" | "minecraft:deepslate" => Some(ToolTier::Iron),
        "minecraft:obsidian" | "minecraft:crying_obsidian" | "minecraft:respawn_anchor"
        | "minecraft:ancient_debris" | "minecraft:netherite_block" => Some(ToolTier::Diamond),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bedrock_unbreakable() {
        assert!(hardness("minecraft:bedrock") < 0.0);
    }

    #[test]
    fn obsidian_requires_diamond_pickaxe() {
        assert_eq!(min_tool_tier_for_drop("minecraft:obsidian"), Some(ToolTier::Diamond));
        assert_eq!(required_tool_type("minecraft:obsidian"), Some(ToolType::Pickaxe));
    }
}
