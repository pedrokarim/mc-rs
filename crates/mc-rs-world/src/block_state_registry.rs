//! Bidirectional block state registry: hash ↔ block name + properties.
//!
//! Used by BDS import/export to convert between our FNV-1a hash palette
//! and BDS NBT compound palette.

use std::collections::HashMap;

use crate::block_hash::{
    hash_block_state, hash_block_state_with_int, hash_block_state_with_props, StateValue,
    LEVER_DIRS, TORCH_DIRS,
};

/// Block state version for 1.21.50 protocol (same as block_hash.rs).
const BLOCK_STATE_VERSION: i32 = 18_100_737;

/// Cardinal direction string values (alphabetical, same order as block_hash.rs).
const CARDINAL_DIRS: [&str; 4] = ["east", "north", "south", "west"];

/// Owned version of [`StateValue`] for storage in the registry.
#[derive(Debug, Clone, PartialEq)]
pub enum StateValueOwned {
    Int(i32),
    Byte(i8),
    Str(String),
}

/// Information about a block state: name + properties.
#[derive(Debug, Clone)]
pub struct BlockStateInfo {
    pub name: String,
    pub properties: Vec<(String, StateValueOwned)>,
}

/// Bidirectional registry mapping FNV-1a hashes to block state info.
#[derive(Debug, Clone)]
pub struct BlockStateRegistry {
    entries: HashMap<u32, BlockStateInfo>,
}

impl BlockStateRegistry {
    /// Build the registry with all known block states.
    pub fn new() -> Self {
        let mut reg = Self {
            entries: HashMap::with_capacity(512),
        };

        // --- Simple blocks (no state properties) ---
        let simple_blocks = [
            "minecraft:air",
            "minecraft:bedrock",
            "minecraft:dirt",
            "minecraft:grass_block",
            "minecraft:stone",
            "minecraft:granite",
            "minecraft:diorite",
            "minecraft:andesite",
            "minecraft:deepslate",
            "minecraft:sand",
            "minecraft:sandstone",
            "minecraft:gravel",
            "minecraft:clay",
            "minecraft:snow_layer",
            "minecraft:ice",
            "minecraft:packed_ice",
            "minecraft:coal_ore",
            "minecraft:iron_ore",
            "minecraft:gold_ore",
            "minecraft:diamond_ore",
            "minecraft:redstone_ore",
            "minecraft:lapis_lazuli_ore",
            "minecraft:emerald_ore",
            "minecraft:copper_ore",
            "minecraft:deepslate_coal_ore",
            "minecraft:deepslate_iron_ore",
            "minecraft:deepslate_gold_ore",
            "minecraft:deepslate_diamond_ore",
            "minecraft:deepslate_redstone_ore",
            "minecraft:deepslate_lapis_lazuli_ore",
            "minecraft:deepslate_emerald_ore",
            "minecraft:deepslate_copper_ore",
            "minecraft:oak_log",
            "minecraft:oak_leaves",
            "minecraft:birch_log",
            "minecraft:birch_leaves",
            "minecraft:spruce_log",
            "minecraft:spruce_leaves",
            "minecraft:acacia_log",
            "minecraft:acacia_leaves",
            "minecraft:tallgrass",
            "minecraft:red_flower",
            "minecraft:yellow_flower",
            "minecraft:deadbush",
            "minecraft:cactus",
            "minecraft:cobblestone",
            "minecraft:mossy_cobblestone",
            "minecraft:planks",
            "minecraft:stonebrick",
            "minecraft:mob_spawner",
            "minecraft:red_sand",
            "minecraft:redstone_block",
            "minecraft:fire",
            "minecraft:enchanting_table",
            "minecraft:bookshelf",
            "minecraft:end_portal",
            "minecraft:end_stone",
            "minecraft:obsidian",
            "minecraft:netherrack",
            "minecraft:soul_sand",
            "minecraft:soul_soil",
            "minecraft:glowstone",
            "minecraft:nether_brick",
            "minecraft:quartz_ore",
            "minecraft:nether_gold_ore",
            "minecraft:magma",
            "minecraft:glass",
            "minecraft:wool",
            "minecraft:torch",
            "minecraft:crafting_table",
            "minecraft:ladder",
            "minecraft:sponge",
            "minecraft:tnt",
        ];
        for name in &simple_blocks {
            reg.register_simple(name);
        }

        // --- Crops: growth 0..N ---
        for g in 0..8 {
            reg.register_int("minecraft:wheat", "growth", g);
            reg.register_int("minecraft:carrots", "growth", g);
            reg.register_int("minecraft:potatoes", "growth", g);
        }
        for g in 0..4 {
            reg.register_int("minecraft:beetroot", "growth", g);
        }

        // --- Farmland: moisturized_amount 0..7 ---
        for m in 0..8 {
            reg.register_int("minecraft:farmland", "moisturized_amount", m);
        }

        // --- Fluids: liquid_depth 0..15 ---
        for d in 0..16 {
            reg.register_int("minecraft:water", "liquid_depth", d);
            reg.register_int("minecraft:lava", "liquid_depth", d);
        }

        // --- Redstone wire: redstone_signal 0..15 ---
        for s in 0..16 {
            reg.register_int("minecraft:redstone_wire", "redstone_signal", s);
        }

        // --- Lever: lever_direction × open_bit ---
        for dir in &LEVER_DIRS {
            for bit in 0..2i8 {
                reg.register_props(
                    "minecraft:lever",
                    &[
                        ("lever_direction", StateValue::Str(dir)),
                        ("open_bit", StateValue::Byte(bit)),
                    ],
                );
            }
        }

        // --- Redstone torch: lit and unlit × torch_facing_direction ---
        for dir in &TORCH_DIRS {
            reg.register_props(
                "minecraft:redstone_torch",
                &[("torch_facing_direction", StateValue::Str(dir))],
            );
            reg.register_props(
                "minecraft:unlit_redstone_torch",
                &[("torch_facing_direction", StateValue::Str(dir))],
            );
        }

        // --- Repeater: direction × repeater_delay, unpowered and powered ---
        for dir in 0..4 {
            for delay in 0..4 {
                reg.register_props(
                    "minecraft:unpowered_repeater",
                    &[
                        ("direction", StateValue::Int(dir)),
                        ("repeater_delay", StateValue::Int(delay)),
                    ],
                );
                reg.register_props(
                    "minecraft:powered_repeater",
                    &[
                        ("direction", StateValue::Int(dir)),
                        ("repeater_delay", StateValue::Int(delay)),
                    ],
                );
            }
        }

        // --- Pistons: facing_direction 0..5 ---
        for fd in 0..6 {
            reg.register_int("minecraft:piston", "facing_direction", fd);
            reg.register_int("minecraft:sticky_piston", "facing_direction", fd);
            reg.register_int("minecraft:piston_arm_collision", "facing_direction", fd);
            reg.register_int(
                "minecraft:sticky_piston_arm_collision",
                "facing_direction",
                fd,
            );
        }

        // --- Portal blocks ---
        reg.register_props("minecraft:portal", &[("portal_axis", StateValue::Str("x"))]);
        reg.register_props("minecraft:portal", &[("portal_axis", StateValue::Str("z"))]);

        // --- End portal frame: direction × end_portal_eye_bit ---
        for dir in 0..4 {
            for eye in 0..2i8 {
                reg.register_props(
                    "minecraft:end_portal_frame",
                    &[
                        ("direction", StateValue::Int(dir)),
                        ("end_portal_eye_bit", StateValue::Byte(eye)),
                    ],
                );
            }
        }

        // --- Signs ---
        for gsd in 0..16 {
            reg.register_int("minecraft:oak_sign", "ground_sign_direction", gsd);
        }
        for face in 2..=5 {
            reg.register_int("minecraft:oak_wall_sign", "facing_direction", face);
        }

        // --- Chest: facing_direction 2-5 ---
        for face in 2..=5 {
            reg.register_int("minecraft:chest", "facing_direction", face);
        }

        // --- Furnaces: cardinal_direction × (lit/unlit) × 3 variants ---
        let furnace_pairs = [
            ("minecraft:furnace", "minecraft:lit_furnace"),
            ("minecraft:blast_furnace", "minecraft:lit_blast_furnace"),
            ("minecraft:smoker", "minecraft:lit_smoker"),
        ];
        for (unlit, lit) in &furnace_pairs {
            for dir in &CARDINAL_DIRS {
                reg.register_props(
                    unlit,
                    &[("minecraft:cardinal_direction", StateValue::Str(dir))],
                );
                reg.register_props(
                    lit,
                    &[("minecraft:cardinal_direction", StateValue::Str(dir))],
                );
            }
        }

        // --- Stonecutter: cardinal_direction ---
        for dir in &CARDINAL_DIRS {
            reg.register_props(
                "minecraft:stonecutter_block",
                &[("minecraft:cardinal_direction", StateValue::Str(dir))],
            );
        }

        // --- Grindstone: attachment × direction ---
        let attachments = ["hanging", "multiple", "side", "standing"];
        for att in &attachments {
            for dir in 0..4 {
                reg.register_props(
                    "minecraft:grindstone",
                    &[
                        ("attachment", StateValue::Str(att)),
                        ("direction", StateValue::Int(dir)),
                    ],
                );
            }
        }

        // --- Loom: direction 0-3 ---
        for dir in 0..4 {
            reg.register_int("minecraft:loom", "direction", dir);
        }

        // --- Anvil: damage × cardinal_direction ---
        let damage_states = ["undamaged", "slightly_damaged", "very_damaged"];
        for dmg in &damage_states {
            for dir in &CARDINAL_DIRS {
                reg.register_props(
                    "minecraft:anvil",
                    &[
                        ("damage", StateValue::Str(dmg)),
                        ("minecraft:cardinal_direction", StateValue::Str(dir)),
                    ],
                );
            }
        }

        reg
    }

    /// Look up block state info by hash.
    pub fn get(&self, hash: u32) -> Option<&BlockStateInfo> {
        self.entries.get(&hash)
    }

    /// Number of registered block states.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Serialize a block state hash to BDS little-endian NBT compound.
    ///
    /// BDS format: TAG_Compound with "name" (String), "states" (Compound), "version" (Int),
    /// using standard LE NBT (i16_le string lengths, i32_le for ints).
    pub fn hash_to_nbt_le(&self, hash: u32) -> Option<Vec<u8>> {
        let info = self.entries.get(&hash)?;
        Some(serialize_bds_nbt_le(&info.name, &info.properties))
    }

    /// Parse a BDS little-endian NBT block state compound and compute its FNV-1a hash.
    ///
    /// This extracts "name" and "states" from the LE NBT, then re-serializes
    /// to network NBT format and runs FNV-1a to produce the hash our server uses.
    pub fn nbt_le_to_hash(nbt_data: &[u8]) -> Option<u32> {
        let (name, props) = parse_bds_nbt_le(nbt_data)?;
        if props.is_empty() {
            Some(hash_block_state(&name))
        } else {
            let prop_refs: Vec<(&str, StateValue)> = props
                .iter()
                .map(|(k, v)| {
                    let sv = match v {
                        StateValueOwned::Int(i) => StateValue::Int(*i),
                        StateValueOwned::Byte(b) => StateValue::Byte(*b),
                        StateValueOwned::Str(s) => StateValue::Str(s.as_str()),
                    };
                    (k.as_str(), sv)
                })
                .collect();
            Some(hash_block_state_with_props(&name, &prop_refs))
        }
    }

    // --- Internal helpers ---

    fn register_simple(&mut self, name: &str) {
        let hash = hash_block_state(name);
        self.entries.insert(
            hash,
            BlockStateInfo {
                name: name.to_string(),
                properties: Vec::new(),
            },
        );
    }

    fn register_int(&mut self, name: &str, prop: &str, value: i32) {
        let hash = hash_block_state_with_int(name, prop, value);
        self.entries.insert(
            hash,
            BlockStateInfo {
                name: name.to_string(),
                properties: vec![(prop.to_string(), StateValueOwned::Int(value))],
            },
        );
    }

    fn register_props(&mut self, name: &str, props: &[(&str, StateValue)]) {
        let hash = hash_block_state_with_props(name, props);
        let owned: Vec<(String, StateValueOwned)> = props
            .iter()
            .map(|(k, v)| {
                let ov = match v {
                    StateValue::Int(i) => StateValueOwned::Int(*i),
                    StateValue::Byte(b) => StateValueOwned::Byte(*b),
                    StateValue::Str(s) => StateValueOwned::Str(s.to_string()),
                };
                (k.to_string(), ov)
            })
            .collect();
        self.entries.insert(
            hash,
            BlockStateInfo {
                name: name.to_string(),
                properties: owned,
            },
        );
    }
}

impl Default for BlockStateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// BDS LE NBT serialization (for export)
// ---------------------------------------------------------------------------

/// Serialize a block state to BDS little-endian NBT compound.
///
/// Format: standard LE NBT with i16_le string lengths and i32_le for TAG_Int.
fn serialize_bds_nbt_le(name: &str, props: &[(String, StateValueOwned)]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);

    // Root TAG_Compound with empty name
    buf.push(0x0A);
    write_le_string(&mut buf, "");

    // "name" -> TAG_String
    buf.push(0x08);
    write_le_string(&mut buf, "name");
    write_le_string(&mut buf, name);

    // "states" -> TAG_Compound
    buf.push(0x0A);
    write_le_string(&mut buf, "states");

    // Sort properties alphabetically (BDS order)
    let mut sorted: Vec<_> = props.iter().collect();
    sorted.sort_by_key(|(k, _)| k.as_str());

    for (prop_name, value) in &sorted {
        match value {
            StateValueOwned::Int(v) => {
                buf.push(0x03); // TAG_Int
                write_le_string(&mut buf, prop_name);
                buf.extend_from_slice(&v.to_le_bytes());
            }
            StateValueOwned::Byte(v) => {
                buf.push(0x01); // TAG_Byte
                write_le_string(&mut buf, prop_name);
                buf.push(*v as u8);
            }
            StateValueOwned::Str(v) => {
                buf.push(0x08); // TAG_String
                write_le_string(&mut buf, prop_name);
                write_le_string(&mut buf, v);
            }
        }
    }
    buf.push(0x00); // TAG_End for states

    // "version" -> TAG_Int
    buf.push(0x03);
    write_le_string(&mut buf, "version");
    buf.extend_from_slice(&BLOCK_STATE_VERSION.to_le_bytes());

    // TAG_End for root
    buf.push(0x00);

    buf
}

/// Write a LE NBT string: i16_le(length) + UTF-8 bytes.
fn write_le_string(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(&(s.len() as i16).to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
}

// ---------------------------------------------------------------------------
// BDS LE NBT parsing (for import)
// ---------------------------------------------------------------------------

/// Parse a BDS LE NBT block state compound, extracting name and state properties.
fn parse_bds_nbt_le(data: &[u8]) -> Option<(String, Vec<(String, StateValueOwned)>)> {
    let mut cursor = 0usize;
    let mut name = String::new();
    let mut props = Vec::new();

    // Root TAG_Compound (0x0A)
    if *data.get(cursor)? != 0x0A {
        return None;
    }
    cursor += 1;

    // Root compound name (skip)
    let root_name_len = read_le_i16(data, &mut cursor)? as usize;
    cursor += root_name_len;

    // Read entries until TAG_End
    loop {
        let tag_type = *data.get(cursor)?;
        cursor += 1;

        if tag_type == 0x00 {
            break; // TAG_End
        }

        let key = read_le_string(data, &mut cursor)?;

        match tag_type {
            0x08 => {
                // TAG_String
                let val = read_le_string(data, &mut cursor)?;
                if key == "name" {
                    name = val;
                }
            }
            0x03 => {
                // TAG_Int
                let _val = read_le_i32(data, &mut cursor)?;
                // Skip "version" field (i32 at root level)
            }
            0x0A => {
                // TAG_Compound
                if key == "states" {
                    props = parse_states_compound(data, &mut cursor)?;
                } else {
                    skip_compound(data, &mut cursor)?;
                }
            }
            _ => {
                // Unknown tag type at root level — skip
                skip_tag_value(tag_type, data, &mut cursor)?;
            }
        }
    }

    if name.is_empty() {
        return None;
    }

    Some((name, props))
}

/// Parse the "states" compound entries.
fn parse_states_compound(
    data: &[u8],
    cursor: &mut usize,
) -> Option<Vec<(String, StateValueOwned)>> {
    let mut props = Vec::new();

    loop {
        let tag_type = *data.get(*cursor)?;
        *cursor += 1;

        if tag_type == 0x00 {
            break; // TAG_End
        }

        let key = read_le_string(data, cursor)?;

        match tag_type {
            0x01 => {
                // TAG_Byte
                let val = *data.get(*cursor)? as i8;
                *cursor += 1;
                props.push((key, StateValueOwned::Byte(val)));
            }
            0x03 => {
                // TAG_Int
                let val = read_le_i32(data, cursor)?;
                props.push((key, StateValueOwned::Int(val)));
            }
            0x08 => {
                // TAG_String
                let val = read_le_string(data, cursor)?;
                props.push((key, StateValueOwned::Str(val)));
            }
            _ => {
                skip_tag_value(tag_type, data, cursor)?;
            }
        }
    }

    Some(props)
}

/// Read i16 LE from data at cursor, advance cursor.
fn read_le_i16(data: &[u8], cursor: &mut usize) -> Option<i16> {
    if *cursor + 2 > data.len() {
        return None;
    }
    let val = i16::from_le_bytes([data[*cursor], data[*cursor + 1]]);
    *cursor += 2;
    Some(val)
}

/// Read i32 LE from data at cursor, advance cursor.
fn read_le_i32(data: &[u8], cursor: &mut usize) -> Option<i32> {
    if *cursor + 4 > data.len() {
        return None;
    }
    let val = i32::from_le_bytes([
        data[*cursor],
        data[*cursor + 1],
        data[*cursor + 2],
        data[*cursor + 3],
    ]);
    *cursor += 4;
    Some(val)
}

/// Read an LE NBT string: i16_le(len) + UTF-8 bytes.
fn read_le_string(data: &[u8], cursor: &mut usize) -> Option<String> {
    let len = read_le_i16(data, cursor)? as usize;
    if *cursor + len > data.len() {
        return None;
    }
    let s = String::from_utf8(data[*cursor..*cursor + len].to_vec()).ok()?;
    *cursor += len;
    Some(s)
}

/// Skip a TAG_Compound body (read until TAG_End).
fn skip_compound(data: &[u8], cursor: &mut usize) -> Option<()> {
    loop {
        let tag_type = *data.get(*cursor)?;
        *cursor += 1;
        if tag_type == 0x00 {
            return Some(());
        }
        // Skip key name
        let key_len = read_le_i16(data, cursor)? as usize;
        *cursor += key_len;
        // Skip value
        skip_tag_value(tag_type, data, cursor)?;
    }
}

/// Skip a tag value based on its type ID.
fn skip_tag_value(tag_type: u8, data: &[u8], cursor: &mut usize) -> Option<()> {
    match tag_type {
        0x01 => *cursor += 1, // TAG_Byte
        0x02 => *cursor += 2, // TAG_Short
        0x03 => *cursor += 4, // TAG_Int
        0x04 => *cursor += 8, // TAG_Long
        0x05 => *cursor += 4, // TAG_Float
        0x06 => *cursor += 8, // TAG_Double
        0x07 => {
            // TAG_Byte_Array
            let len = read_le_i32(data, cursor)? as usize;
            *cursor += len;
        }
        0x08 => {
            // TAG_String
            let len = read_le_i16(data, cursor)? as usize;
            *cursor += len;
        }
        0x09 => {
            // TAG_List
            let elem_type = *data.get(*cursor)?;
            *cursor += 1;
            let count = read_le_i32(data, cursor)?;
            for _ in 0..count {
                skip_tag_value(elem_type, data, cursor)?;
            }
        }
        0x0A => {
            // TAG_Compound
            skip_compound(data, cursor)?;
        }
        0x0B => {
            // TAG_Int_Array
            let len = read_le_i32(data, cursor)? as usize;
            *cursor += len * 4;
        }
        0x0C => {
            // TAG_Long_Array
            let len = read_le_i32(data, cursor)? as usize;
            *cursor += len * 8;
        }
        _ => return None,
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_hash::{
        hash_block_state, hash_block_state_with_int, hash_block_state_with_props, StateValue,
    };

    #[test]
    fn registry_not_empty() {
        let reg = BlockStateRegistry::new();
        assert!(reg.len() > 300, "expected 300+ entries, got {}", reg.len());
    }

    #[test]
    fn lookup_air() {
        let reg = BlockStateRegistry::new();
        let air_hash = hash_block_state("minecraft:air");
        let info = reg.get(air_hash).expect("air must be registered");
        assert_eq!(info.name, "minecraft:air");
        assert!(info.properties.is_empty());
    }

    #[test]
    fn lookup_stone() {
        let reg = BlockStateRegistry::new();
        let hash = hash_block_state("minecraft:stone");
        let info = reg.get(hash).expect("stone must be registered");
        assert_eq!(info.name, "minecraft:stone");
        assert!(info.properties.is_empty());
    }

    #[test]
    fn lookup_wheat_growth_3() {
        let reg = BlockStateRegistry::new();
        let hash = hash_block_state_with_int("minecraft:wheat", "growth", 3);
        let info = reg.get(hash).expect("wheat growth=3 must be registered");
        assert_eq!(info.name, "minecraft:wheat");
        assert_eq!(info.properties.len(), 1);
        assert_eq!(info.properties[0].0, "growth");
        assert_eq!(info.properties[0].1, StateValueOwned::Int(3));
    }

    #[test]
    fn lookup_water_depth_7() {
        let reg = BlockStateRegistry::new();
        let hash = hash_block_state_with_int("minecraft:water", "liquid_depth", 7);
        let info = reg.get(hash).expect("water depth=7 must be registered");
        assert_eq!(info.name, "minecraft:water");
        assert_eq!(info.properties[0].1, StateValueOwned::Int(7));
    }

    #[test]
    fn lookup_lever_multi_props() {
        let reg = BlockStateRegistry::new();
        let hash = hash_block_state_with_props(
            "minecraft:lever",
            &[
                ("lever_direction", StateValue::Str("north")),
                ("open_bit", StateValue::Byte(1)),
            ],
        );
        let info = reg.get(hash).expect("lever north/open must be registered");
        assert_eq!(info.name, "minecraft:lever");
        assert_eq!(info.properties.len(), 2);
    }

    #[test]
    fn lookup_furnace_cardinal() {
        let reg = BlockStateRegistry::new();
        let hash = hash_block_state_with_props(
            "minecraft:furnace",
            &[("minecraft:cardinal_direction", StateValue::Str("east"))],
        );
        let info = reg.get(hash).expect("furnace east must be registered");
        assert_eq!(info.name, "minecraft:furnace");
    }

    #[test]
    fn lookup_anvil() {
        let reg = BlockStateRegistry::new();
        let hash = hash_block_state_with_props(
            "minecraft:anvil",
            &[
                ("damage", StateValue::Str("undamaged")),
                ("minecraft:cardinal_direction", StateValue::Str("north")),
            ],
        );
        let info = reg.get(hash).expect("anvil must be registered");
        assert_eq!(info.name, "minecraft:anvil");
    }

    #[test]
    fn lookup_piston() {
        let reg = BlockStateRegistry::new();
        let hash = hash_block_state_with_int("minecraft:piston", "facing_direction", 1);
        let info = reg.get(hash).expect("piston facing=1 must be registered");
        assert_eq!(info.name, "minecraft:piston");
        assert_eq!(info.properties[0].1, StateValueOwned::Int(1));
    }

    #[test]
    fn lookup_grindstone() {
        let reg = BlockStateRegistry::new();
        let hash = hash_block_state_with_props(
            "minecraft:grindstone",
            &[
                ("attachment", StateValue::Str("standing")),
                ("direction", StateValue::Int(2)),
            ],
        );
        let info = reg.get(hash).expect("grindstone must be registered");
        assert_eq!(info.name, "minecraft:grindstone");
    }

    #[test]
    fn hash_to_nbt_le_roundtrip() {
        let reg = BlockStateRegistry::new();
        let stone_hash = hash_block_state("minecraft:stone");

        let nbt_le = reg.hash_to_nbt_le(stone_hash).expect("must serialize");
        assert!(!nbt_le.is_empty());
        assert_eq!(nbt_le[0], 0x0A, "must start with TAG_Compound");

        // Parse back and re-hash
        let recovered = BlockStateRegistry::nbt_le_to_hash(&nbt_le).expect("must parse");
        assert_eq!(recovered, stone_hash);
    }

    #[test]
    fn nbt_le_roundtrip_with_props() {
        let reg = BlockStateRegistry::new();
        let hash = hash_block_state_with_int("minecraft:wheat", "growth", 5);

        let nbt_le = reg.hash_to_nbt_le(hash).expect("must serialize");
        let recovered = BlockStateRegistry::nbt_le_to_hash(&nbt_le).expect("must parse");
        assert_eq!(recovered, hash);
    }

    #[test]
    fn nbt_le_roundtrip_multi_props() {
        let reg = BlockStateRegistry::new();
        let hash = hash_block_state_with_props(
            "minecraft:lever",
            &[
                ("lever_direction", StateValue::Str("east")),
                ("open_bit", StateValue::Byte(0)),
            ],
        );

        let nbt_le = reg.hash_to_nbt_le(hash).expect("must serialize");
        let recovered = BlockStateRegistry::nbt_le_to_hash(&nbt_le).expect("must parse");
        assert_eq!(recovered, hash);
    }

    #[test]
    fn nbt_le_to_hash_invalid_data() {
        assert!(BlockStateRegistry::nbt_le_to_hash(&[]).is_none());
        assert!(BlockStateRegistry::nbt_le_to_hash(&[0xFF]).is_none());
    }

    #[test]
    fn all_registered_roundtrip() {
        let reg = BlockStateRegistry::new();
        let mut failures = 0;
        for &hash in reg.entries.keys() {
            let nbt_le = reg.hash_to_nbt_le(hash).unwrap();
            let recovered = BlockStateRegistry::nbt_le_to_hash(&nbt_le).unwrap();
            if recovered != hash {
                failures += 1;
            }
        }
        assert_eq!(failures, 0, "all entries must roundtrip successfully");
    }
}
