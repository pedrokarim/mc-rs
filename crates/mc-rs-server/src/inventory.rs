use crate::world::block_registry::BLOCKS;
use mc_rs_proto::packets::player::{ItemStack, ItemStackWrapper};

/// Block runtime ID → (item network ID, item block runtime ID) mapping.
/// Item network IDs from PMMP required_item_list.json.
/// Block runtime IDs resolved dynamically from block registry.
pub fn block_to_item(block_runtime_id: u32) -> Option<(i32, i32)> {
    let b = &*BLOCKS;
    let item_id: i32 = if block_runtime_id == b.dirt {
        3
    } else if block_runtime_id == b.grass_block {
        2
    } else if block_runtime_id == b.bedrock {
        7
    } else if block_runtime_id == b.stone {
        1
    } else if block_runtime_id == b.sand {
        12
    } else if block_runtime_id == b.sandstone {
        24
    } else if block_runtime_id == b.gravel {
        13
    } else if block_runtime_id == b.oak_log {
        17
    } else if block_runtime_id == b.oak_leaves {
        18
    } else if block_runtime_id == b.snow_layer {
        78
    } else if block_runtime_id == b.coal_ore {
        16
    } else if block_runtime_id == b.iron_ore {
        15
    } else if block_runtime_id == b.gold_ore {
        14
    } else if block_runtime_id == b.diamond_ore {
        56
    } else if block_runtime_id == b.redstone_ore {
        73
    } else if block_runtime_id == b.lapis_ore {
        21
    } else if block_runtime_id == b.mycelium {
        110
    } else if block_runtime_id == b.red_sand {
        12
    } else if block_runtime_id == b.hardened_clay {
        172
    } else if block_runtime_id == b.snow_block {
        80
    } else if block_runtime_id == b.podzol {
        3
    } else if block_runtime_id == b.coarse_dirt {
        3
    } else if block_runtime_id == b.red_sandstone {
        179
    } else if block_runtime_id == b.deepslate {
        -378
    } else if block_runtime_id == b.tuff {
        -333
    } else if block_runtime_id == b.granite {
        -590
    } else if block_runtime_id == b.diorite {
        -592
    } else if block_runtime_id == b.andesite {
        -594
    } else {
        return None;
    };
    Some((item_id, block_runtime_id as i32))
}

/// Get the drop item for a broken block.
/// Some blocks drop different items (stone → cobblestone, grass → dirt).
pub fn block_drop(block_runtime_id: u32) -> Option<ItemStack> {
    let b = &*BLOCKS;
    if block_runtime_id == b.stone {
        Some(ItemStack::new(4, 1, b.cobblestone as i32))
    } else if block_runtime_id == b.grass_block {
        Some(ItemStack::new(3, 1, b.dirt as i32))
    } else if block_runtime_id == b.oak_leaves {
        None
    } else if block_runtime_id == b.short_grass {
        None
    } else if block_runtime_id == b.bedrock {
        None
    } else {
        block_to_item(block_runtime_id).map(|(item_id, brid)| ItemStack::new(item_id, 1, brid))
    }
}

/// Item network ID → block runtime ID mapping for placement.
pub fn item_to_block(item_id: i32) -> Option<u32> {
    let b = &*BLOCKS;
    match item_id {
        3 => Some(b.dirt),
        2 => Some(b.grass_block),
        7 => Some(b.bedrock),
        1 => Some(b.stone),
        4 => Some(b.cobblestone),
        12 => Some(b.sand),
        24 => Some(b.sandstone),
        13 => Some(b.gravel),
        17 => Some(b.oak_log),
        18 => Some(b.oak_leaves),
        16 => Some(b.coal_ore),
        15 => Some(b.iron_ore),
        14 => Some(b.gold_ore),
        56 => Some(b.diamond_ore),
        73 => Some(b.redstone_ore),
        21 => Some(b.lapis_ore),
        80 => Some(b.snow_block),
        172 => Some(b.hardened_clay),
        -378 => Some(b.deepslate),
        -333 => Some(b.tuff),
        -590 => Some(b.granite),
        -592 => Some(b.diorite),
        -594 => Some(b.andesite),
        _ => None,
    }
}

/// Player inventory: 36 main slots + 4 armor + 1 offhand.
pub struct PlayerInventory {
    pub slots: Vec<ItemStackWrapper>, // 36 slots (hotbar 0-8, main 9-35)
    pub armor: Vec<ItemStackWrapper>, // 4 slots
    pub offhand: ItemStackWrapper,
    pub held_slot: u8, // 0-8 hotbar index
    next_stack_id: i32,
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerInventory {
    pub fn new() -> Self {
        Self {
            slots: vec![ItemStackWrapper::air(); 36],
            armor: vec![ItemStackWrapper::air(); 4],
            offhand: ItemStackWrapper::air(),
            held_slot: 0,
            next_stack_id: 1,
        }
    }

    /// Get the next unique stack ID.
    pub fn next_stack_id(&mut self) -> i32 {
        let id = self.next_stack_id;
        self.next_stack_id += 1;
        id
    }

    /// Get the item in the currently held hotbar slot.
    pub fn held_item(&self) -> &ItemStackWrapper {
        &self.slots[self.held_slot as usize]
    }

    /// Add an item to the first available slot.
    /// Returns the slot index if successful, None if inventory is full.
    pub fn add_item(&mut self, item: ItemStack) -> Option<usize> {
        // First try to stack with existing items
        for i in 0..36 {
            let slot = &self.slots[i];
            if slot.item.id == item.id
                && slot.item.meta == item.meta
                && slot.item.count < 64
                && !slot.item.is_air()
            {
                let space = 64 - self.slots[i].item.count;
                let add = item.count.min(space);
                self.slots[i].item.count += add;
                return Some(i);
            }
        }

        // Then find empty slot
        for i in 0..36 {
            if self.slots[i].item.is_air() {
                let stack_id = self.next_stack_id();
                self.slots[i] = ItemStackWrapper::new(item, stack_id);
                return Some(i);
            }
        }

        None // Inventory full
    }
}
