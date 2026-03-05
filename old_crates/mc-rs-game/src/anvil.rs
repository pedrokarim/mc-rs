//! Anvil logic — item renaming, repair with materials, and item combination
//! (durability merge + enchantment fusion).

use mc_rs_nbt::tag::{NbtCompound, NbtRoot, NbtTag};
use mc_rs_proto::item_stack::ItemStack;

use crate::combat::{build_enchantment_nbt, parse_enchantments, Enchantment};
use crate::enchanting::conflicts;

/// Result of an anvil operation.
pub struct AnvilResult {
    /// The output item.
    pub output: ItemStack,
    /// XP level cost for this operation.
    pub xp_cost: i32,
}

/// Compute the anvil output for the given inputs.
///
/// `get_item_name` resolves a runtime_id to the item's string name
/// (e.g. `"minecraft:iron_sword"`).
///
/// Returns `None` if the inputs don't produce a valid operation.
pub fn compute_anvil_output(
    input: &ItemStack,
    material: &ItemStack,
    new_name: Option<&str>,
    get_item_name: impl Fn(i32) -> Option<String>,
) -> Option<AnvilResult> {
    if input.is_empty() {
        return None;
    }

    let input_name = get_item_name(input.runtime_id)?;
    let mut cost = 0i32;
    let mut output = input.clone();

    // --- Rename ---
    let has_rename = new_name.is_some_and(|n| !n.is_empty());
    if has_rename {
        cost += 1;
        output.nbt_data = set_display_name(&output.nbt_data, new_name.unwrap());
    }

    if material.is_empty() {
        // Rename-only: need a name to rename
        if !has_rename {
            return None;
        }
        return Some(AnvilResult {
            output,
            xp_cost: cost,
        });
    }

    let material_name = get_item_name(material.runtime_id)?;

    // --- Repair with matching material ---
    if let Some(repair_mat) = repair_material(&input_name) {
        if material_name == repair_mat {
            // Each unit of material restores 25% of max durability
            // Simplified: reduce metadata (damage) by 25% of max per unit consumed
            // metadata represents damage in Bedrock
            let units = material.count.min(4) as i32; // max 4 units meaningful
            cost += units;
            // Reduce damage (metadata) — lower metadata = less damaged
            let repair_per_unit = 100i32; // ~25% of typical 400 durability
            let new_damage = (output.metadata as i32 - units * repair_per_unit).max(0);
            output.metadata = new_damage as u16;
            return Some(AnvilResult {
                output,
                xp_cost: cost,
            });
        }
    }

    // --- Combine two identical items ---
    if input.runtime_id == material.runtime_id {
        cost += 1; // base combination cost

        // Merge durability: input + material remaining + 12% bonus
        let combined_damage = (input.metadata as i32 + material.metadata as i32) / 2;
        output.metadata = combined_damage.max(0) as u16;

        // Merge enchantments
        let base_enchs = parse_enchantments(&input.nbt_data);
        let addition_enchs = parse_enchantments(&material.nbt_data);
        let merged = merge_enchantments(&base_enchs, &addition_enchs);

        // Count transferred enchantments for cost
        let transferred = merged.len() as i32 - base_enchs.len() as i32;
        cost += transferred.max(0);

        // Rebuild NBT with merged enchantments + optional display name
        output.nbt_data = build_anvil_nbt(&merged, new_name.filter(|n| !n.is_empty()));

        return Some(AnvilResult {
            output,
            xp_cost: cost,
        });
    }

    // --- Enchanted book application ---
    if material_name == "minecraft:enchanted_book" {
        let base_enchs = parse_enchantments(&input.nbt_data);
        let book_enchs = parse_enchantments(&material.nbt_data);
        if book_enchs.is_empty() {
            return None;
        }
        let merged = merge_enchantments(&base_enchs, &book_enchs);
        let transferred = merged.len() as i32 - base_enchs.len() as i32;
        cost += transferred.max(1);
        output.nbt_data = build_anvil_nbt(&merged, new_name.filter(|n| !n.is_empty()));
        return Some(AnvilResult {
            output,
            xp_cost: cost,
        });
    }

    // No valid operation with these inputs
    if has_rename {
        // Still valid as rename-only even if material doesn't match
        Some(AnvilResult {
            output,
            xp_cost: cost,
        })
    } else {
        None
    }
}

/// Get the repair material item name for a given tool/armor item.
pub fn repair_material(item_name: &str) -> Option<&'static str> {
    let name = item_name.strip_prefix("minecraft:").unwrap_or(item_name);
    if name.starts_with("wooden_") || name == "bow" || name == "fishing_rod" {
        Some("minecraft:planks")
    } else if name.starts_with("stone_") {
        Some("minecraft:cobblestone")
    } else if name.starts_with("iron_")
        || name == "chainmail_helmet"
        || name == "chainmail_chestplate"
        || name == "chainmail_leggings"
        || name == "chainmail_boots"
    {
        Some("minecraft:iron_ingot")
    } else if name.starts_with("golden_") {
        Some("minecraft:gold_ingot")
    } else if name.starts_with("diamond_") {
        Some("minecraft:diamond")
    } else if name.starts_with("netherite_") {
        Some("minecraft:netherite_ingot")
    } else if name.starts_with("leather_") {
        Some("minecraft:leather")
    } else if name == "turtle_helmet" {
        Some("minecraft:turtle_scute")
    } else if name == "elytra" {
        Some("minecraft:phantom_membrane")
    } else {
        None
    }
}

/// Set or update the display name in item NBT data (network format).
///
/// Adds/modifies the `display.Name` tag inside the root compound.
pub fn set_display_name(nbt_data: &[u8], name: &str) -> Vec<u8> {
    let mut root_compound = if nbt_data.is_empty() {
        NbtCompound::new()
    } else {
        match mc_rs_nbt::read_nbt_network(&mut &nbt_data[..]) {
            Ok(root) => root.compound,
            Err(_) => NbtCompound::new(),
        }
    };

    // Get or create display compound
    let mut display = match root_compound.remove("display") {
        Some(NbtTag::Compound(c)) => c,
        _ => NbtCompound::new(),
    };
    display.insert("Name".to_string(), NbtTag::String(name.to_string()));
    root_compound.insert("display".to_string(), NbtTag::Compound(display));

    let root = NbtRoot::new("", root_compound);
    let mut buf = Vec::new();
    mc_rs_nbt::write_nbt_network(&mut buf, &root);
    buf
}

/// Merge enchantments from a base item and an addition item.
///
/// Rules:
/// - Same enchantment: take the higher level, or +1 if same level (capped at max_level)
/// - Conflicting enchantments: keep base, discard addition
/// - New enchantments from addition are added if no conflict
pub fn merge_enchantments(base: &[Enchantment], addition: &[Enchantment]) -> Vec<Enchantment> {
    let mut result: Vec<Enchantment> = base.to_vec();

    for add_ench in addition {
        // Check if this enchantment conflicts with any existing one
        let has_conflict = result
            .iter()
            .any(|r| r.id != add_ench.id && conflicts(r.id, add_ench.id));
        if has_conflict {
            continue;
        }

        // Check if we already have this enchantment
        if let Some(existing) = result.iter_mut().find(|r| r.id == add_ench.id) {
            if add_ench.level > existing.level {
                existing.level = add_ench.level;
            } else if add_ench.level == existing.level {
                // Same level: +1 up to max
                let max_lvl = max_level_for(add_ench.id);
                existing.level = (existing.level + 1).min(max_lvl);
            }
        } else {
            // New enchantment, no conflict — add it
            result.push(*add_ench);
        }
    }

    result
}

/// Build NBT data with enchantments and optional display name (network format).
fn build_anvil_nbt(enchantments: &[Enchantment], display_name: Option<&str>) -> Vec<u8> {
    if enchantments.is_empty() && display_name.is_none() {
        return Vec::new();
    }

    // Start with enchantment NBT if present
    let mut root_compound = if !enchantments.is_empty() {
        let data = build_enchantment_nbt(enchantments);
        match mc_rs_nbt::read_nbt_network(&mut &data[..]) {
            Ok(root) => root.compound,
            Err(_) => NbtCompound::new(),
        }
    } else {
        NbtCompound::new()
    };

    // Add display name if present
    if let Some(name) = display_name {
        let mut display = NbtCompound::new();
        display.insert("Name".to_string(), NbtTag::String(name.to_string()));
        root_compound.insert("display".to_string(), NbtTag::Compound(display));
    }

    let root = NbtRoot::new("", root_compound);
    let mut buf = Vec::new();
    mc_rs_nbt::write_nbt_network(&mut buf, &root);
    buf
}

/// Get the maximum level for a given enchantment ID.
fn max_level_for(id: i16) -> i16 {
    use crate::combat::ENCHANTMENT_LIST;
    ENCHANTMENT_LIST
        .iter()
        .find(|e| e.id == id)
        .map(|e| e.max_level)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(runtime_id: i32) -> ItemStack {
        ItemStack {
            runtime_id,
            count: 1,
            metadata: 0,
            block_runtime_id: 0,
            nbt_data: Vec::new(),
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
            stack_network_id: 0,
        }
    }

    fn make_item_with_enchants(runtime_id: i32, enchs: &[(i16, i16)]) -> ItemStack {
        let enchantments: Vec<Enchantment> = enchs
            .iter()
            .map(|&(id, level)| Enchantment { id, level })
            .collect();
        let nbt_data = build_enchantment_nbt(&enchantments);
        ItemStack {
            runtime_id,
            count: 1,
            metadata: 0,
            block_runtime_id: 0,
            nbt_data,
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
            stack_network_id: 0,
        }
    }

    fn name_lookup(rid: i32) -> Option<String> {
        match rid {
            1 => Some("minecraft:iron_sword".to_string()),
            2 => Some("minecraft:diamond_pickaxe".to_string()),
            3 => Some("minecraft:iron_ingot".to_string()),
            4 => Some("minecraft:diamond".to_string()),
            5 => Some("minecraft:enchanted_book".to_string()),
            _ => None,
        }
    }

    #[test]
    fn empty_input_no_output() {
        let result =
            compute_anvil_output(&ItemStack::empty(), &ItemStack::empty(), None, name_lookup);
        assert!(result.is_none());
    }

    #[test]
    fn rename_item() {
        let input = make_item(1); // iron_sword
        let result =
            compute_anvil_output(&input, &ItemStack::empty(), Some("Excalibur"), name_lookup)
                .unwrap();
        assert_eq!(result.xp_cost, 1);
        assert!(!result.output.nbt_data.is_empty());

        // Verify display name in NBT
        let root = mc_rs_nbt::read_nbt_network(&mut &result.output.nbt_data[..]).unwrap();
        let display = root.compound.get("display").unwrap().as_compound().unwrap();
        let name = display.get("Name").unwrap().as_string().unwrap();
        assert_eq!(name, "Excalibur");
    }

    #[test]
    fn repair_with_material() {
        let mut input = make_item(1); // iron_sword
        input.metadata = 200; // damaged
        let mut material = make_item(3); // iron_ingot
        material.count = 2;
        let result = compute_anvil_output(&input, &material, None, name_lookup).unwrap();
        assert!(result.output.metadata < 200); // less damaged
        assert!(result.xp_cost >= 2); // 2 units consumed
    }

    #[test]
    fn combine_items() {
        let input = make_item_with_enchants(1, &[(9, 3)]); // Sharpness III
        let material = make_item_with_enchants(1, &[(14, 2)]); // Unbreaking II
        let result = compute_anvil_output(&input, &material, None, name_lookup).unwrap();
        assert!(result.xp_cost >= 1);

        // Verify merged enchantments
        let enchs = parse_enchantments(&result.output.nbt_data);
        assert!(enchs.iter().any(|e| e.id == 9 && e.level == 3)); // Sharpness III
        assert!(enchs.iter().any(|e| e.id == 14 && e.level == 2)); // Unbreaking II
    }

    #[test]
    fn merge_enchantments_basic() {
        let base = vec![
            Enchantment { id: 9, level: 3 }, // Sharpness III
        ];
        let addition = vec![
            Enchantment { id: 14, level: 2 }, // Unbreaking II
        ];
        let merged = merge_enchantments(&base, &addition);
        assert_eq!(merged.len(), 2);
        assert!(merged.iter().any(|e| e.id == 9 && e.level == 3));
        assert!(merged.iter().any(|e| e.id == 14 && e.level == 2));
    }

    #[test]
    fn merge_enchantments_same_level_upgrade() {
        let base = vec![Enchantment { id: 9, level: 3 }]; // Sharpness III
        let addition = vec![Enchantment { id: 9, level: 3 }]; // Sharpness III
        let merged = merge_enchantments(&base, &addition);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id, 9);
        assert_eq!(merged[0].level, 4); // Sharpness IV (III + III → IV)
    }

    #[test]
    fn merge_enchantments_conflict() {
        use crate::combat::enchantment_id::{SHARPNESS, SMITE};
        let base = vec![Enchantment {
            id: SHARPNESS,
            level: 3,
        }];
        let addition = vec![Enchantment {
            id: SMITE,
            level: 4,
        }];
        let merged = merge_enchantments(&base, &addition);
        // Smite conflicts with Sharpness — should be discarded
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id, SHARPNESS);
    }

    #[test]
    fn repair_material_lookup() {
        assert_eq!(
            repair_material("minecraft:iron_sword"),
            Some("minecraft:iron_ingot")
        );
        assert_eq!(
            repair_material("minecraft:diamond_pickaxe"),
            Some("minecraft:diamond")
        );
        assert_eq!(
            repair_material("minecraft:golden_helmet"),
            Some("minecraft:gold_ingot")
        );
        assert_eq!(
            repair_material("minecraft:netherite_sword"),
            Some("minecraft:netherite_ingot")
        );
        assert_eq!(
            repair_material("minecraft:leather_chestplate"),
            Some("minecraft:leather")
        );
        assert_eq!(
            repair_material("minecraft:turtle_helmet"),
            Some("minecraft:turtle_scute")
        );
        assert_eq!(
            repair_material("minecraft:elytra"),
            Some("minecraft:phantom_membrane")
        );
        assert_eq!(repair_material("minecraft:dirt"), None);
    }
}
