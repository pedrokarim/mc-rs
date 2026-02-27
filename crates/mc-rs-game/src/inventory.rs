//! Player inventory management.
//!
//! Manages the 36-slot main inventory, 4 armor slots, offhand, and cursor.
//! Processes ItemStackRequest actions and generates responses.

use std::sync::atomic::{AtomicI32, Ordering};

use tracing::debug;

use mc_rs_proto::item_stack::ItemStack;
use mc_rs_proto::packets::item_stack_request::{StackAction, StackRequest, StackSlot};
use mc_rs_proto::packets::item_stack_response::{
    StackResponseContainer, StackResponseEntry, StackResponseSlot,
};
use mc_rs_world::item_registry::ItemRegistry;

use crate::recipe::RecipeRegistry;

/// Bedrock container IDs.
pub const CONTAINER_INVENTORY: u8 = 0;
pub const CONTAINER_ARMOR: u8 = 119;
pub const CONTAINER_CREATIVE_OUTPUT: u8 = 120;
pub const CONTAINER_OFFHAND: u8 = 124;
pub const CONTAINER_CURSOR: u8 = 58;
pub const CONTAINER_CREATIVE: u8 = 59;
pub const CONTAINER_CRAFTING_INPUT: u8 = 28;
pub const CONTAINER_CRAFTING_OUTPUT: u8 = 29;

/// Player inventory with all container slots.
pub struct PlayerInventory {
    /// Main inventory: 36 slots (0-35). Slots 0-8 = hotbar.
    pub main: Vec<ItemStack>,
    /// Armor: 4 slots (helmet, chestplate, leggings, boots).
    pub armor: Vec<ItemStack>,
    /// Offhand: 1 slot.
    pub offhand: ItemStack,
    /// Cursor: item being dragged by the player.
    pub cursor: ItemStack,
    /// Crafting grid: 9 slots (3×3, used for crafting table; first 4 for 2×2).
    pub crafting_grid: Vec<ItemStack>,
    /// Crafting output: 1 slot for the result.
    pub crafting_output: ItemStack,
    /// Currently selected hotbar slot (0-8).
    pub held_slot: u8,
    /// Global counter for assigning unique stack network IDs.
    next_stack_id: AtomicI32,
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerInventory {
    /// Create an empty inventory.
    pub fn new() -> Self {
        Self {
            main: (0..36).map(|_| ItemStack::empty()).collect(),
            armor: (0..4).map(|_| ItemStack::empty()).collect(),
            offhand: ItemStack::empty(),
            cursor: ItemStack::empty(),
            crafting_grid: (0..9).map(|_| ItemStack::empty()).collect(),
            crafting_output: ItemStack::empty(),
            held_slot: 0,
            next_stack_id: AtomicI32::new(1),
        }
    }

    /// Get a reference to the item in a specific slot.
    pub fn get_slot(&self, container_id: u8, slot: u8) -> Option<&ItemStack> {
        match container_id {
            CONTAINER_INVENTORY => self.main.get(slot as usize),
            CONTAINER_ARMOR => self.armor.get(slot as usize),
            CONTAINER_OFFHAND => Some(&self.offhand),
            CONTAINER_CURSOR => Some(&self.cursor),
            CONTAINER_CRAFTING_INPUT => self.crafting_grid.get(slot as usize),
            CONTAINER_CRAFTING_OUTPUT => Some(&self.crafting_output),
            _ => None,
        }
    }

    /// Get a mutable reference to the item in a specific slot.
    pub fn get_slot_mut(&mut self, container_id: u8, slot: u8) -> Option<&mut ItemStack> {
        match container_id {
            CONTAINER_INVENTORY => self.main.get_mut(slot as usize),
            CONTAINER_ARMOR => self.armor.get_mut(slot as usize),
            CONTAINER_OFFHAND => Some(&mut self.offhand),
            CONTAINER_CURSOR => Some(&mut self.cursor),
            CONTAINER_CRAFTING_INPUT => self.crafting_grid.get_mut(slot as usize),
            CONTAINER_CRAFTING_OUTPUT => Some(&mut self.crafting_output),
            _ => None,
        }
    }

    /// Set the item in a specific slot.
    pub fn set_slot(&mut self, container_id: u8, slot: u8, item: ItemStack) {
        match container_id {
            CONTAINER_INVENTORY => {
                if let Some(s) = self.main.get_mut(slot as usize) {
                    *s = item;
                }
            }
            CONTAINER_ARMOR => {
                if let Some(s) = self.armor.get_mut(slot as usize) {
                    *s = item;
                }
            }
            CONTAINER_OFFHAND => self.offhand = item,
            CONTAINER_CURSOR => self.cursor = item,
            CONTAINER_CRAFTING_INPUT => {
                if let Some(s) = self.crafting_grid.get_mut(slot as usize) {
                    *s = item;
                }
            }
            CONTAINER_CRAFTING_OUTPUT => self.crafting_output = item,
            _ => {}
        }
    }

    /// Get the currently held item (hotbar slot).
    pub fn held_item(&self) -> &ItemStack {
        self.main
            .get(self.held_slot as usize)
            .unwrap_or(&self.main[0])
    }

    /// Get a mutable reference to the currently held item.
    pub fn held_item_mut(&mut self) -> &mut ItemStack {
        let slot = self.held_slot as usize;
        &mut self.main[slot]
    }

    /// Allocate a unique stack network ID.
    pub fn next_stack_network_id(&self) -> i32 {
        self.next_stack_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Assign a stack network ID to an item if it doesn't have one.
    pub fn assign_stack_id(&self, item: &mut ItemStack) {
        if !item.is_empty() && item.stack_network_id == 0 {
            item.stack_network_id = self.next_stack_network_id();
        }
    }

    /// Process a single ItemStackRequest and return the response.
    pub fn process_request(
        &mut self,
        request: &StackRequest,
        item_registry: &ItemRegistry,
        recipe_registry: &RecipeRegistry,
    ) -> StackResponseEntry {
        let mut changed_containers: std::collections::HashMap<u8, Vec<StackResponseSlot>> =
            std::collections::HashMap::new();

        for action in &request.actions {
            match action {
                StackAction::Take { count, src, dst } | StackAction::Place { count, src, dst } => {
                    if !self.process_take_place(*count, src, dst, item_registry) {
                        return error_response(request.request_id);
                    }
                    record_slot_change(&mut changed_containers, src, self);
                    record_slot_change(&mut changed_containers, dst, self);
                }
                StackAction::Swap { src, dst } => {
                    if !self.process_swap(src, dst) {
                        return error_response(request.request_id);
                    }
                    record_slot_change(&mut changed_containers, src, self);
                    record_slot_change(&mut changed_containers, dst, self);
                }
                StackAction::Drop { count, src, .. } => {
                    if !self.process_drop(*count, src) {
                        return error_response(request.request_id);
                    }
                    record_slot_change(&mut changed_containers, src, self);
                }
                StackAction::Destroy { count, src } | StackAction::Consume { count, src } => {
                    if !self.process_destroy(*count, src) {
                        return error_response(request.request_id);
                    }
                    record_slot_change(&mut changed_containers, src, self);
                }
                StackAction::CraftRecipe { recipe_network_id } => {
                    if !self.process_craft_recipe(
                        *recipe_network_id,
                        1,
                        recipe_registry,
                        item_registry,
                    ) {
                        return error_response(request.request_id);
                    }
                    // Record crafting output slot change
                    record_crafting_output_change(&mut changed_containers, self);
                }
                StackAction::CraftRecipeAuto {
                    recipe_network_id,
                    times_crafted,
                    ..
                } => {
                    let times = (*times_crafted).max(1);
                    if !self.process_craft_recipe(
                        *recipe_network_id,
                        times,
                        recipe_registry,
                        item_registry,
                    ) {
                        return error_response(request.request_id);
                    }
                    record_crafting_output_change(&mut changed_containers, self);
                }
                StackAction::CraftCreative {
                    creative_item_network_id,
                } => {
                    // Creative mode: create the item directly.
                    debug!("CraftCreative: network_id={}", creative_item_network_id);
                }
                StackAction::Create { result_slot } => {
                    debug!("Create: result_slot={}", result_slot);
                }
                StackAction::Unknown { action_type } => {
                    debug!("Unknown stack action type: {}", action_type);
                }
            }
        }

        // Build success response
        let containers: Vec<StackResponseContainer> = changed_containers
            .into_iter()
            .map(|(container_id, slots)| StackResponseContainer {
                container_id,
                slots,
            })
            .collect();

        StackResponseEntry {
            status: 0, // Success
            request_id: request.request_id,
            containers,
        }
    }

    /// Process a CraftRecipe / CraftRecipeAuto action.
    ///
    /// Looks up the recipe, places the output in `crafting_output`, and
    /// consumes ingredients from `crafting_grid`. The client sends
    /// subsequent Take/Place actions to move the result to inventory.
    fn process_craft_recipe(
        &mut self,
        recipe_network_id: u32,
        times: u8,
        recipe_registry: &RecipeRegistry,
        item_registry: &ItemRegistry,
    ) -> bool {
        use crate::recipe::RecipeRef;

        let recipe = match recipe_registry.get_by_network_id(recipe_network_id) {
            Some(r) => r,
            None => {
                debug!("Unknown recipe network_id: {}", recipe_network_id);
                return false;
            }
        };

        let output = &recipe.output()[0];
        let output_rid = item_registry
            .get_by_name(&output.item_name)
            .map(|e| e.numeric_id as i32)
            .unwrap_or(0);
        if output_rid == 0 {
            debug!("Recipe output item not found: {}", output.item_name);
            return false;
        }

        // Consume ingredients from crafting grid (simplified: trust the client placement)
        match &recipe {
            RecipeRef::Shaped(shaped) => {
                for (i, inp) in shaped.input.iter().enumerate() {
                    if inp.item_name.is_empty() {
                        continue;
                    }
                    if let Some(grid_item) = self.crafting_grid.get_mut(i) {
                        let consume = (inp.count as u16) * (times as u16);
                        if grid_item.count <= consume {
                            *grid_item = ItemStack::empty();
                        } else {
                            grid_item.count -= consume;
                        }
                    }
                }
            }
            RecipeRef::Shapeless(shapeless) => {
                // For shapeless, consume from any matching grid slots
                for inp in &shapeless.inputs {
                    let need_rid = item_registry
                        .get_by_name(&inp.item_name)
                        .map(|e| e.numeric_id as i32)
                        .unwrap_or(0);
                    let mut remaining = (inp.count as u16) * (times as u16);
                    for grid_item in &mut self.crafting_grid {
                        if remaining == 0 {
                            break;
                        }
                        if grid_item.runtime_id == need_rid && !grid_item.is_empty() {
                            let take = remaining.min(grid_item.count);
                            grid_item.count -= take;
                            if grid_item.count == 0 {
                                *grid_item = ItemStack::empty();
                            }
                            remaining -= take;
                        }
                    }
                }
            }
        }

        // Place result in crafting output
        let mut result = ItemStack::new(output_rid, output.count as u16 * times as u16);
        result.metadata = output.metadata;
        result.stack_network_id = self.next_stack_network_id();
        self.crafting_output = result;

        debug!(
            "Crafted recipe {} (×{}) → {} ×{}",
            recipe_network_id,
            times,
            output.item_name,
            output.count as u16 * times as u16
        );
        true
    }

    /// Clear the crafting grid (called when the player closes the crafting UI).
    pub fn clear_crafting_grid(&mut self) {
        for slot in &mut self.crafting_grid {
            *slot = ItemStack::empty();
        }
        self.crafting_output = ItemStack::empty();
    }

    /// Move `count` items from src to dst. Returns false on failure.
    fn process_take_place(
        &mut self,
        count: u8,
        src: &StackSlot,
        dst: &StackSlot,
        item_registry: &ItemRegistry,
    ) -> bool {
        // Special case: source is creative menu (container 59)
        if src.container_id == CONTAINER_CREATIVE {
            // In creative mode, items are created from nothing.
            // The dst slot gets a new item based on what the client requests.
            return true;
        }

        let src_item = match self.get_slot(src.container_id, src.slot) {
            Some(item) => item.clone(),
            None => return false,
        };

        if src_item.is_empty() {
            return false;
        }

        let actual_count = count.min(src_item.count as u8);

        let dst_item = match self.get_slot(dst.container_id, dst.slot) {
            Some(item) => item.clone(),
            None => return false,
        };

        if dst_item.is_empty() {
            // Place into empty slot
            let mut new_dst = src_item.clone();
            new_dst.count = actual_count as u16;
            new_dst.stack_network_id = self.next_stack_network_id();
            self.set_slot(dst.container_id, dst.slot, new_dst);

            // Update source
            let remaining = src_item.count - actual_count as u16;
            if remaining == 0 {
                self.set_slot(src.container_id, src.slot, ItemStack::empty());
            } else {
                let mut new_src = src_item;
                new_src.count = remaining;
                self.set_slot(src.container_id, src.slot, new_src);
            }
        } else if dst_item.runtime_id == src_item.runtime_id
            && dst_item.metadata == src_item.metadata
        {
            // Stack onto existing same item
            let max_stack = item_registry.max_stack_size(src_item.runtime_id as i16);
            let space = max_stack as u16 - dst_item.count;
            let to_move = (actual_count as u16).min(space);
            if to_move == 0 {
                return false;
            }

            let mut new_dst = dst_item;
            new_dst.count += to_move;
            self.set_slot(dst.container_id, dst.slot, new_dst);

            let remaining = src_item.count - to_move;
            if remaining == 0 {
                self.set_slot(src.container_id, src.slot, ItemStack::empty());
            } else {
                let mut new_src = src_item;
                new_src.count = remaining;
                self.set_slot(src.container_id, src.slot, new_src);
            }
        } else {
            // Different items — can't stack
            return false;
        }

        true
    }

    /// Swap items between two slots. Returns false on failure.
    fn process_swap(&mut self, src: &StackSlot, dst: &StackSlot) -> bool {
        let src_item = match self.get_slot(src.container_id, src.slot) {
            Some(item) => item.clone(),
            None => return false,
        };
        let dst_item = match self.get_slot(dst.container_id, dst.slot) {
            Some(item) => item.clone(),
            None => return false,
        };

        self.set_slot(src.container_id, src.slot, dst_item);
        self.set_slot(dst.container_id, dst.slot, src_item);
        true
    }

    /// Remove `count` items from src (drop). Returns false on failure.
    fn process_drop(&mut self, count: u8, src: &StackSlot) -> bool {
        let src_item = match self.get_slot(src.container_id, src.slot) {
            Some(item) => item.clone(),
            None => return false,
        };

        if src_item.is_empty() {
            return false;
        }

        let actual_count = count.min(src_item.count as u8);
        let remaining = src_item.count - actual_count as u16;

        if remaining == 0 {
            self.set_slot(src.container_id, src.slot, ItemStack::empty());
        } else {
            let mut new_src = src_item;
            new_src.count = remaining;
            self.set_slot(src.container_id, src.slot, new_src);
        }

        true
    }

    /// Remove `count` items from src (destroy/consume). Returns false on failure.
    fn process_destroy(&mut self, count: u8, src: &StackSlot) -> bool {
        self.process_drop(count, src) // Same logic
    }

    /// Process a single ItemStackRequest with an external container (e.g., chest).
    ///
    /// Slots with `container_id == ext_container_id` are routed to `ext_items`
    /// instead of the player's inventory.
    pub fn process_request_with_container(
        &mut self,
        request: &StackRequest,
        item_registry: &ItemRegistry,
        ext_container_id: u8,
        ext_items: &mut [ItemStack],
    ) -> StackResponseEntry {
        let mut changed_containers: std::collections::HashMap<u8, Vec<StackResponseSlot>> =
            std::collections::HashMap::new();

        for action in &request.actions {
            match action {
                StackAction::Take { count, src, dst } | StackAction::Place { count, src, dst } => {
                    let src_item = match get_item_ext(
                        self,
                        src.container_id,
                        src.slot,
                        ext_container_id,
                        ext_items,
                    ) {
                        Some(i) if !i.is_empty() => i,
                        _ => return error_response(request.request_id),
                    };
                    let dst_item = match get_item_ext(
                        self,
                        dst.container_id,
                        dst.slot,
                        ext_container_id,
                        ext_items,
                    ) {
                        Some(i) => i,
                        None => return error_response(request.request_id),
                    };
                    let actual_count = (*count).min(src_item.count as u8);

                    if dst_item.is_empty() {
                        let mut new_dst = src_item.clone();
                        new_dst.count = actual_count as u16;
                        new_dst.stack_network_id = self.next_stack_network_id();
                        set_item_ext(
                            self,
                            dst.container_id,
                            dst.slot,
                            new_dst,
                            ext_container_id,
                            ext_items,
                        );
                        let remaining = src_item.count - actual_count as u16;
                        if remaining == 0 {
                            set_item_ext(
                                self,
                                src.container_id,
                                src.slot,
                                ItemStack::empty(),
                                ext_container_id,
                                ext_items,
                            );
                        } else {
                            let mut new_src = src_item;
                            new_src.count = remaining;
                            set_item_ext(
                                self,
                                src.container_id,
                                src.slot,
                                new_src,
                                ext_container_id,
                                ext_items,
                            );
                        }
                    } else if dst_item.runtime_id == src_item.runtime_id
                        && dst_item.metadata == src_item.metadata
                    {
                        let max_stack = item_registry.max_stack_size(src_item.runtime_id as i16);
                        let space = max_stack as u16 - dst_item.count;
                        let to_move = (actual_count as u16).min(space);
                        if to_move == 0 {
                            return error_response(request.request_id);
                        }
                        let mut new_dst = dst_item;
                        new_dst.count += to_move;
                        set_item_ext(
                            self,
                            dst.container_id,
                            dst.slot,
                            new_dst,
                            ext_container_id,
                            ext_items,
                        );
                        let remaining = src_item.count - to_move;
                        if remaining == 0 {
                            set_item_ext(
                                self,
                                src.container_id,
                                src.slot,
                                ItemStack::empty(),
                                ext_container_id,
                                ext_items,
                            );
                        } else {
                            let mut new_src = src_item;
                            new_src.count = remaining;
                            set_item_ext(
                                self,
                                src.container_id,
                                src.slot,
                                new_src,
                                ext_container_id,
                                ext_items,
                            );
                        }
                    } else {
                        return error_response(request.request_id);
                    }
                    record_slot_change_ext(
                        &mut changed_containers,
                        src,
                        self,
                        ext_container_id,
                        ext_items,
                    );
                    record_slot_change_ext(
                        &mut changed_containers,
                        dst,
                        self,
                        ext_container_id,
                        ext_items,
                    );
                }
                StackAction::Swap { src, dst } => {
                    let src_item = match get_item_ext(
                        self,
                        src.container_id,
                        src.slot,
                        ext_container_id,
                        ext_items,
                    ) {
                        Some(i) => i,
                        None => return error_response(request.request_id),
                    };
                    let dst_item = match get_item_ext(
                        self,
                        dst.container_id,
                        dst.slot,
                        ext_container_id,
                        ext_items,
                    ) {
                        Some(i) => i,
                        None => return error_response(request.request_id),
                    };
                    set_item_ext(
                        self,
                        src.container_id,
                        src.slot,
                        dst_item,
                        ext_container_id,
                        ext_items,
                    );
                    set_item_ext(
                        self,
                        dst.container_id,
                        dst.slot,
                        src_item,
                        ext_container_id,
                        ext_items,
                    );
                    record_slot_change_ext(
                        &mut changed_containers,
                        src,
                        self,
                        ext_container_id,
                        ext_items,
                    );
                    record_slot_change_ext(
                        &mut changed_containers,
                        dst,
                        self,
                        ext_container_id,
                        ext_items,
                    );
                }
                StackAction::Drop { count, src, .. }
                | StackAction::Destroy { count, src }
                | StackAction::Consume { count, src } => {
                    let src_item = match get_item_ext(
                        self,
                        src.container_id,
                        src.slot,
                        ext_container_id,
                        ext_items,
                    ) {
                        Some(i) if !i.is_empty() => i,
                        _ => return error_response(request.request_id),
                    };
                    let actual_count = (*count).min(src_item.count as u8);
                    let remaining = src_item.count - actual_count as u16;
                    if remaining == 0 {
                        set_item_ext(
                            self,
                            src.container_id,
                            src.slot,
                            ItemStack::empty(),
                            ext_container_id,
                            ext_items,
                        );
                    } else {
                        let mut new_src = src_item;
                        new_src.count = remaining;
                        set_item_ext(
                            self,
                            src.container_id,
                            src.slot,
                            new_src,
                            ext_container_id,
                            ext_items,
                        );
                    }
                    record_slot_change_ext(
                        &mut changed_containers,
                        src,
                        self,
                        ext_container_id,
                        ext_items,
                    );
                }
                _ => {
                    debug!("Unexpected action in container request");
                }
            }
        }

        let containers: Vec<StackResponseContainer> = changed_containers
            .into_iter()
            .map(|(container_id, slots)| StackResponseContainer {
                container_id,
                slots,
            })
            .collect();

        StackResponseEntry {
            status: 0,
            request_id: request.request_id,
            containers,
        }
    }
}

/// Get an item from the player inventory or external container.
fn get_item_ext(
    inv: &PlayerInventory,
    container_id: u8,
    slot: u8,
    ext_cid: u8,
    ext_items: &[ItemStack],
) -> Option<ItemStack> {
    if container_id == ext_cid {
        ext_items.get(slot as usize).cloned()
    } else {
        inv.get_slot(container_id, slot).cloned()
    }
}

/// Set an item in the player inventory or external container.
fn set_item_ext(
    inv: &mut PlayerInventory,
    container_id: u8,
    slot: u8,
    item: ItemStack,
    ext_cid: u8,
    ext_items: &mut [ItemStack],
) {
    if container_id == ext_cid {
        if let Some(s) = ext_items.get_mut(slot as usize) {
            *s = item;
        }
    } else {
        inv.set_slot(container_id, slot, item);
    }
}

/// Record slot change for response, with external container support.
fn record_slot_change_ext(
    containers: &mut std::collections::HashMap<u8, Vec<StackResponseSlot>>,
    slot_ref: &StackSlot,
    inv: &PlayerInventory,
    ext_cid: u8,
    ext_items: &[ItemStack],
) {
    let item = if slot_ref.container_id == ext_cid {
        ext_items.get(slot_ref.slot as usize)
    } else {
        inv.get_slot(slot_ref.container_id, slot_ref.slot)
    };
    if let Some(item) = item {
        let entry = containers.entry(slot_ref.container_id).or_default();
        if !entry.iter().any(|s| s.slot == slot_ref.slot) {
            entry.push(StackResponseSlot {
                slot: slot_ref.slot,
                hotbar_slot: slot_ref.slot,
                count: if item.is_empty() { 0 } else { item.count as u8 },
                stack_network_id: item.stack_network_id,
                custom_name: String::new(),
                durability_correction: 0,
            });
        }
    }
}

/// Create an error response for a failed request.
fn error_response(request_id: i32) -> StackResponseEntry {
    StackResponseEntry {
        status: 1, // Error
        request_id,
        containers: Vec::new(),
    }
}

/// Record crafting output slot change for response.
fn record_crafting_output_change(
    containers: &mut std::collections::HashMap<u8, Vec<StackResponseSlot>>,
    inventory: &PlayerInventory,
) {
    let item = &inventory.crafting_output;
    let entry = containers.entry(CONTAINER_CRAFTING_OUTPUT).or_default();
    if !entry.iter().any(|s| s.slot == 0) {
        entry.push(StackResponseSlot {
            slot: 0,
            hotbar_slot: 0,
            count: if item.is_empty() { 0 } else { item.count as u8 },
            stack_network_id: item.stack_network_id,
            custom_name: String::new(),
            durability_correction: 0,
        });
    }
}

/// Record the current state of a slot for the response.
fn record_slot_change(
    containers: &mut std::collections::HashMap<u8, Vec<StackResponseSlot>>,
    slot_ref: &StackSlot,
    inventory: &PlayerInventory,
) {
    if let Some(item) = inventory.get_slot(slot_ref.container_id, slot_ref.slot) {
        let entry = containers.entry(slot_ref.container_id).or_default();
        // Avoid duplicate entries for the same slot
        if !entry.iter().any(|s| s.slot == slot_ref.slot) {
            entry.push(StackResponseSlot {
                slot: slot_ref.slot,
                hotbar_slot: slot_ref.slot,
                count: if item.is_empty() { 0 } else { item.count as u8 },
                stack_network_id: item.stack_network_id,
                custom_name: String::new(),
                durability_correction: 0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> ItemRegistry {
        ItemRegistry::new()
    }

    #[test]
    fn new_inventory_is_empty() {
        let inv = PlayerInventory::new();
        assert_eq!(inv.main.len(), 36);
        assert!(inv.main.iter().all(|s| s.is_empty()));
        assert_eq!(inv.armor.len(), 4);
        assert!(inv.armor.iter().all(|s| s.is_empty()));
        assert!(inv.offhand.is_empty());
        assert!(inv.cursor.is_empty());
        assert_eq!(inv.held_slot, 0);
    }

    #[test]
    fn get_set_slot() {
        let mut inv = PlayerInventory::new();
        let item = ItemStack::new(1, 64);
        inv.set_slot(CONTAINER_INVENTORY, 0, item);
        let stored = inv.get_slot(CONTAINER_INVENTORY, 0).unwrap();
        assert_eq!(stored.runtime_id, 1);
        assert_eq!(stored.count, 64);
    }

    #[test]
    fn get_set_armor() {
        let mut inv = PlayerInventory::new();
        let helmet = ItemStack::new(379, 1); // diamond_helmet
        inv.set_slot(CONTAINER_ARMOR, 0, helmet);
        let stored = inv.get_slot(CONTAINER_ARMOR, 0).unwrap();
        assert_eq!(stored.runtime_id, 379);
    }

    #[test]
    fn get_set_offhand() {
        let mut inv = PlayerInventory::new();
        inv.set_slot(CONTAINER_OFFHAND, 0, ItemStack::new(422, 16)); // egg
        assert_eq!(inv.offhand.runtime_id, 422);
    }

    #[test]
    fn held_item_changes_with_slot() {
        let mut inv = PlayerInventory::new();
        inv.set_slot(CONTAINER_INVENTORY, 3, ItemStack::new(1, 32));
        inv.held_slot = 3;
        assert_eq!(inv.held_item().runtime_id, 1);
        assert_eq!(inv.held_item().count, 32);
    }

    #[test]
    fn stack_network_id_increments() {
        let inv = PlayerInventory::new();
        let id1 = inv.next_stack_network_id();
        let id2 = inv.next_stack_network_id();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn assign_stack_id_to_item() {
        let inv = PlayerInventory::new();
        let mut item = ItemStack::new(1, 64);
        assert_eq!(item.stack_network_id, 0);
        inv.assign_stack_id(&mut item);
        assert_ne!(item.stack_network_id, 0);
    }

    #[test]
    fn swap_items() {
        let mut inv = PlayerInventory::new();
        inv.set_slot(CONTAINER_INVENTORY, 0, ItemStack::new(1, 64)); // stone
        inv.set_slot(CONTAINER_INVENTORY, 1, ItemStack::new(3, 32)); // dirt

        let src = StackSlot {
            container_id: CONTAINER_INVENTORY,
            slot: 0,
            stack_network_id: 0,
        };
        let dst = StackSlot {
            container_id: CONTAINER_INVENTORY,
            slot: 1,
            stack_network_id: 0,
        };

        assert!(inv.process_swap(&src, &dst));
        assert_eq!(inv.get_slot(CONTAINER_INVENTORY, 0).unwrap().runtime_id, 3);
        assert_eq!(inv.get_slot(CONTAINER_INVENTORY, 1).unwrap().runtime_id, 1);
    }

    #[test]
    fn drop_partial_stack() {
        let mut inv = PlayerInventory::new();
        inv.set_slot(CONTAINER_INVENTORY, 0, ItemStack::new(1, 64));

        let src = StackSlot {
            container_id: CONTAINER_INVENTORY,
            slot: 0,
            stack_network_id: 0,
        };

        assert!(inv.process_drop(32, &src));
        assert_eq!(inv.get_slot(CONTAINER_INVENTORY, 0).unwrap().count, 32);
    }

    #[test]
    fn drop_full_stack() {
        let mut inv = PlayerInventory::new();
        inv.set_slot(CONTAINER_INVENTORY, 0, ItemStack::new(1, 64));

        let src = StackSlot {
            container_id: CONTAINER_INVENTORY,
            slot: 0,
            stack_network_id: 0,
        };

        assert!(inv.process_drop(64, &src));
        assert!(inv.get_slot(CONTAINER_INVENTORY, 0).unwrap().is_empty());
    }

    #[test]
    fn take_place_into_empty_slot() {
        let mut inv = PlayerInventory::new();
        let registry = test_registry();
        let mut item = ItemStack::new(1, 64);
        item.stack_network_id = 1;
        inv.set_slot(CONTAINER_INVENTORY, 0, item);

        let src = StackSlot {
            container_id: CONTAINER_INVENTORY,
            slot: 0,
            stack_network_id: 1,
        };
        let dst = StackSlot {
            container_id: CONTAINER_INVENTORY,
            slot: 1,
            stack_network_id: 0,
        };

        assert!(inv.process_take_place(32, &src, &dst, &registry));
        assert_eq!(inv.get_slot(CONTAINER_INVENTORY, 0).unwrap().count, 32);
        assert_eq!(inv.get_slot(CONTAINER_INVENTORY, 1).unwrap().count, 32);
        assert_eq!(inv.get_slot(CONTAINER_INVENTORY, 1).unwrap().runtime_id, 1);
    }

    #[test]
    fn process_request_success() {
        let mut inv = PlayerInventory::new();
        let registry = test_registry();
        let mut item = ItemStack::new(1, 64);
        item.stack_network_id = 1;
        inv.set_slot(CONTAINER_INVENTORY, 0, item);

        let request = StackRequest {
            request_id: 1,
            actions: vec![StackAction::Drop {
                count: 64,
                src: StackSlot {
                    container_id: CONTAINER_INVENTORY,
                    slot: 0,
                    stack_network_id: 1,
                },
                randomly: false,
            }],
            filter_strings: Vec::new(),
            filter_cause: 0,
        };

        let recipe_reg = RecipeRegistry::new();
        let response = inv.process_request(&request, &registry, &recipe_reg);
        assert_eq!(response.status, 0);
        assert_eq!(response.request_id, 1);
        assert!(inv.get_slot(CONTAINER_INVENTORY, 0).unwrap().is_empty());
    }

    #[test]
    fn crafting_grid_access() {
        let mut inv = PlayerInventory::new();
        assert_eq!(inv.crafting_grid.len(), 9);
        assert!(inv.crafting_output.is_empty());

        inv.set_slot(CONTAINER_CRAFTING_INPUT, 0, ItemStack::new(1, 4));
        assert_eq!(
            inv.get_slot(CONTAINER_CRAFTING_INPUT, 0)
                .unwrap()
                .runtime_id,
            1
        );

        inv.set_slot(CONTAINER_CRAFTING_OUTPUT, 0, ItemStack::new(5, 1));
        assert_eq!(
            inv.get_slot(CONTAINER_CRAFTING_OUTPUT, 0)
                .unwrap()
                .runtime_id,
            5
        );
    }

    #[test]
    fn clear_crafting_grid_works() {
        let mut inv = PlayerInventory::new();
        inv.set_slot(CONTAINER_CRAFTING_INPUT, 0, ItemStack::new(1, 4));
        inv.set_slot(CONTAINER_CRAFTING_INPUT, 1, ItemStack::new(2, 8));
        inv.crafting_output = ItemStack::new(5, 1);

        inv.clear_crafting_grid();
        assert!(inv.crafting_grid.iter().all(|s| s.is_empty()));
        assert!(inv.crafting_output.is_empty());
    }

    #[test]
    fn craft_recipe_produces_output() {
        let mut inv = PlayerInventory::new();
        let registry = test_registry();
        let recipe_reg = RecipeRegistry::new();

        // Find the first shapeless recipe (planks from log)
        let planks_recipe = recipe_reg.shapeless_recipes().first().unwrap();
        let log_rid = registry
            .get_by_name(&planks_recipe.inputs[0].item_name)
            .map(|e| e.numeric_id as i32)
            .unwrap_or(0);

        // Place log in crafting grid slot 0
        if log_rid != 0 {
            inv.set_slot(CONTAINER_CRAFTING_INPUT, 0, ItemStack::new(log_rid, 1));
            let ok = inv.process_craft_recipe(planks_recipe.network_id, 1, &recipe_reg, &registry);
            assert!(ok);
            assert!(!inv.crafting_output.is_empty());
            assert_eq!(inv.crafting_output.count, 4); // planks = 4
        }
    }
}
