//! Grindstone logic — remove enchantments and compute XP refund.

use mc_rs_proto::item_stack::ItemStack;

/// Compute the grindstone output: strip enchantments from the input item.
///
/// Returns `None` if there is nothing to disenchant.
pub fn compute_grindstone_output(input1: &ItemStack, input2: &ItemStack) -> Option<ItemStack> {
    if input1.is_empty() && input2.is_empty() {
        return None;
    }

    // Primary item is whichever slot is non-empty (prefer slot 1)
    let primary = if !input1.is_empty() { input1 } else { input2 };

    // Only produce output if item has enchantments (non-empty nbt_data with ench tag)
    if primary.nbt_data.is_empty() {
        return None;
    }

    // Check if item has enchantments
    if !has_enchantments(&primary.nbt_data) {
        return None;
    }

    // Output = same item without enchantments (clear nbt_data)
    Some(ItemStack {
        runtime_id: primary.runtime_id,
        count: primary.count,
        metadata: primary.metadata,
        block_runtime_id: primary.block_runtime_id,
        nbt_data: Vec::new(), // stripped
        can_place_on: primary.can_place_on.clone(),
        can_destroy: primary.can_destroy.clone(),
        stack_network_id: 0,
    })
}

/// Calculate XP reward from stripping enchantments via grindstone.
///
/// Each enchantment level contributes a random amount of XP.
/// Simplified: each enchantment level = 1-3 XP (we use the minimum: level count).
pub fn grindstone_xp_reward(input1: &ItemStack, input2: &ItemStack) -> i32 {
    let mut total = 0;
    total += count_enchantment_levels(&input1.nbt_data);
    total += count_enchantment_levels(&input2.nbt_data);
    total
}

/// Check if NBT data contains an `ench` tag.
fn has_enchantments(nbt_data: &[u8]) -> bool {
    // Parse the NBT to look for the ench list
    match mc_rs_nbt::read_nbt_le(&mut &nbt_data[..]) {
        Ok(root) => root.compound.contains_key("ench"),
        Err(_) => false,
    }
}

/// Count total enchantment levels in the item's NBT data.
fn count_enchantment_levels(nbt_data: &[u8]) -> i32 {
    if nbt_data.is_empty() {
        return 0;
    }
    let root = match mc_rs_nbt::read_nbt_le(&mut &nbt_data[..]) {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let ench_list = match root.compound.get("ench") {
        Some(mc_rs_nbt::tag::NbtTag::List(list)) => list,
        _ => return 0,
    };
    let mut total = 0i32;
    for tag in ench_list {
        if let mc_rs_nbt::tag::NbtTag::Compound(c) = tag {
            if let Some(mc_rs_nbt::tag::NbtTag::Short(lvl)) = c.get("lvl") {
                total += *lvl as i32;
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_rs_nbt::tag::{NbtCompound, NbtRoot, NbtTag};

    fn make_enchanted_item(ench_levels: &[(i16, i16)]) -> ItemStack {
        let mut ench_list = Vec::new();
        for &(id, lvl) in ench_levels {
            let mut c = NbtCompound::new();
            c.insert("id".to_string(), NbtTag::Short(id));
            c.insert("lvl".to_string(), NbtTag::Short(lvl));
            ench_list.push(NbtTag::Compound(c));
        }
        let mut root_c = NbtCompound::new();
        root_c.insert("ench".to_string(), NbtTag::List(ench_list));
        let root = NbtRoot::new("", root_c);
        let mut nbt_data = Vec::new();
        mc_rs_nbt::write_nbt_le(&mut nbt_data, &root);

        ItemStack {
            runtime_id: 42,
            count: 1,
            metadata: 0,
            block_runtime_id: 0,
            nbt_data,
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
            stack_network_id: 0,
        }
    }

    #[test]
    fn output_strips_enchantments() {
        let item = make_enchanted_item(&[(9, 3)]); // Sharpness III
        let output = compute_grindstone_output(&item, &ItemStack::empty()).unwrap();
        assert_eq!(output.runtime_id, 42);
        assert!(output.nbt_data.is_empty());
    }

    #[test]
    fn empty_inputs_no_output() {
        assert!(compute_grindstone_output(&ItemStack::empty(), &ItemStack::empty()).is_none());
    }

    #[test]
    fn no_enchantments_no_output() {
        let item = ItemStack {
            runtime_id: 10,
            count: 1,
            metadata: 0,
            block_runtime_id: 0,
            nbt_data: Vec::new(),
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
            stack_network_id: 0,
        };
        assert!(compute_grindstone_output(&item, &ItemStack::empty()).is_none());
    }

    #[test]
    fn xp_reward_counts_levels() {
        let item = make_enchanted_item(&[(9, 3), (17, 2)]); // Sharpness III + Smite II = 5
        let xp = grindstone_xp_reward(&item, &ItemStack::empty());
        assert_eq!(xp, 5);
    }

    #[test]
    fn xp_reward_both_slots() {
        let item1 = make_enchanted_item(&[(9, 2)]); // 2
        let item2 = make_enchanted_item(&[(0, 4)]); // 4
        assert_eq!(grindstone_xp_reward(&item1, &item2), 6);
    }
}
