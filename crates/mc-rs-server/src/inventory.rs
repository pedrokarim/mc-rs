use mc_rs_proto::packets::player::{ItemStack, ItemStackWrapper};

/// Block runtime ID → (item network ID, item block runtime ID) mapping.
/// Item network IDs from PMMP required_item_list.json (protocol 924).
/// Block runtime IDs from canonical_block_states.nbt sequential indices.
pub fn block_to_item(block_runtime_id: u32) -> Option<(i32, i32)> {
    // Returns (item_network_id, item_block_runtime_id)
    // For block items, the item_block_runtime_id is the block's runtime ID
    let item_id: i32 = match block_runtime_id {
        9852 => 3,    // dirt
        11062 => 2,   // grass_block → drops dirt item (id=3) but as grass_block
        13079 => 7,   // bedrock
        2532 => 1,    // stone → drops cobblestone
        6234 => 12,   // sand
        5213 => 24,   // sandstone
        15806 => 13,  // gravel
        1366 => 17,   // oak_log
        2752 => 18,   // oak_leaves
        1019 => 78,   // snow_layer
        6318 => 16,   // coal_ore
        7336 => 15,   // iron_ore
        3203 => 14,   // gold_ore
        6501 => 56,   // diamond_ore
        6356 => 73,   // redstone_ore
        14583 => 21,  // lapis_ore
        5240 => 110,  // mycelium
        2732 => 12,   // red_sand → sand item
        2086 => 172,  // hardened_clay
        6233 => 80,   // snow_block
        7292 => 3,    // podzol → dirt item (simplified)
        6725 => 3,    // coarse_dirt → dirt item
        12454 => 179, // red_sandstone
        1310 => -378, // deepslate
        1763 => -333, // tuff
        284 => -590,  // granite
        415 => -592,  // diorite
        2530 => -594, // andesite
        _ => return None,
    };
    Some((item_id, block_runtime_id as i32))
}

/// Get the drop item for a broken block.
/// Some blocks drop different items (stone → cobblestone, grass → dirt).
pub fn block_drop(block_runtime_id: u32) -> Option<ItemStack> {
    match block_runtime_id {
        // Stone drops cobblestone
        2532 => Some(ItemStack::new(4, 1, 14254)), // cobblestone runtime_id
        // Grass block drops dirt
        11062 => Some(ItemStack::new(3, 1, 9852)),
        // Leaves drop nothing (simplified)
        2752 => None,
        // Short grass drops nothing
        12421 => None,
        // Bedrock is unbreakable in survival
        13079 => None,
        _ => {
            // Default: block drops itself
            block_to_item(block_runtime_id).map(|(item_id, brid)| ItemStack::new(item_id, 1, brid))
        }
    }
}

/// Item network ID → block runtime ID mapping for placement.
pub fn item_to_block(item_id: i32) -> Option<u32> {
    match item_id {
        3 => Some(9852),    // dirt
        2 => Some(11062),   // grass_block
        7 => Some(13079),   // bedrock
        1 => Some(2532),    // stone
        4 => Some(14254),   // cobblestone
        12 => Some(6234),   // sand
        24 => Some(5213),   // sandstone
        13 => Some(15806),  // gravel
        17 => Some(1366),   // oak_log
        18 => Some(2752),   // oak_leaves
        16 => Some(6318),   // coal_ore
        15 => Some(7336),   // iron_ore
        14 => Some(3203),   // gold_ore
        56 => Some(6501),   // diamond_ore
        73 => Some(6356),   // redstone_ore
        21 => Some(14583),  // lapis_ore
        80 => Some(6233),   // snow_block
        172 => Some(2086),  // hardened_clay
        -378 => Some(1310), // deepslate
        -333 => Some(1763), // tuff
        -590 => Some(284),  // granite
        -592 => Some(415),  // diorite
        -594 => Some(2530), // andesite
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
