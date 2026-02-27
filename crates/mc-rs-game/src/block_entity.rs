//! Block entity data types and NBT serialization.
//!
//! Block entities are blocks with associated data (signs with text,
//! chests with inventory, etc.). This module defines the data model
//! and NBT (de)serialization for both network and disk formats.

use mc_rs_nbt::tag::{NbtCompound, NbtRoot, NbtTag};
use mc_rs_nbt::{read_nbt_le, read_nbt_network, write_nbt_le, write_nbt_network};
use mc_rs_proto::item_stack::ItemStack;

/// Block entity data stored per-block.
#[derive(Debug, Clone)]
pub enum BlockEntityData {
    Sign {
        front_text: String,
        back_text: String,
        is_editable: bool,
    },
    Chest {
        items: Vec<ItemStack>,
    },
}

/// Number of slots in a single chest.
pub const CHEST_SLOTS: usize = 27;

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
            _ => return None,
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
}
