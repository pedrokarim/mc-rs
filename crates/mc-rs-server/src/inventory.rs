use crate::item_registry;
use crate::world::block_registry::BLOCKS;
use mc_rs_proto::packets::player::{ItemStack, ItemStackWrapper};

fn required_item_id(name: &str) -> i32 {
    item_registry::required_item_id(name)
}

/// Block runtime ID → (item network ID, item block runtime ID) mapping.
/// Item network IDs come from Bedrock's required_item_list.json for protocol 944.
/// Block runtime IDs resolved dynamically from block registry.
pub fn block_to_item(block_runtime_id: u32) -> Option<(i32, i32)> {
    let b = &*BLOCKS;
    let item_id: i32 = if block_runtime_id == b.dirt {
        required_item_id("minecraft:dirt")
    } else if block_runtime_id == b.grass_block {
        required_item_id("minecraft:grass_block")
    } else if block_runtime_id == b.bedrock {
        required_item_id("minecraft:bedrock")
    } else if block_runtime_id == b.stone {
        required_item_id("minecraft:stone")
    } else if block_runtime_id == b.sand {
        required_item_id("minecraft:sand")
    } else if block_runtime_id == b.sandstone {
        required_item_id("minecraft:sandstone")
    } else if block_runtime_id == b.gravel {
        required_item_id("minecraft:gravel")
    } else if block_runtime_id == b.oak_log {
        required_item_id("minecraft:oak_log")
    } else if block_runtime_id == b.oak_leaves {
        required_item_id("minecraft:oak_leaves")
    } else if block_runtime_id == b.snow_layer {
        required_item_id("minecraft:snow_layer")
    } else if block_runtime_id == b.coal_ore {
        required_item_id("minecraft:coal_ore")
    } else if block_runtime_id == b.iron_ore {
        required_item_id("minecraft:iron_ore")
    } else if block_runtime_id == b.gold_ore {
        required_item_id("minecraft:gold_ore")
    } else if block_runtime_id == b.diamond_ore {
        required_item_id("minecraft:diamond_ore")
    } else if block_runtime_id == b.redstone_ore {
        required_item_id("minecraft:redstone_ore")
    } else if block_runtime_id == b.lapis_ore {
        required_item_id("minecraft:lapis_ore")
    } else if block_runtime_id == b.mycelium {
        required_item_id("minecraft:mycelium")
    } else if block_runtime_id == b.red_sand {
        required_item_id("minecraft:red_sand")
    } else if block_runtime_id == b.hardened_clay {
        required_item_id("minecraft:hardened_clay")
    } else if block_runtime_id == b.snow_block {
        required_item_id("minecraft:snow")
    } else if block_runtime_id == b.podzol {
        required_item_id("minecraft:podzol")
    } else if block_runtime_id == b.coarse_dirt {
        required_item_id("minecraft:coarse_dirt")
    } else if block_runtime_id == b.red_sandstone {
        required_item_id("minecraft:red_sandstone")
    } else if block_runtime_id == b.deepslate {
        required_item_id("minecraft:deepslate")
    } else if block_runtime_id == b.tuff {
        required_item_id("minecraft:tuff")
    } else if block_runtime_id == b.granite {
        required_item_id("minecraft:granite")
    } else if block_runtime_id == b.diorite {
        required_item_id("minecraft:diorite")
    } else if block_runtime_id == b.andesite {
        required_item_id("minecraft:andesite")
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
        Some(ItemStack::new(
            required_item_id("minecraft:cobblestone"),
            1,
            b.cobblestone as i32,
        ))
    } else if block_runtime_id == b.grass_block {
        Some(ItemStack::new(
            required_item_id("minecraft:dirt"),
            1,
            b.dirt as i32,
        ))
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
        id if id == required_item_id("minecraft:dirt") => Some(b.dirt),
        id if id == required_item_id("minecraft:grass_block") => Some(b.grass_block),
        id if id == required_item_id("minecraft:bedrock") => Some(b.bedrock),
        id if id == required_item_id("minecraft:stone") => Some(b.stone),
        id if id == required_item_id("minecraft:cobblestone") => Some(b.cobblestone),
        id if id == required_item_id("minecraft:sand") => Some(b.sand),
        id if id == required_item_id("minecraft:sandstone") => Some(b.sandstone),
        id if id == required_item_id("minecraft:gravel") => Some(b.gravel),
        id if id == required_item_id("minecraft:oak_log") => Some(b.oak_log),
        id if id == required_item_id("minecraft:oak_leaves") => Some(b.oak_leaves),
        id if id == required_item_id("minecraft:coal_ore") => Some(b.coal_ore),
        id if id == required_item_id("minecraft:iron_ore") => Some(b.iron_ore),
        id if id == required_item_id("minecraft:gold_ore") => Some(b.gold_ore),
        id if id == required_item_id("minecraft:diamond_ore") => Some(b.diamond_ore),
        id if id == required_item_id("minecraft:redstone_ore") => Some(b.redstone_ore),
        id if id == required_item_id("minecraft:lapis_ore") => Some(b.lapis_ore),
        id if id == required_item_id("minecraft:mycelium") => Some(b.mycelium),
        id if id == required_item_id("minecraft:red_sand") => Some(b.red_sand),
        id if id == required_item_id("minecraft:podzol") => Some(b.podzol),
        id if id == required_item_id("minecraft:coarse_dirt") => Some(b.coarse_dirt),
        id if id == required_item_id("minecraft:red_sandstone") => Some(b.red_sandstone),
        id if id == required_item_id("minecraft:snow_layer") => Some(b.snow_layer),
        id if id == required_item_id("minecraft:snow") => Some(b.snow_block),
        id if id == required_item_id("minecraft:hardened_clay") => Some(b.hardened_clay),
        id if id == required_item_id("minecraft:deepslate") => Some(b.deepslate),
        id if id == required_item_id("minecraft:tuff") => Some(b.tuff),
        id if id == required_item_id("minecraft:granite") => Some(b.granite),
        id if id == required_item_id("minecraft:diorite") => Some(b.diorite),
        id if id == required_item_id("minecraft:andesite") => Some(b.andesite),
        _ => None,
    }
}

/// Player inventory: 36 main slots + 4 armor + 1 offhand + cursor + 2x2 craft grid.
///
/// Mirrors PMMP split across PlayerInventory, ArmorInventory, PlayerOffHandInventory,
/// PlayerCursorInventory and PlayerCraftingInventory. Bundled here because Rust lacks
/// `spl_object_id` — InventoryManager identifies them via `InvKey` instead.
pub struct PlayerInventory {
    pub slots: Vec<ItemStackWrapper>, // 36 slots (hotbar 0-8, main 9-35)
    pub armor: Vec<ItemStackWrapper>, // 4 slots
    pub offhand: ItemStackWrapper,
    pub cursor: ItemStackWrapper,           // 1 slot (UI slot 0)
    pub craft_grid: [ItemStackWrapper; 4],  // 2x2 (UI slots 28..32)
    pub craft_result: ItemStackWrapper,     // (UI slot 50)
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
            cursor: ItemStackWrapper::air(),
            craft_grid: [
                ItemStackWrapper::air(),
                ItemStackWrapper::air(),
                ItemStackWrapper::air(),
                ItemStackWrapper::air(),
            ],
            craft_result: ItemStackWrapper::air(),
            held_slot: 0,
            next_stack_id: 1,
        }
    }

    pub fn from_parts(
        mut slots: Vec<ItemStackWrapper>,
        mut armor: Vec<ItemStackWrapper>,
        offhand: ItemStackWrapper,
        held_slot: u8,
    ) -> Self {
        slots.resize(36, ItemStackWrapper::air());
        armor.resize(4, ItemStackWrapper::air());

        let max_stack_id = slots
            .iter()
            .chain(armor.iter())
            .chain(std::iter::once(&offhand))
            .map(|slot| slot.stack_id)
            .max()
            .unwrap_or(0);

        Self {
            slots,
            armor,
            offhand,
            cursor: ItemStackWrapper::air(),
            craft_grid: [
                ItemStackWrapper::air(),
                ItemStackWrapper::air(),
                ItemStackWrapper::air(),
                ItemStackWrapper::air(),
            ],
            craft_result: ItemStackWrapper::air(),
            held_slot: held_slot.min(8),
            next_stack_id: max_stack_id.max(0) + 1,
        }
    }

    /// Get the next unique stack ID.
    pub fn next_stack_id(&mut self) -> i32 {
        let id = self.next_stack_id;
        self.next_stack_id += 1;
        id
    }

    /// Get a mutable reference to a slot inside the logical inventory identified by `key`.
    /// Returns None if the slot is out of range for that inventory.
    pub fn slot_mut(&mut self, key: crate::inventory_manager::InvKey, core_slot: usize) -> Option<&mut ItemStackWrapper> {
        use crate::inventory_manager::InvKey;
        match key {
            InvKey::Main => self.slots.get_mut(core_slot),
            InvKey::Offhand => (core_slot == 0).then_some(&mut self.offhand),
            InvKey::Armor => self.armor.get_mut(core_slot),
            InvKey::Cursor => (core_slot == 0).then_some(&mut self.cursor),
            InvKey::Craft2x2 => self.craft_grid.get_mut(core_slot),
            InvKey::CraftResult => (core_slot == 0).then_some(&mut self.craft_result),
        }
    }

    pub fn slot_ref(&self, key: crate::inventory_manager::InvKey, core_slot: usize) -> Option<&ItemStackWrapper> {
        use crate::inventory_manager::InvKey;
        match key {
            InvKey::Main => self.slots.get(core_slot),
            InvKey::Offhand => (core_slot == 0).then_some(&self.offhand),
            InvKey::Armor => self.armor.get(core_slot),
            InvKey::Cursor => (core_slot == 0).then_some(&self.cursor),
            InvKey::Craft2x2 => self.craft_grid.get(core_slot),
            InvKey::CraftResult => (core_slot == 0).then_some(&self.craft_result),
        }
    }

    pub fn inventory_size(key: crate::inventory_manager::InvKey) -> usize {
        use crate::inventory_manager::InvKey;
        match key {
            InvKey::Main => 36,
            InvKey::Offhand => 1,
            InvKey::Armor => 4,
            InvKey::Cursor => 1,
            InvKey::Craft2x2 => 4,
            InvKey::CraftResult => 1,
        }
    }

    /// Get the item in the currently held hotbar slot.
    pub fn held_item(&self) -> &ItemStackWrapper {
        &self.slots[self.held_slot as usize]
    }

    pub fn clear(&mut self) {
        self.slots.fill(ItemStackWrapper::air());
        self.armor.fill(ItemStackWrapper::air());
        self.offhand = ItemStackWrapper::air();
        self.cursor = ItemStackWrapper::air();
        for s in &mut self.craft_grid {
            *s = ItemStackWrapper::air();
        }
        self.craft_result = ItemStackWrapper::air();
        self.held_slot = 0;
        self.next_stack_id = 1;
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
