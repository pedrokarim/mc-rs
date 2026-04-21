use crate::item_registry;
use crate::world::block_registry::BLOCKS;
use mc_rs_proto::packets::player::{ItemStack, ItemStackWrapper};

fn required_item_id(name: &str) -> i32 {
    item_registry::required_item_id(name)
}

/// Block runtime ID → (item network ID, item block runtime ID) mapping.
///
/// Generic : on récupère le nom du block via `BLOCKS.name_for`, puis on
/// demande l'item_id correspondant via `required_item_id`. Couvre donc
/// automatiquement tous les blocs du registry qui ont un item avec le
/// MÊME nom (ce qui est le cas de la plupart : stone/dirt/oak_log/etc.).
pub fn block_to_item(block_runtime_id: u32) -> Option<(i32, i32)> {
    let name = BLOCKS.name_for(block_runtime_id)?;
    let item_id = item_registry::network_id(name).unwrap_or(0);
    if item_id == 0 {
        return None;
    }
    Some((item_id, block_runtime_id as i32))
}

/// Get the drop item for a broken block.
///
/// Règles :
/// - `bedrock` → None (incassable, ne devrait jamais être appelé)
/// - `stone` → cobblestone
/// - `grass_block` → dirt
/// - `coal_ore` → coal, `diamond_ore` → diamond, `lapis_ore` → lapis, `redstone_ore` → redstone (PMMP-like)
/// - `short_grass`/`tall_grass`/`fern`/`large_fern` → None sans shears (hand drop rien)
/// - `oak_leaves`/autres leaves → None sans shears (PMMP : drop rare sapling/apple avec proba)
/// - fleurs (dandelion, poppy, blue_orchid, etc.) → **se droppent elles-mêmes**
/// - Tous les autres blocs → se droppent eux-mêmes (logs, planks, sand, etc.)
pub fn block_drop(block_runtime_id: u32) -> Option<ItemStack> {
    let b = &*BLOCKS;

    // 1. Blocs incassables / spéciaux — pas de drop.
    if block_runtime_id == b.bedrock {
        return None;
    }

    // 2. Remplacements — bloc → autre item.
    if block_runtime_id == b.stone {
        return Some(ItemStack::new(
            required_item_id("minecraft:cobblestone"),
            1,
            b.cobblestone as i32,
        ));
    }
    if block_runtime_id == b.grass_block {
        return Some(ItemStack::new(
            required_item_id("minecraft:dirt"),
            1,
            b.dirt as i32,
        ));
    }
    if block_runtime_id == b.coal_ore {
        return Some(ItemStack::new(required_item_id("minecraft:coal"), 1, 0));
    }
    if block_runtime_id == b.diamond_ore {
        return Some(ItemStack::new(required_item_id("minecraft:diamond"), 1, 0));
    }
    if block_runtime_id == b.lapis_ore {
        // 4..8 lapis en vanilla ; on met 4 constant (simplification).
        return Some(ItemStack::new(required_item_id("minecraft:lapis_lazuli"), 4, 0));
    }
    if block_runtime_id == b.redstone_ore {
        return Some(ItemStack::new(required_item_id("minecraft:redstone"), 4, 0));
    }

    // 3. Feuilles / herbes / plantes fragiles : rien à la main (hand-break).
    // Pour être plus fidèle à PMMP, il faudrait vérifier l'outil (shears,
    // épée) et retourner selon l'outil. Pour l'instant simplification : hand
    // → rien sur ces blocs.
    let leaves = [
        b.oak_leaves,
        b.birch_leaves,
        b.spruce_leaves,
        b.acacia_leaves,
        b.dark_oak_leaves,
        b.jungle_leaves,
    ];
    if leaves.contains(&block_runtime_id) {
        return None;
    }
    // short_grass / tall_grass / fern / large_fern : 1/8 chance de wheat_seeds
    // à la main (PMMP TallGrassTrait::getDropsForIncompatibleTool +
    // FortuneDropHelper::bonusChanceDivisor(8, 2)).
    if block_runtime_id == b.short_grass
        || block_runtime_id == b.tall_grass
        || block_runtime_id == b.fern
        || block_runtime_id == b.large_fern
    {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        if rng.gen_range(0..8) == 0 {
            return Some(ItemStack::new(
                required_item_id("minecraft:wheat_seeds"),
                1,
                0,
            ));
        }
        return None;
    }
    // deadbush : 0..2 sticks à la main (PMMP `DeadBush::getDropsForIncompatibleTool`).
    if block_runtime_id == b.deadbush {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let n = rng.gen_range(0..=2);
        if n == 0 {
            return None;
        }
        return Some(ItemStack::new(required_item_id("minecraft:stick"), n, 0));
    }
    // seagrass : rien à la main.
    if block_runtime_id == b.seagrass {
        return None;
    }

    // 4. Défaut : le bloc se drop lui-même via block_to_item.
    block_to_item(block_runtime_id).map(|(item_id, brid)| ItemStack::new(item_id, 1, brid))
}

/// Item network ID → block runtime ID mapping for placement.
///
/// Générique : pour un item_id, on récupère son nom via item_registry, puis on
/// lookup le block correspondant dans BLOCKS. Couvre tous les items placables
/// dont le nom item == nom block (cas vanilla de la plupart des items).
pub fn item_to_block(item_id: i32) -> Option<u32> {
    let item_name = item_registry::item_name_by_id(item_id)?;
    let block_id = BLOCKS.get(item_name);
    // BLOCKS.get retourne air (0) si non trouvé ; on exclut ce cas sauf si
    // item_name == "minecraft:air" (hypothèse invraisemblable pour un place).
    if block_id == BLOCKS.air && item_name != "minecraft:air" {
        return None;
    }
    Some(block_id)
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
    pub cursor: ItemStackWrapper,          // 1 slot (UI slot 0)
    pub craft_grid: [ItemStackWrapper; 4], // 2x2 (UI slots 28..32)
    pub craft_result: ItemStackWrapper,    // (UI slot 50)
    pub held_slot: u8,                     // 0-8 hotbar index
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
    pub fn slot_mut(
        &mut self,
        key: crate::inventory_manager::InvKey,
        core_slot: usize,
    ) -> Option<&mut ItemStackWrapper> {
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

    pub fn slot_ref(
        &self,
        key: crate::inventory_manager::InvKey,
        core_slot: usize,
    ) -> Option<&ItemStackWrapper> {
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
        // Max stack dépend du type d'item (sword=1, ender_pearl=16, stone=64).
        let max_stack = crate::item_registry::item_name_by_id(item.id as i32)
            .map(crate::stack_sizes::max_stack_size)
            .unwrap_or(64);
        // First try to stack with existing items
        for i in 0..36 {
            let slot = &self.slots[i];
            if slot.item.id == item.id
                && slot.item.meta == item.meta
                && slot.item.count < max_stack
                && !slot.item.is_air()
            {
                let space = max_stack - self.slots[i].item.count;
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
