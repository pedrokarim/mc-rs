//! Block entity data types and NBT serialization.
//!
//! Block entities are blocks with associated data (signs with text,
//! chests with inventory, etc.). This module defines the data model
//! and NBT (de)serialization for both network and disk formats.

use mc_rs_nbt::tag::{NbtCompound, NbtRoot, NbtTag};
use mc_rs_nbt::{read_nbt_le, read_nbt_network, write_nbt_le, write_nbt_network};
use mc_rs_proto::item_stack::ItemStack;

use crate::smelting::FurnaceType;

/// Block entity data stored per-block.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum BlockEntityData {
    Sign {
        front_text: String,
        back_text: String,
        is_editable: bool,
    },
    Chest {
        items: Vec<ItemStack>,
    },
    Furnace {
        furnace_type: FurnaceType,
        input: ItemStack,
        fuel: ItemStack,
        output: ItemStack,
        cook_time: i16,
        cook_time_total: i16,
        lit_time: i16,
        lit_duration: i16,
        stored_xp: f32,
    },
    EnchantingTable {
        item: ItemStack,
        lapis: ItemStack,
    },
    Stonecutter {
        input: ItemStack,
    },
    Grindstone {
        input1: ItemStack,
        input2: ItemStack,
    },
    Loom {
        banner: ItemStack,
        dye: ItemStack,
        pattern: ItemStack,
    },
    Anvil {
        input: ItemStack,
        material: ItemStack,
    },
}

/// Number of slots in a single chest.
pub const CHEST_SLOTS: usize = 27;

/// Number of slots in a furnace.
pub const FURNACE_SLOTS: usize = 3;

/// Number of slots in an enchanting table.
pub const ENCHANTING_TABLE_SLOTS: usize = 2;

/// Number of slots in a stonecutter.
pub const STONECUTTER_SLOTS: usize = 1;

/// Number of slots in a grindstone.
pub const GRINDSTONE_SLOTS: usize = 2;

/// Number of slots in a loom.
pub const LOOM_SLOTS: usize = 3;

/// Number of slots in an anvil.
pub const ANVIL_SLOTS: usize = 2;

impl BlockEntityData {
    /// Create a new empty sign (editable, no text).
    pub fn new_sign() -> Self {
        BlockEntityData::Sign {
            front_text: String::new(),
            back_text: String::new(),
            is_editable: true,
        }
    }

    /// Create a new empty chest (27 empty slots).
    pub fn new_chest() -> Self {
        BlockEntityData::Chest {
            items: (0..CHEST_SLOTS).map(|_| ItemStack::empty()).collect(),
        }
    }

    /// Create a new empty enchanting table (2 slots: item + lapis).
    pub fn new_enchanting_table() -> Self {
        BlockEntityData::EnchantingTable {
            item: ItemStack::empty(),
            lapis: ItemStack::empty(),
        }
    }

    /// Create a new empty stonecutter (1 input slot).
    pub fn new_stonecutter() -> Self {
        BlockEntityData::Stonecutter {
            input: ItemStack::empty(),
        }
    }

    /// Create a new empty grindstone (2 input slots).
    pub fn new_grindstone() -> Self {
        BlockEntityData::Grindstone {
            input1: ItemStack::empty(),
            input2: ItemStack::empty(),
        }
    }

    /// Create a new empty loom (3 slots: banner + dye + pattern).
    pub fn new_loom() -> Self {
        BlockEntityData::Loom {
            banner: ItemStack::empty(),
            dye: ItemStack::empty(),
            pattern: ItemStack::empty(),
        }
    }

    /// Create a new empty anvil.
    pub fn new_anvil() -> Self {
        BlockEntityData::Anvil {
            input: ItemStack::empty(),
            material: ItemStack::empty(),
        }
    }

    /// Create a new empty furnace of the given type.
    pub fn new_furnace(furnace_type: FurnaceType) -> Self {
        BlockEntityData::Furnace {
            furnace_type,
            input: ItemStack::empty(),
            fuel: ItemStack::empty(),
            output: ItemStack::empty(),
            cook_time: 0,
            cook_time_total: furnace_type.cook_time(),
            lit_time: 0,
            lit_duration: 0,
            stored_xp: 0.0,
        }
    }

    /// Serialize to network NBT bytes (for BlockActorData packets).
    pub fn to_network_nbt(&self, x: i32, y: i32, z: i32) -> Vec<u8> {
        let compound = self.build_nbt_compound(x, y, z);
        let root = NbtRoot::new("", compound);
        let mut buf = Vec::new();
        write_nbt_network(&mut buf, &root);
        buf
    }

    /// Serialize to LE NBT bytes (for LevelDB persistence).
    pub fn to_le_nbt(&self, x: i32, y: i32, z: i32) -> Vec<u8> {
        let compound = self.build_nbt_compound(x, y, z);
        let root = NbtRoot::new("", compound);
        let mut buf = Vec::new();
        write_nbt_le(&mut buf, &root);
        buf
    }

    /// Parse a block entity from LE NBT bytes (loaded from LevelDB).
    ///
    /// Returns the world position and the block entity data,
    /// or `None` if the NBT is not a recognized block entity.
    pub fn from_le_nbt(data: &[u8]) -> Option<((i32, i32, i32), Self)> {
        let root = read_nbt_le(&mut &data[..]).ok()?;
        Self::from_nbt_compound(&root.compound)
    }

    /// Parse sign text from network NBT bytes (from client BlockActorData).
    ///
    /// Returns `(front_text, back_text)` if valid sign data.
    pub fn sign_from_network_nbt(data: &[u8]) -> Option<(String, String)> {
        let root = read_nbt_network(&mut &data[..]).ok()?;
        let c = &root.compound;

        let front = c
            .get("FrontText")
            .and_then(|t| t.as_compound())
            .and_then(|c| c.get("Text"))
            .and_then(|t| t.as_string())
            .unwrap_or_default()
            .to_string();

        let back = c
            .get("BackText")
            .and_then(|t| t.as_compound())
            .and_then(|c| c.get("Text"))
            .and_then(|t| t.as_string())
            .unwrap_or_default()
            .to_string();

        Some((front, back))
    }

    fn build_nbt_compound(&self, x: i32, y: i32, z: i32) -> NbtCompound {
        let mut c = NbtCompound::new();
        c.insert("x".to_string(), NbtTag::Int(x));
        c.insert("y".to_string(), NbtTag::Int(y));
        c.insert("z".to_string(), NbtTag::Int(z));

        match self {
            BlockEntityData::Sign {
                front_text,
                back_text,
                is_editable,
            } => {
                c.insert("id".to_string(), NbtTag::String("Sign".to_string()));

                let mut front = NbtCompound::new();
                front.insert("Text".to_string(), NbtTag::String(front_text.clone()));
                c.insert("FrontText".to_string(), NbtTag::Compound(front));

                let mut back = NbtCompound::new();
                back.insert("Text".to_string(), NbtTag::String(back_text.clone()));
                c.insert("BackText".to_string(), NbtTag::Compound(back));

                c.insert(
                    "IsEditable".to_string(),
                    NbtTag::Byte(if *is_editable { 1 } else { 0 }),
                );
            }
            BlockEntityData::Chest { items } => {
                c.insert("id".to_string(), NbtTag::String("Chest".to_string()));
                c.insert("Findable".to_string(), NbtTag::Byte(0));

                let item_list: Vec<NbtTag> = items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| !item.is_empty())
                    .map(|(slot, item)| {
                        let mut ic = NbtCompound::new();
                        ic.insert("Slot".to_string(), NbtTag::Byte(slot as i8));
                        ic.insert("id".to_string(), NbtTag::Short(item.runtime_id as i16));
                        ic.insert("Count".to_string(), NbtTag::Byte(item.count as i8));
                        ic.insert("Damage".to_string(), NbtTag::Short(item.metadata as i16));
                        NbtTag::Compound(ic)
                    })
                    .collect();

                c.insert("Items".to_string(), NbtTag::List(item_list));
            }
            BlockEntityData::Furnace {
                furnace_type,
                input,
                fuel,
                output,
                cook_time,
                cook_time_total,
                lit_time,
                lit_duration,
                stored_xp,
            } => {
                c.insert(
                    "id".to_string(),
                    NbtTag::String(furnace_type.nbt_id().to_string()),
                );

                let slots = [(0i8, input), (1, fuel), (2, output)];
                let item_list: Vec<NbtTag> = slots
                    .iter()
                    .filter(|(_, item)| !item.is_empty())
                    .map(|(slot, item)| {
                        let mut ic = NbtCompound::new();
                        ic.insert("Slot".to_string(), NbtTag::Byte(*slot));
                        ic.insert("id".to_string(), NbtTag::Short(item.runtime_id as i16));
                        ic.insert("Count".to_string(), NbtTag::Byte(item.count as i8));
                        ic.insert("Damage".to_string(), NbtTag::Short(item.metadata as i16));
                        NbtTag::Compound(ic)
                    })
                    .collect();
                c.insert("Items".to_string(), NbtTag::List(item_list));

                c.insert("BurnTime".to_string(), NbtTag::Short(*lit_time));
                c.insert("CookTime".to_string(), NbtTag::Short(*cook_time));
                c.insert("CookTimeTotal".to_string(), NbtTag::Short(*cook_time_total));
                c.insert("BurnDuration".to_string(), NbtTag::Short(*lit_duration));
                c.insert("StoredXPInt".to_string(), NbtTag::Int(*stored_xp as i32));
            }
            BlockEntityData::EnchantingTable { item, lapis } => {
                c.insert("id".to_string(), NbtTag::String("EnchantTable".to_string()));

                let slots = [(0i8, item), (1, lapis)];
                let item_list: Vec<NbtTag> = slots
                    .iter()
                    .filter(|(_, it)| !it.is_empty())
                    .map(|(slot, it)| {
                        let mut ic = NbtCompound::new();
                        ic.insert("Slot".to_string(), NbtTag::Byte(*slot));
                        ic.insert("id".to_string(), NbtTag::Short(it.runtime_id as i16));
                        ic.insert("Count".to_string(), NbtTag::Byte(it.count as i8));
                        ic.insert("Damage".to_string(), NbtTag::Short(it.metadata as i16));
                        NbtTag::Compound(ic)
                    })
                    .collect();
                c.insert("Items".to_string(), NbtTag::List(item_list));
            }
            // Transient containers — items are lost on close, no disk persistence.
            BlockEntityData::Stonecutter { .. }
            | BlockEntityData::Grindstone { .. }
            | BlockEntityData::Loom { .. }
            | BlockEntityData::Anvil { .. } => {}
        }

        c
    }

    fn from_nbt_compound(c: &NbtCompound) -> Option<((i32, i32, i32), Self)> {
        let x = c.get("x").and_then(|t| t.as_int())?;
        let y = c.get("y").and_then(|t| t.as_int())?;
        let z = c.get("z").and_then(|t| t.as_int())?;

        let id = c.get("id").and_then(|t| t.as_string())?;

        let data = match id {
            "Sign" => {
                let front = c
                    .get("FrontText")
                    .and_then(|t| t.as_compound())
                    .and_then(|c| c.get("Text"))
                    .and_then(|t| t.as_string())
                    .unwrap_or_default()
                    .to_string();
                let back = c
                    .get("BackText")
                    .and_then(|t| t.as_compound())
                    .and_then(|c| c.get("Text"))
                    .and_then(|t| t.as_string())
                    .unwrap_or_default()
                    .to_string();
                let is_editable = c.get("IsEditable").and_then(|t| t.as_byte()).unwrap_or(0) != 0;
                BlockEntityData::Sign {
                    front_text: front,
                    back_text: back,
                    is_editable,
                }
            }
            "Chest" => {
                let mut items: Vec<ItemStack> =
                    (0..CHEST_SLOTS).map(|_| ItemStack::empty()).collect();
                if let Some(NbtTag::List(item_list)) = c.get("Items") {
                    for tag in item_list {
                        if let NbtTag::Compound(ic) = tag {
                            let slot = ic.get("Slot").and_then(|t| t.as_byte()).unwrap_or(-1);
                            if slot < 0 || slot as usize >= CHEST_SLOTS {
                                continue;
                            }
                            let rid = ic.get("id").and_then(|t| t.as_short()).unwrap_or(0);
                            let count = ic.get("Count").and_then(|t| t.as_byte()).unwrap_or(0);
                            let damage = ic.get("Damage").and_then(|t| t.as_short()).unwrap_or(0);
                            if rid != 0 && count > 0 {
                                items[slot as usize] = ItemStack {
                                    runtime_id: rid as i32,
                                    count: count as u16,
                                    metadata: damage as u16,
                                    block_runtime_id: 0,
                                    nbt_data: Vec::new(),
                                    can_place_on: Vec::new(),
                                    can_destroy: Vec::new(),
                                    stack_network_id: 0,
                                };
                            }
                        }
                    }
                }
                BlockEntityData::Chest { items }
            }
            "EnchantTable" => {
                let mut item = ItemStack::empty();
                let mut lapis = ItemStack::empty();
                if let Some(NbtTag::List(item_list)) = c.get("Items") {
                    for tag in item_list {
                        if let NbtTag::Compound(ic) = tag {
                            let slot = ic.get("Slot").and_then(|t| t.as_byte()).unwrap_or(-1);
                            let rid = ic.get("id").and_then(|t| t.as_short()).unwrap_or(0);
                            let count = ic.get("Count").and_then(|t| t.as_byte()).unwrap_or(0);
                            let damage = ic.get("Damage").and_then(|t| t.as_short()).unwrap_or(0);
                            if rid != 0 && count > 0 {
                                let parsed = ItemStack {
                                    runtime_id: rid as i32,
                                    count: count as u16,
                                    metadata: damage as u16,
                                    block_runtime_id: 0,
                                    nbt_data: Vec::new(),
                                    can_place_on: Vec::new(),
                                    can_destroy: Vec::new(),
                                    stack_network_id: 0,
                                };
                                match slot {
                                    0 => item = parsed,
                                    1 => lapis = parsed,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                BlockEntityData::EnchantingTable { item, lapis }
            }
            other => {
                if let Some(ft) = FurnaceType::from_nbt_id(other) {
                    let mut input = ItemStack::empty();
                    let mut fuel = ItemStack::empty();
                    let mut output = ItemStack::empty();
                    if let Some(NbtTag::List(item_list)) = c.get("Items") {
                        for tag in item_list {
                            if let NbtTag::Compound(ic) = tag {
                                let slot = ic.get("Slot").and_then(|t| t.as_byte()).unwrap_or(-1);
                                let rid = ic.get("id").and_then(|t| t.as_short()).unwrap_or(0);
                                let count = ic.get("Count").and_then(|t| t.as_byte()).unwrap_or(0);
                                let damage =
                                    ic.get("Damage").and_then(|t| t.as_short()).unwrap_or(0);
                                if rid != 0 && count > 0 {
                                    let item = ItemStack {
                                        runtime_id: rid as i32,
                                        count: count as u16,
                                        metadata: damage as u16,
                                        block_runtime_id: 0,
                                        nbt_data: Vec::new(),
                                        can_place_on: Vec::new(),
                                        can_destroy: Vec::new(),
                                        stack_network_id: 0,
                                    };
                                    match slot {
                                        0 => input = item,
                                        1 => fuel = item,
                                        2 => output = item,
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    let lit_time = c.get("BurnTime").and_then(|t| t.as_short()).unwrap_or(0);
                    let cook_time = c.get("CookTime").and_then(|t| t.as_short()).unwrap_or(0);
                    let cook_time_total = c
                        .get("CookTimeTotal")
                        .and_then(|t| t.as_short())
                        .unwrap_or(ft.cook_time());
                    let lit_duration = c
                        .get("BurnDuration")
                        .and_then(|t| t.as_short())
                        .unwrap_or(0);
                    let stored_xp =
                        c.get("StoredXPInt").and_then(|t| t.as_int()).unwrap_or(0) as f32;
                    BlockEntityData::Furnace {
                        furnace_type: ft,
                        input,
                        fuel,
                        output,
                        cook_time,
                        cook_time_total,
                        lit_time,
                        lit_duration,
                        stored_xp,
                    }
                } else {
                    return None;
                }
            }
        };

        Some(((x, y, z), data))
    }
}

/// Parse multiple concatenated LE NBT compounds (from LevelDB tag 0x31).
pub fn parse_block_entities(data: &[u8]) -> Vec<((i32, i32, i32), BlockEntityData)> {
    let mut result = Vec::new();
    let mut cursor = data;
    while !cursor.is_empty() {
        match read_nbt_le(&mut cursor) {
            Ok(root) => {
                if let Some(entry) = BlockEntityData::from_nbt_compound(&root.compound) {
                    result.push(entry);
                }
            }
            Err(_) => break,
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sign_defaults() {
        let be = BlockEntityData::new_sign();
        match &be {
            BlockEntityData::Sign {
                front_text,
                back_text,
                is_editable,
            } => {
                assert!(front_text.is_empty());
                assert!(back_text.is_empty());
                assert!(*is_editable);
            }
            _ => panic!("Expected Sign"),
        }
    }

    #[test]
    fn new_chest_defaults() {
        let be = BlockEntityData::new_chest();
        match &be {
            BlockEntityData::Chest { items } => {
                assert_eq!(items.len(), 27);
                assert!(items.iter().all(|i| i.is_empty()));
            }
            _ => panic!("Expected Chest"),
        }
    }

    #[test]
    fn sign_network_nbt_roundtrip() {
        let be = BlockEntityData::Sign {
            front_text: "Hello\nWorld".to_string(),
            back_text: "Back".to_string(),
            is_editable: false,
        };
        let nbt = be.to_network_nbt(10, 64, -5);
        assert!(!nbt.is_empty());

        let (front, back) = BlockEntityData::sign_from_network_nbt(&nbt).unwrap();
        assert_eq!(front, "Hello\nWorld");
        assert_eq!(back, "Back");
    }

    #[test]
    fn sign_le_nbt_roundtrip() {
        let be = BlockEntityData::Sign {
            front_text: "Line1".to_string(),
            back_text: "Line2".to_string(),
            is_editable: true,
        };
        let data = be.to_le_nbt(5, 100, -3);
        let ((x, y, z), parsed) = BlockEntityData::from_le_nbt(&data).unwrap();
        assert_eq!((x, y, z), (5, 100, -3));
        match parsed {
            BlockEntityData::Sign {
                front_text,
                back_text,
                is_editable,
            } => {
                assert_eq!(front_text, "Line1");
                assert_eq!(back_text, "Line2");
                assert!(is_editable);
            }
            _ => panic!("Expected Sign"),
        }
    }

    #[test]
    fn chest_le_nbt_roundtrip() {
        let mut be = BlockEntityData::new_chest();
        if let BlockEntityData::Chest { ref mut items } = be {
            items[0] = ItemStack {
                runtime_id: 1,
                count: 64,
                metadata: 0,
                block_runtime_id: 0,
                nbt_data: Vec::new(),
                can_place_on: Vec::new(),
                can_destroy: Vec::new(),
                stack_network_id: 0,
            };
            items[26] = ItemStack {
                runtime_id: 5,
                count: 10,
                metadata: 3,
                block_runtime_id: 0,
                nbt_data: Vec::new(),
                can_place_on: Vec::new(),
                can_destroy: Vec::new(),
                stack_network_id: 0,
            };
        }
        let data = be.to_le_nbt(0, 64, 0);
        let ((x, y, z), parsed) = BlockEntityData::from_le_nbt(&data).unwrap();
        assert_eq!((x, y, z), (0, 64, 0));
        match parsed {
            BlockEntityData::Chest { items } => {
                assert_eq!(items.len(), 27);
                assert_eq!(items[0].runtime_id, 1);
                assert_eq!(items[0].count, 64);
                assert_eq!(items[26].runtime_id, 5);
                assert_eq!(items[26].count, 10);
                assert_eq!(items[26].metadata, 3);
                assert!(items[1].is_empty());
            }
            _ => panic!("Expected Chest"),
        }
    }

    #[test]
    fn chest_network_nbt() {
        let be = BlockEntityData::new_chest();
        let nbt = be.to_network_nbt(0, 64, 0);
        assert!(!nbt.is_empty());
        // Should be parseable as network NBT
        let root = read_nbt_network(&mut &nbt[..]).unwrap();
        assert_eq!(
            root.compound.get("id").and_then(|t| t.as_string()),
            Some("Chest")
        );
    }

    #[test]
    fn parse_multiple_block_entities() {
        let sign = BlockEntityData::Sign {
            front_text: "A".to_string(),
            back_text: "B".to_string(),
            is_editable: false,
        };
        let chest = BlockEntityData::new_chest();

        let mut data = Vec::new();
        data.extend_from_slice(&sign.to_le_nbt(0, 64, 0));
        data.extend_from_slice(&chest.to_le_nbt(1, 64, 1));

        let entries = parse_block_entities(&data);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, (0, 64, 0));
        assert_eq!(entries[1].0, (1, 64, 1));
        assert!(matches!(entries[0].1, BlockEntityData::Sign { .. }));
        assert!(matches!(entries[1].1, BlockEntityData::Chest { .. }));
    }

    #[test]
    fn sign_from_network_nbt_empty() {
        let be = BlockEntityData::new_sign();
        let nbt = be.to_network_nbt(0, 0, 0);
        let (front, back) = BlockEntityData::sign_from_network_nbt(&nbt).unwrap();
        assert!(front.is_empty());
        assert!(back.is_empty());
    }

    #[test]
    fn new_furnace_defaults() {
        let be = BlockEntityData::new_furnace(FurnaceType::Furnace);
        match &be {
            BlockEntityData::Furnace {
                furnace_type,
                input,
                fuel,
                output,
                cook_time,
                cook_time_total,
                lit_time,
                ..
            } => {
                assert_eq!(*furnace_type, FurnaceType::Furnace);
                assert!(input.is_empty());
                assert!(fuel.is_empty());
                assert!(output.is_empty());
                assert_eq!(*cook_time, 0);
                assert_eq!(*cook_time_total, 200);
                assert_eq!(*lit_time, 0);
            }
            _ => panic!("Expected Furnace"),
        }
    }

    #[test]
    fn furnace_le_nbt_roundtrip() {
        let mut be = BlockEntityData::new_furnace(FurnaceType::BlastFurnace);
        if let BlockEntityData::Furnace {
            ref mut input,
            ref mut fuel,
            ref mut cook_time,
            ref mut lit_time,
            ref mut lit_duration,
            ..
        } = be
        {
            *input = ItemStack {
                runtime_id: 10,
                count: 5,
                metadata: 0,
                block_runtime_id: 0,
                nbt_data: Vec::new(),
                can_place_on: Vec::new(),
                can_destroy: Vec::new(),
                stack_network_id: 0,
            };
            *fuel = ItemStack {
                runtime_id: 20,
                count: 64,
                metadata: 0,
                block_runtime_id: 0,
                nbt_data: Vec::new(),
                can_place_on: Vec::new(),
                can_destroy: Vec::new(),
                stack_network_id: 0,
            };
            *cook_time = 50;
            *lit_time = 1200;
            *lit_duration = 1600;
        }
        let data = be.to_le_nbt(3, 65, -7);
        let ((x, y, z), parsed) = BlockEntityData::from_le_nbt(&data).unwrap();
        assert_eq!((x, y, z), (3, 65, -7));
        match parsed {
            BlockEntityData::Furnace {
                furnace_type,
                input,
                fuel,
                output,
                cook_time,
                lit_time,
                lit_duration,
                ..
            } => {
                assert_eq!(furnace_type, FurnaceType::BlastFurnace);
                assert_eq!(input.runtime_id, 10);
                assert_eq!(input.count, 5);
                assert_eq!(fuel.runtime_id, 20);
                assert_eq!(fuel.count, 64);
                assert!(output.is_empty());
                assert_eq!(cook_time, 50);
                assert_eq!(lit_time, 1200);
                assert_eq!(lit_duration, 1600);
            }
            _ => panic!("Expected Furnace"),
        }
    }

    #[test]
    fn furnace_network_nbt() {
        let be = BlockEntityData::new_furnace(FurnaceType::Smoker);
        let nbt = be.to_network_nbt(0, 64, 0);
        assert!(!nbt.is_empty());
        let root = read_nbt_network(&mut &nbt[..]).unwrap();
        assert_eq!(
            root.compound.get("id").and_then(|t| t.as_string()),
            Some("Smoker")
        );
    }

    #[test]
    fn new_enchanting_table_defaults() {
        let be = BlockEntityData::new_enchanting_table();
        match &be {
            BlockEntityData::EnchantingTable { item, lapis } => {
                assert!(item.is_empty());
                assert!(lapis.is_empty());
            }
            _ => panic!("Expected EnchantingTable"),
        }
    }

    #[test]
    fn enchanting_table_le_nbt_roundtrip() {
        let mut be = BlockEntityData::new_enchanting_table();
        if let BlockEntityData::EnchantingTable {
            ref mut item,
            ref mut lapis,
        } = be
        {
            *item = ItemStack {
                runtime_id: 100,
                count: 1,
                metadata: 0,
                block_runtime_id: 0,
                nbt_data: Vec::new(),
                can_place_on: Vec::new(),
                can_destroy: Vec::new(),
                stack_network_id: 0,
            };
            *lapis = ItemStack {
                runtime_id: 200,
                count: 3,
                metadata: 0,
                block_runtime_id: 0,
                nbt_data: Vec::new(),
                can_place_on: Vec::new(),
                can_destroy: Vec::new(),
                stack_network_id: 0,
            };
        }
        let data = be.to_le_nbt(8, 70, -2);
        let ((x, y, z), parsed) = BlockEntityData::from_le_nbt(&data).unwrap();
        assert_eq!((x, y, z), (8, 70, -2));
        match parsed {
            BlockEntityData::EnchantingTable { item, lapis } => {
                assert_eq!(item.runtime_id, 100);
                assert_eq!(item.count, 1);
                assert_eq!(lapis.runtime_id, 200);
                assert_eq!(lapis.count, 3);
            }
            _ => panic!("Expected EnchantingTable"),
        }
    }

    #[test]
    fn parse_mixed_block_entities_with_furnace() {
        let sign = BlockEntityData::new_sign();
        let furnace = BlockEntityData::new_furnace(FurnaceType::Furnace);

        let mut data = Vec::new();
        data.extend_from_slice(&sign.to_le_nbt(0, 64, 0));
        data.extend_from_slice(&furnace.to_le_nbt(1, 65, 1));

        let entries = parse_block_entities(&data);
        assert_eq!(entries.len(), 2);
        assert!(matches!(entries[0].1, BlockEntityData::Sign { .. }));
        assert!(matches!(entries[1].1, BlockEntityData::Furnace { .. }));
    }
}
