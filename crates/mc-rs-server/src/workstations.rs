//! Workstations — port PMMP `src/block/*` pour grindstone, smithing table,
//! stonecutter, loom, cartography table.

use mc_rs_proto::packets::player::ItemStack;
use crate::enchantments::EnchantmentInstance;

// ── Grindstone : retire les enchants d'un item et donne de l'XP ──────────────

#[derive(Debug, Clone)]
pub struct GrindstoneResult {
    pub output: ItemStack,
    pub xp_orbs: u32,
}

pub fn grindstone_process(
    item: &ItemStack,
    enchants: &[EnchantmentInstance],
) -> GrindstoneResult {
    let xp_orbs = enchants.iter().map(|e| e.level as u32 * 3).sum();
    let mut output = item.clone();
    // Si item était damagé, retire 25% du damage.
    output.meta = output.meta.saturating_sub(output.meta / 4);
    GrindstoneResult { output, xp_orbs }
}

// ── Smithing table : upgrade diamond → netherite ────────────────────────────

pub fn smithing_upgrade(diamond_item_id: i32) -> Option<i32> {
    use crate::item_registry::network_id;
    let table: &[(&str, &str)] = &[
        ("minecraft:diamond_helmet", "minecraft:netherite_helmet"),
        ("minecraft:diamond_chestplate", "minecraft:netherite_chestplate"),
        ("minecraft:diamond_leggings", "minecraft:netherite_leggings"),
        ("minecraft:diamond_boots", "minecraft:netherite_boots"),
        ("minecraft:diamond_sword", "minecraft:netherite_sword"),
        ("minecraft:diamond_pickaxe", "minecraft:netherite_pickaxe"),
        ("minecraft:diamond_axe", "minecraft:netherite_axe"),
        ("minecraft:diamond_shovel", "minecraft:netherite_shovel"),
        ("minecraft:diamond_hoe", "minecraft:netherite_hoe"),
    ];
    for (from, to) in table {
        if network_id(from) == Some(diamond_item_id) {
            return network_id(to);
        }
    }
    None
}

// ── Stonecutter : découpe pierre en variants ────────────────────────────────

pub fn stonecutter_outputs_for(item_name: &str) -> Vec<&'static str> {
    match item_name {
        "minecraft:stone" => vec![
            "minecraft:stone_bricks",
            "minecraft:stone_stairs",
            "minecraft:stone_slab",
            "minecraft:chiseled_stone_bricks",
        ],
        "minecraft:cobblestone" => vec![
            "minecraft:cobblestone_stairs",
            "minecraft:cobblestone_slab",
            "minecraft:cobblestone_wall",
        ],
        "minecraft:sandstone" => vec![
            "minecraft:sandstone_stairs",
            "minecraft:sandstone_slab",
            "minecraft:chiseled_sandstone",
            "minecraft:cut_sandstone",
            "minecraft:sandstone_wall",
        ],
        "minecraft:oak_planks" => vec![
            "minecraft:oak_stairs",
            "minecraft:oak_slab",
            "minecraft:oak_fence",
            "minecraft:oak_fence_gate",
            "minecraft:oak_door",
            "minecraft:oak_trapdoor",
            "minecraft:oak_pressure_plate",
            "minecraft:oak_button",
        ],
        _ => vec![],
    }
}

// ── Loom : applique pattern banner ──────────────────────────────────────────

use crate::banner::{Banner, BannerColor, BannerPattern, BannerPatternType};

pub fn loom_apply_pattern(
    banner: &mut Banner,
    dye_color: BannerColor,
    pattern: BannerPatternType,
) {
    banner.add_pattern(BannerPattern {
        pattern_type: pattern,
        color: dye_color,
    });
}

// ── Cartography table : copie/extend/lock/zoom une map ──────────────────────

use crate::maps::{MapData, MapScale};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartographyOp {
    Copy,
    Zoom, // increase scale
    Lock,
}

pub fn cartography_apply(map: &mut MapData, op: CartographyOp) -> bool {
    match op {
        CartographyOp::Copy => true, // new map with same data
        CartographyOp::Zoom => {
            match map.scale {
                MapScale::Level0 => { map.scale = MapScale::Level1; true }
                MapScale::Level1 => { map.scale = MapScale::Level2; true }
                MapScale::Level2 => { map.scale = MapScale::Level3; true }
                MapScale::Level3 => { map.scale = MapScale::Level4; true }
                MapScale::Level4 => false, // already max
            }
        }
        CartographyOp::Lock => {
            if map.locked {
                false
            } else {
                map.locked = true;
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smithing_upgrades_diamond_sword() {
        use crate::item_registry::network_id;
        if let Some(ds) = network_id("minecraft:diamond_sword") {
            let ns = smithing_upgrade(ds);
            assert_eq!(ns, network_id("minecraft:netherite_sword"));
        }
    }

    #[test]
    fn stonecutter_stone_options() {
        let out = stonecutter_outputs_for("minecraft:stone");
        assert!(out.contains(&"minecraft:stone_bricks"));
    }

    #[test]
    fn cartography_lock_once() {
        let mut m = MapData::new(1, MapScale::Level0, [0, 0], 0);
        assert!(cartography_apply(&mut m, CartographyOp::Lock));
        assert!(m.locked);
        assert!(!cartography_apply(&mut m, CartographyOp::Lock));
    }

    #[test]
    fn cartography_zoom_caps_at_4() {
        let mut m = MapData::new(1, MapScale::Level3, [0, 0], 0);
        assert!(cartography_apply(&mut m, CartographyOp::Zoom));
        assert_eq!(m.scale, MapScale::Level4);
        assert!(!cartography_apply(&mut m, CartographyOp::Zoom));
    }
}
