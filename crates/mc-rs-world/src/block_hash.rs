//! FNV-1a 32-bit block state hash computation.
//!
//! When `block_network_ids_are_hashes = true` in StartGame, the client computes
//! block runtime IDs as FNV-1a hashes of network-serialized block state NBT.
//! The server must produce identical hashes.

use bytes::{BufMut, BytesMut};

/// FNV-1a 32-bit offset basis.
const FNV1_32_INIT: u32 = 0x811c_9dc5;
/// FNV-1a 32-bit prime.
const FNV1_32_PRIME: u32 = 0x0100_0193;

/// Block state version for 1.21.50 protocol.
const BLOCK_STATE_VERSION: i32 = 18_100_737;

/// Compute FNV-1a 32-bit hash of a byte slice.
pub fn fnv1a_32(data: &[u8]) -> u32 {
    let mut hash = FNV1_32_INIT;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(FNV1_32_PRIME);
    }
    hash
}

/// Compute the block runtime ID (FNV-1a hash) for a block with empty states.
pub fn hash_block_state(name: &str) -> u32 {
    let nbt_bytes = serialize_block_state_nbt(name);
    fnv1a_32(&nbt_bytes)
}

/// Serialize a block state to network NBT bytes with deterministic key order.
///
/// Key order matches BDS: "name", "states", "version".
/// We serialize manually to guarantee deterministic output — the mc-rs-nbt
/// crate uses HashMap which has non-deterministic iteration order.
fn serialize_block_state_nbt(name: &str) -> Vec<u8> {
    let mut buf = BytesMut::new();

    // Root TAG_Compound with empty name
    buf.put_u8(0x0A);
    write_nbt_varuint_string(&mut buf, "");

    // "name" -> TAG_String
    buf.put_u8(0x08);
    write_nbt_varuint_string(&mut buf, "name");
    write_nbt_varuint_string(&mut buf, name);

    // "states" -> TAG_Compound (empty)
    buf.put_u8(0x0A);
    write_nbt_varuint_string(&mut buf, "states");
    buf.put_u8(0x00); // TAG_End

    // "version" -> TAG_Int (network NBT uses ZigZag VarInt for ints)
    buf.put_u8(0x03);
    write_nbt_varuint_string(&mut buf, "version");
    write_zigzag_varint(&mut buf, BLOCK_STATE_VERSION);

    // TAG_End for root compound
    buf.put_u8(0x00);

    buf.to_vec()
}

/// Write a network NBT string: VarUInt32(length) + UTF-8 bytes.
fn write_nbt_varuint_string(buf: &mut BytesMut, s: &str) {
    write_varuint32(buf, s.len() as u32);
    buf.put_slice(s.as_bytes());
}

/// Write unsigned VarInt (LEB128).
fn write_varuint32(buf: &mut BytesMut, mut value: u32) {
    loop {
        if value & !0x7F == 0 {
            buf.put_u8(value as u8);
            return;
        }
        buf.put_u8((value & 0x7F | 0x80) as u8);
        value >>= 7;
    }
}

/// Write signed VarInt (ZigZag + LEB128).
fn write_zigzag_varint(buf: &mut BytesMut, value: i32) {
    let encoded = ((value << 1) ^ (value >> 31)) as u32;
    write_varuint32(buf, encoded);
}

/// A block state property value for multi-property hashing.
#[derive(Debug, Clone)]
pub enum StateValue<'a> {
    /// TAG_Int (0x03) — numeric properties like `redstone_signal`, `direction`.
    Int(i32),
    /// TAG_Byte (0x01) — boolean-like properties like `open_bit`.
    Byte(i8),
    /// TAG_String (0x08) — string properties like `lever_direction`.
    Str(&'a str),
}

/// Compute the block runtime ID for a block with a single integer state property.
///
/// This produces the same hash the Bedrock client computes when
/// `block_network_ids_are_hashes = true`.  The NBT compound "states"
/// contains exactly one TAG_Int entry with the given property name and value.
pub fn hash_block_state_with_int(name: &str, prop_name: &str, value: i32) -> u32 {
    hash_block_state_with_props(name, &[(prop_name, StateValue::Int(value))])
}

/// Compute the block runtime ID for a block with multiple state properties.
///
/// Properties are sorted alphabetically by name inside the "states" compound,
/// matching the Bedrock client's serialization order.
pub fn hash_block_state_with_props(name: &str, props: &[(&str, StateValue)]) -> u32 {
    let nbt_bytes = serialize_block_state_nbt_with_props(name, props);
    fnv1a_32(&nbt_bytes)
}

/// Serialize a block state with multiple state properties to network NBT.
///
/// Key order inside root compound: "name", "states", "version".
/// Properties inside "states" are sorted alphabetically by name.
fn serialize_block_state_nbt_with_props(name: &str, props: &[(&str, StateValue)]) -> Vec<u8> {
    let mut buf = BytesMut::new();

    // Sort properties alphabetically
    let mut sorted: Vec<_> = props.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);

    // Root TAG_Compound with empty name
    buf.put_u8(0x0A);
    write_nbt_varuint_string(&mut buf, "");

    // "name" -> TAG_String
    buf.put_u8(0x08);
    write_nbt_varuint_string(&mut buf, "name");
    write_nbt_varuint_string(&mut buf, name);

    // "states" -> TAG_Compound with entries
    buf.put_u8(0x0A);
    write_nbt_varuint_string(&mut buf, "states");
    for (prop_name, value) in &sorted {
        match value {
            StateValue::Int(v) => {
                buf.put_u8(0x03); // TAG_Int
                write_nbt_varuint_string(&mut buf, prop_name);
                write_zigzag_varint(&mut buf, *v);
            }
            StateValue::Byte(v) => {
                buf.put_u8(0x01); // TAG_Byte
                write_nbt_varuint_string(&mut buf, prop_name);
                buf.put_i8(*v);
            }
            StateValue::Str(v) => {
                buf.put_u8(0x08); // TAG_String
                write_nbt_varuint_string(&mut buf, prop_name);
                write_nbt_varuint_string(&mut buf, v);
            }
        }
    }
    buf.put_u8(0x00); // TAG_End for states compound

    // "version" -> TAG_Int
    buf.put_u8(0x03);
    write_nbt_varuint_string(&mut buf, "version");
    write_zigzag_varint(&mut buf, BLOCK_STATE_VERSION);

    // TAG_End for root compound
    buf.put_u8(0x00);

    buf.to_vec()
}

/// Pre-computed block runtime IDs for the flat world.
#[derive(Debug, Clone)]
pub struct FlatWorldBlocks {
    pub air: u32,
    pub bedrock: u32,
    pub dirt: u32,
    pub grass_block: u32,
}

impl FlatWorldBlocks {
    /// Compute all block hashes needed for a flat world.
    pub fn compute() -> Self {
        Self {
            air: hash_block_state("minecraft:air"),
            bedrock: hash_block_state("minecraft:bedrock"),
            dirt: hash_block_state("minecraft:dirt"),
            grass_block: hash_block_state("minecraft:grass_block"),
        }
    }
}

/// Pre-computed block runtime IDs for the overworld generator.
#[derive(Debug, Clone)]
pub struct WorldBlocks {
    // Basics
    pub air: u32,
    pub bedrock: u32,
    pub dirt: u32,
    pub grass_block: u32,
    // Stone variants
    pub stone: u32,
    pub granite: u32,
    pub diorite: u32,
    pub andesite: u32,
    pub deepslate: u32,
    // Surface variants
    pub sand: u32,
    pub sandstone: u32,
    pub gravel: u32,
    pub clay: u32,
    pub snow_layer: u32,
    pub ice: u32,
    pub packed_ice: u32,
    // Liquids
    pub water: u32,
    pub lava: u32,
    // Ores
    pub coal_ore: u32,
    pub iron_ore: u32,
    pub gold_ore: u32,
    pub diamond_ore: u32,
    pub redstone_ore: u32,
    pub lapis_ore: u32,
    pub emerald_ore: u32,
    pub copper_ore: u32,
    // Deepslate ores
    pub deepslate_coal_ore: u32,
    pub deepslate_iron_ore: u32,
    pub deepslate_gold_ore: u32,
    pub deepslate_diamond_ore: u32,
    pub deepslate_redstone_ore: u32,
    pub deepslate_lapis_ore: u32,
    pub deepslate_emerald_ore: u32,
    pub deepslate_copper_ore: u32,
    // Trees
    pub oak_log: u32,
    pub oak_leaves: u32,
    pub birch_log: u32,
    pub birch_leaves: u32,
    pub spruce_log: u32,
    pub spruce_leaves: u32,
    pub acacia_log: u32,
    pub acacia_leaves: u32,
    // Vegetation
    pub tallgrass: u32,
    pub poppy: u32,
    pub dandelion: u32,
    pub dead_bush: u32,
    pub cactus: u32,
}

impl WorldBlocks {
    /// Compute all block hashes needed for overworld generation.
    pub fn compute() -> Self {
        Self {
            air: hash_block_state("minecraft:air"),
            bedrock: hash_block_state("minecraft:bedrock"),
            dirt: hash_block_state("minecraft:dirt"),
            grass_block: hash_block_state("minecraft:grass_block"),
            stone: hash_block_state("minecraft:stone"),
            granite: hash_block_state("minecraft:granite"),
            diorite: hash_block_state("minecraft:diorite"),
            andesite: hash_block_state("minecraft:andesite"),
            deepslate: hash_block_state("minecraft:deepslate"),
            sand: hash_block_state("minecraft:sand"),
            sandstone: hash_block_state("minecraft:sandstone"),
            gravel: hash_block_state("minecraft:gravel"),
            clay: hash_block_state("minecraft:clay"),
            snow_layer: hash_block_state("minecraft:snow_layer"),
            ice: hash_block_state("minecraft:ice"),
            packed_ice: hash_block_state("minecraft:packed_ice"),
            water: hash_block_state("minecraft:water"),
            lava: hash_block_state("minecraft:lava"),
            coal_ore: hash_block_state("minecraft:coal_ore"),
            iron_ore: hash_block_state("minecraft:iron_ore"),
            gold_ore: hash_block_state("minecraft:gold_ore"),
            diamond_ore: hash_block_state("minecraft:diamond_ore"),
            redstone_ore: hash_block_state("minecraft:redstone_ore"),
            lapis_ore: hash_block_state("minecraft:lapis_lazuli_ore"),
            emerald_ore: hash_block_state("minecraft:emerald_ore"),
            copper_ore: hash_block_state("minecraft:copper_ore"),
            deepslate_coal_ore: hash_block_state("minecraft:deepslate_coal_ore"),
            deepslate_iron_ore: hash_block_state("minecraft:deepslate_iron_ore"),
            deepslate_gold_ore: hash_block_state("minecraft:deepslate_gold_ore"),
            deepslate_diamond_ore: hash_block_state("minecraft:deepslate_diamond_ore"),
            deepslate_redstone_ore: hash_block_state("minecraft:deepslate_redstone_ore"),
            deepslate_lapis_ore: hash_block_state("minecraft:deepslate_lapis_lazuli_ore"),
            deepslate_emerald_ore: hash_block_state("minecraft:deepslate_emerald_ore"),
            deepslate_copper_ore: hash_block_state("minecraft:deepslate_copper_ore"),
            oak_log: hash_block_state("minecraft:oak_log"),
            oak_leaves: hash_block_state("minecraft:oak_leaves"),
            birch_log: hash_block_state("minecraft:birch_log"),
            birch_leaves: hash_block_state("minecraft:birch_leaves"),
            spruce_log: hash_block_state("minecraft:spruce_log"),
            spruce_leaves: hash_block_state("minecraft:spruce_leaves"),
            acacia_log: hash_block_state("minecraft:acacia_log"),
            acacia_leaves: hash_block_state("minecraft:acacia_leaves"),
            tallgrass: hash_block_state("minecraft:tallgrass"),
            poppy: hash_block_state("minecraft:red_flower"),
            dandelion: hash_block_state("minecraft:yellow_flower"),
            dead_bush: hash_block_state("minecraft:deadbush"),
            cactus: hash_block_state("minecraft:cactus"),
        }
    }

    /// Resolve a biome surface/filler block name to its pre-computed hash.
    pub fn by_name(&self, name: &str) -> u32 {
        match name {
            "minecraft:air" => self.air,
            "minecraft:bedrock" => self.bedrock,
            "minecraft:dirt" => self.dirt,
            "minecraft:grass_block" => self.grass_block,
            "minecraft:stone" => self.stone,
            "minecraft:sand" => self.sand,
            "minecraft:sandstone" => self.sandstone,
            "minecraft:gravel" => self.gravel,
            "minecraft:clay" => self.clay,
            "minecraft:snow_layer" => self.snow_layer,
            "minecraft:ice" => self.ice,
            "minecraft:water" => self.water,
            _ => self.air,
        }
    }
}

/// Lever direction string values (alphabetical order for indexing).
pub const LEVER_DIRS: [&str; 8] = [
    "down_east_west",
    "down_north_south",
    "east",
    "north",
    "south",
    "up_east_west",
    "up_north_south",
    "west",
];

/// Torch facing direction string values.
pub const TORCH_DIRS: [&str; 6] = ["east", "north", "south", "top", "unknown", "west"];

/// Pre-computed block runtime IDs for the tick system (random ticks, fluids, gravity, redstone).
#[derive(Debug, Clone)]
pub struct TickBlocks {
    pub air: u32,
    pub dirt: u32,
    pub grass_block: u32,
    // Crops (growth 0..N)
    pub wheat: [u32; 8],
    pub carrots: [u32; 8],
    pub potatoes: [u32; 8],
    pub beetroot: [u32; 4],
    pub farmland: [u32; 8],
    // Fluids (liquid_depth 0..15)
    pub water: [u32; 16],
    pub lava: [u32; 16],
    // Gravity blocks
    pub sand: u32,
    pub gravel: u32,
    pub red_sand: u32,
    // Leaves (for decay)
    pub oak_leaves: u32,
    pub birch_leaves: u32,
    pub spruce_leaves: u32,
    pub acacia_leaves: u32,
    // Logs (for leaf decay check)
    pub oak_log: u32,
    pub birch_log: u32,
    pub spruce_log: u32,
    pub acacia_log: u32,
    // Interaction products (for fluid phase)
    pub obsidian: u32,
    pub cobblestone: u32,
    pub stone: u32,
    // Redstone wire (redstone_signal 0..15)
    pub redstone_wire: [u32; 16],
    // Lever: [direction_idx][open_bit] — 8 dirs × 2 states
    pub lever: [[u32; 2]; 8],
    // Redstone torch: lit and unlit, indexed by direction
    pub torch_lit: [u32; 6],
    pub torch_unlit: [u32; 6],
    // Repeater: [direction][delay] — unpowered and powered
    pub repeater_off: [[u32; 4]; 4],
    pub repeater_on: [[u32; 4]; 4],
    // Redstone block (constant power source)
    pub redstone_block: u32,
}

impl TickBlocks {
    /// Compute all block hashes needed by the tick system.
    pub fn compute() -> Self {
        let mut wheat = [0u32; 8];
        let mut carrots = [0u32; 8];
        let mut potatoes = [0u32; 8];
        let mut beetroot = [0u32; 4];
        let mut farmland = [0u32; 8];
        let mut water = [0u32; 16];
        let mut lava = [0u32; 16];

        for (i, slot) in wheat.iter_mut().enumerate() {
            *slot = hash_block_state_with_int("minecraft:wheat", "growth", i as i32);
        }
        for (i, slot) in carrots.iter_mut().enumerate() {
            *slot = hash_block_state_with_int("minecraft:carrots", "growth", i as i32);
        }
        for (i, slot) in potatoes.iter_mut().enumerate() {
            *slot = hash_block_state_with_int("minecraft:potatoes", "growth", i as i32);
        }
        for (i, slot) in farmland.iter_mut().enumerate() {
            *slot = hash_block_state_with_int("minecraft:farmland", "moisturized_amount", i as i32);
        }
        for (i, slot) in beetroot.iter_mut().enumerate() {
            *slot = hash_block_state_with_int("minecraft:beetroot", "growth", i as i32);
        }
        for (i, slot) in water.iter_mut().enumerate() {
            *slot = hash_block_state_with_int("minecraft:water", "liquid_depth", i as i32);
        }
        for (i, slot) in lava.iter_mut().enumerate() {
            *slot = hash_block_state_with_int("minecraft:lava", "liquid_depth", i as i32);
        }

        // Redstone wire: redstone_signal 0..15
        let mut redstone_wire = [0u32; 16];
        for (i, slot) in redstone_wire.iter_mut().enumerate() {
            *slot =
                hash_block_state_with_int("minecraft:redstone_wire", "redstone_signal", i as i32);
        }

        // Lever: lever_direction (8 strings) × open_bit (0/1)
        let mut lever = [[0u32; 2]; 8];
        for (di, dir) in LEVER_DIRS.iter().enumerate() {
            for bit in 0..2i8 {
                lever[di][bit as usize] = hash_block_state_with_props(
                    "minecraft:lever",
                    &[
                        ("lever_direction", StateValue::Str(dir)),
                        ("open_bit", StateValue::Byte(bit)),
                    ],
                );
            }
        }

        // Redstone torch: torch_facing_direction (6 strings), lit and unlit
        let mut torch_lit = [0u32; 6];
        let mut torch_unlit = [0u32; 6];
        for (di, dir) in TORCH_DIRS.iter().enumerate() {
            torch_lit[di] = hash_block_state_with_props(
                "minecraft:redstone_torch",
                &[("torch_facing_direction", StateValue::Str(dir))],
            );
            torch_unlit[di] = hash_block_state_with_props(
                "minecraft:unlit_redstone_torch",
                &[("torch_facing_direction", StateValue::Str(dir))],
            );
        }

        // Repeater: direction (0-3) × repeater_delay (0-3), unpowered and powered
        let mut repeater_off = [[0u32; 4]; 4];
        let mut repeater_on = [[0u32; 4]; 4];
        for dir in 0..4 {
            for delay in 0..4 {
                repeater_off[dir][delay] = hash_block_state_with_props(
                    "minecraft:unpowered_repeater",
                    &[
                        ("direction", StateValue::Int(dir as i32)),
                        ("repeater_delay", StateValue::Int(delay as i32)),
                    ],
                );
                repeater_on[dir][delay] = hash_block_state_with_props(
                    "minecraft:powered_repeater",
                    &[
                        ("direction", StateValue::Int(dir as i32)),
                        ("repeater_delay", StateValue::Int(delay as i32)),
                    ],
                );
            }
        }

        Self {
            air: hash_block_state("minecraft:air"),
            dirt: hash_block_state("minecraft:dirt"),
            grass_block: hash_block_state("minecraft:grass_block"),
            wheat,
            carrots,
            potatoes,
            beetroot,
            farmland,
            water,
            lava,
            sand: hash_block_state("minecraft:sand"),
            gravel: hash_block_state("minecraft:gravel"),
            red_sand: hash_block_state("minecraft:red_sand"),
            oak_leaves: hash_block_state("minecraft:oak_leaves"),
            birch_leaves: hash_block_state("minecraft:birch_leaves"),
            spruce_leaves: hash_block_state("minecraft:spruce_leaves"),
            acacia_leaves: hash_block_state("minecraft:acacia_leaves"),
            oak_log: hash_block_state("minecraft:oak_log"),
            birch_log: hash_block_state("minecraft:birch_log"),
            spruce_log: hash_block_state("minecraft:spruce_log"),
            acacia_log: hash_block_state("minecraft:acacia_log"),
            obsidian: hash_block_state("minecraft:obsidian"),
            cobblestone: hash_block_state("minecraft:cobblestone"),
            stone: hash_block_state("minecraft:stone"),
            redstone_wire,
            lever,
            torch_lit,
            torch_unlit,
            repeater_off,
            repeater_on,
            redstone_block: hash_block_state("minecraft:redstone_block"),
        }
    }

    /// Check if a runtime ID is any leaf type.
    pub fn is_leaf(&self, rid: u32) -> bool {
        rid == self.oak_leaves
            || rid == self.birch_leaves
            || rid == self.spruce_leaves
            || rid == self.acacia_leaves
    }

    /// Check if a runtime ID is any log type.
    pub fn is_log(&self, rid: u32) -> bool {
        rid == self.oak_log
            || rid == self.birch_log
            || rid == self.spruce_log
            || rid == self.acacia_log
    }

    /// Find the growth stage of a crop block. Returns None if not a crop.
    pub fn crop_growth(&self, rid: u32) -> Option<(CropType, usize)> {
        for (i, &h) in self.wheat.iter().enumerate() {
            if h == rid {
                return Some((CropType::Wheat, i));
            }
        }
        for (i, &h) in self.carrots.iter().enumerate() {
            if h == rid {
                return Some((CropType::Carrots, i));
            }
        }
        for (i, &h) in self.potatoes.iter().enumerate() {
            if h == rid {
                return Some((CropType::Potatoes, i));
            }
        }
        for (i, &h) in self.beetroot.iter().enumerate() {
            if h == rid {
                return Some((CropType::Beetroot, i));
            }
        }
        None
    }

    /// Get the runtime ID for a crop at the given growth stage.
    pub fn crop_at_growth(&self, crop: CropType, growth: usize) -> u32 {
        match crop {
            CropType::Wheat => self.wheat[growth],
            CropType::Carrots => self.carrots[growth],
            CropType::Potatoes => self.potatoes[growth],
            CropType::Beetroot => self.beetroot[growth],
        }
    }

    /// Maximum growth stage for a crop type.
    pub fn crop_max_growth(crop: CropType) -> usize {
        match crop {
            CropType::Wheat | CropType::Carrots | CropType::Potatoes => 7,
            CropType::Beetroot => 3,
        }
    }

    /// Get the liquid_depth of a water block, or None if not water.
    pub fn water_depth(&self, rid: u32) -> Option<u8> {
        for (i, &h) in self.water.iter().enumerate() {
            if h == rid {
                return Some(i as u8);
            }
        }
        None
    }

    /// Get the liquid_depth of a lava block, or None if not lava.
    pub fn lava_depth(&self, rid: u32) -> Option<u8> {
        for (i, &h) in self.lava.iter().enumerate() {
            if h == rid {
                return Some(i as u8);
            }
        }
        None
    }

    /// Check if a runtime ID is any water block (any liquid_depth).
    pub fn is_water(&self, rid: u32) -> bool {
        self.water.contains(&rid)
    }

    /// Check if a runtime ID is any lava block (any liquid_depth).
    pub fn is_lava(&self, rid: u32) -> bool {
        self.lava.contains(&rid)
    }

    /// Check if a runtime ID is any fluid (water or lava).
    pub fn is_fluid(&self, rid: u32) -> bool {
        self.is_water(rid) || self.is_lava(rid)
    }

    /// Get the fluid type of a block, or None if not a fluid.
    pub fn fluid_type(&self, rid: u32) -> Option<FluidType> {
        if self.is_water(rid) {
            Some(FluidType::Water)
        } else if self.is_lava(rid) {
            Some(FluidType::Lava)
        } else {
            None
        }
    }

    /// Check if a runtime ID is a gravity-affected block (sand, gravel, red sand).
    pub fn is_gravity_block(&self, rid: u32) -> bool {
        rid == self.sand || rid == self.gravel || rid == self.red_sand
    }

    // -----------------------------------------------------------------------
    // Redstone helpers
    // -----------------------------------------------------------------------

    /// Check if a runtime ID is redstone wire (any signal level).
    pub fn is_wire(&self, rid: u32) -> bool {
        self.redstone_wire.contains(&rid)
    }

    /// Get the signal level of a redstone wire, or None if not wire.
    pub fn wire_signal(&self, rid: u32) -> Option<u8> {
        for (i, &h) in self.redstone_wire.iter().enumerate() {
            if h == rid {
                return Some(i as u8);
            }
        }
        None
    }

    /// Check if a runtime ID is any lever state.
    pub fn is_lever(&self, rid: u32) -> bool {
        self.lever
            .iter()
            .any(|pair| rid == pair[0] || rid == pair[1])
    }

    /// Check if a lever is in the ON state (open_bit=1).
    pub fn is_lever_on(&self, rid: u32) -> bool {
        self.lever.iter().any(|pair| rid == pair[1])
    }

    /// Toggle a lever: returns the toggled hash, or None if not a lever.
    pub fn toggle_lever(&self, rid: u32) -> Option<u32> {
        for pair in &self.lever {
            if rid == pair[0] {
                return Some(pair[1]);
            }
            if rid == pair[1] {
                return Some(pair[0]);
            }
        }
        None
    }

    /// Check if a runtime ID is a lit redstone torch.
    pub fn is_torch_lit(&self, rid: u32) -> bool {
        self.torch_lit.contains(&rid)
    }

    /// Check if a runtime ID is an unlit redstone torch.
    pub fn is_torch_unlit(&self, rid: u32) -> bool {
        self.torch_unlit.contains(&rid)
    }

    /// Check if a runtime ID is any redstone torch (lit or unlit).
    pub fn is_torch(&self, rid: u32) -> bool {
        self.is_torch_lit(rid) || self.is_torch_unlit(rid)
    }

    /// Toggle a torch between lit and unlit, preserving direction.
    pub fn toggle_torch(&self, rid: u32) -> Option<u32> {
        for (i, &h) in self.torch_lit.iter().enumerate() {
            if h == rid {
                return Some(self.torch_unlit[i]);
            }
        }
        for (i, &h) in self.torch_unlit.iter().enumerate() {
            if h == rid {
                return Some(self.torch_lit[i]);
            }
        }
        None
    }

    /// Get the direction index of a torch (index into TORCH_DIRS).
    pub fn torch_direction(&self, rid: u32) -> Option<usize> {
        for (i, &h) in self.torch_lit.iter().enumerate() {
            if h == rid {
                return Some(i);
            }
        }
        for (i, &h) in self.torch_unlit.iter().enumerate() {
            if h == rid {
                return Some(i);
            }
        }
        None
    }

    /// Check if a runtime ID is any repeater (powered or unpowered).
    pub fn is_repeater(&self, rid: u32) -> bool {
        self.repeater_off
            .iter()
            .flatten()
            .chain(self.repeater_on.iter().flatten())
            .any(|&h| h == rid)
    }

    /// Check if a repeater is in the powered state.
    pub fn is_repeater_powered(&self, rid: u32) -> bool {
        self.repeater_on.iter().flatten().any(|&h| h == rid)
    }

    /// Toggle a repeater between powered and unpowered, preserving direction and delay.
    pub fn toggle_repeater(&self, rid: u32) -> Option<u32> {
        for dir in 0..4 {
            for delay in 0..4 {
                if rid == self.repeater_off[dir][delay] {
                    return Some(self.repeater_on[dir][delay]);
                }
                if rid == self.repeater_on[dir][delay] {
                    return Some(self.repeater_off[dir][delay]);
                }
            }
        }
        None
    }

    /// Cycle a repeater's delay: 0→1→2→3→0, preserving direction and powered state.
    pub fn cycle_repeater_delay(&self, rid: u32) -> Option<u32> {
        for dir in 0..4 {
            for delay in 0..4 {
                let next = (delay + 1) % 4;
                if rid == self.repeater_off[dir][delay] {
                    return Some(self.repeater_off[dir][next]);
                }
                if rid == self.repeater_on[dir][delay] {
                    return Some(self.repeater_on[dir][next]);
                }
            }
        }
        None
    }

    /// Get the delay setting (0-3) of a repeater.
    pub fn repeater_delay(&self, rid: u32) -> Option<u8> {
        for dir in 0..4 {
            for delay in 0..4 {
                if rid == self.repeater_off[dir][delay] || rid == self.repeater_on[dir][delay] {
                    return Some(delay as u8);
                }
            }
        }
        None
    }

    /// Get the direction (0-3) of a repeater.
    pub fn repeater_direction(&self, rid: u32) -> Option<u8> {
        for dir in 0..4 {
            for delay in 0..4 {
                if rid == self.repeater_off[dir][delay] || rid == self.repeater_on[dir][delay] {
                    return Some(dir as u8);
                }
            }
        }
        None
    }

    /// Check if a block is a redstone power source (lever on, torch lit, redstone block).
    pub fn is_power_source(&self, rid: u32) -> bool {
        self.is_lever_on(rid) || self.is_torch_lit(rid) || rid == self.redstone_block
    }

    /// Get the power output of a block (15 for power sources, 0 otherwise).
    pub fn power_output(&self, rid: u32) -> u8 {
        if self.is_power_source(rid) {
            15
        } else {
            0
        }
    }

    /// Check if a block is any redstone component (wire, torch, or repeater).
    pub fn is_redstone_component(&self, rid: u32) -> bool {
        self.is_wire(rid) || self.is_torch(rid) || self.is_repeater(rid)
    }
}

/// Crop type for growth stage lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropType {
    Wheat,
    Carrots,
    Potatoes,
    Beetroot,
}

/// Fluid type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidType {
    Water,
    Lava,
}

/// Cardinal direction strings for furnace/blast_furnace/smoker (sorted alphabetically for hash).
const CARDINAL_DIRS: [&str; 4] = ["east", "north", "south", "west"];

/// Furnace variant type (mirrors mc_rs_game::smelting::FurnaceType without dep).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FurnaceVariant {
    Furnace,
    BlastFurnace,
    Smoker,
}

/// Pre-computed block runtime IDs for block entities (signs, chests, furnaces).
#[derive(Debug, Clone)]
pub struct BlockEntityHashes {
    /// Standing sign: `ground_sign_direction` 0-15.
    pub standing_sign: [u32; 16],
    /// Wall sign: `facing_direction` 2-5 (index 0-3 maps to face 2-5).
    pub wall_sign: [u32; 4],
    /// Chest: `facing_direction` 2-5 (index 0-3 maps to face 2-5).
    pub chest: [u32; 4],
    /// Furnace: `minecraft:cardinal_direction` east/north/south/west.
    pub furnace: [u32; 4],
    pub lit_furnace: [u32; 4],
    pub blast_furnace: [u32; 4],
    pub lit_blast_furnace: [u32; 4],
    pub smoker: [u32; 4],
    pub lit_smoker: [u32; 4],
    /// Enchanting table (no directional state).
    pub enchanting_table: u32,
    /// Bookshelf (no directional state).
    pub bookshelf: u32,
    /// Stonecutter: `minecraft:cardinal_direction` east/north/south/west.
    pub stonecutter: [u32; 4],
    /// Grindstone: `direction` (Int 0-3) × `attachment` (String: 4 values) = 16 hashes.
    pub grindstone: Vec<u32>,
    /// Loom: `direction` (Int 0-3).
    pub loom: [u32; 4],
    /// Anvil: `minecraft:cardinal_direction` (4 dirs) × `damage` (3 states) = 12 hashes.
    pub anvil: Vec<u32>,
}

impl BlockEntityHashes {
    /// Compute all block entity hashes.
    pub fn compute() -> Self {
        let mut standing_sign = [0u32; 16];
        for (i, hash) in standing_sign.iter_mut().enumerate() {
            *hash =
                hash_block_state_with_int("minecraft:oak_sign", "ground_sign_direction", i as i32);
        }

        let mut wall_sign = [0u32; 4];
        for (idx, face) in (2..=5).enumerate() {
            wall_sign[idx] =
                hash_block_state_with_int("minecraft:oak_wall_sign", "facing_direction", face);
        }

        let mut chest = [0u32; 4];
        for (idx, face) in (2..=5).enumerate() {
            chest[idx] = hash_block_state_with_int("minecraft:chest", "facing_direction", face);
        }

        let furnace_names = [
            ("minecraft:furnace", "minecraft:lit_furnace"),
            ("minecraft:blast_furnace", "minecraft:lit_blast_furnace"),
            ("minecraft:smoker", "minecraft:lit_smoker"),
        ];
        let mut furnace = [0u32; 4];
        let mut lit_furnace = [0u32; 4];
        let mut blast_furnace = [0u32; 4];
        let mut lit_blast_furnace = [0u32; 4];
        let mut smoker = [0u32; 4];
        let mut lit_smoker = [0u32; 4];

        let all_arrays = [
            (&mut furnace, &mut lit_furnace),
            (&mut blast_furnace, &mut lit_blast_furnace),
            (&mut smoker, &mut lit_smoker),
        ];

        for (i, ((unlit_arr, lit_arr), (unlit_name, lit_name))) in
            all_arrays.into_iter().zip(furnace_names.iter()).enumerate()
        {
            let _ = i;
            for (di, dir) in CARDINAL_DIRS.iter().enumerate() {
                unlit_arr[di] = hash_block_state_with_props(
                    unlit_name,
                    &[("minecraft:cardinal_direction", StateValue::Str(dir))],
                );
                lit_arr[di] = hash_block_state_with_props(
                    lit_name,
                    &[("minecraft:cardinal_direction", StateValue::Str(dir))],
                );
            }
        }

        let enchanting_table = hash_block_state("minecraft:enchanting_table");
        let bookshelf = hash_block_state("minecraft:bookshelf");

        // Stonecutter: uses minecraft:cardinal_direction (same as furnace)
        let mut stonecutter = [0u32; 4];
        for (di, dir) in CARDINAL_DIRS.iter().enumerate() {
            stonecutter[di] = hash_block_state_with_props(
                "minecraft:stonecutter_block",
                &[("minecraft:cardinal_direction", StateValue::Str(dir))],
            );
        }

        // Grindstone: direction (Int 0-3) × attachment (String: 4 values)
        let attachments = ["hanging", "multiple", "side", "standing"];
        let mut grindstone = Vec::with_capacity(16);
        for att in &attachments {
            for dir in 0..4i32 {
                grindstone.push(hash_block_state_with_props(
                    "minecraft:grindstone",
                    &[
                        ("attachment", StateValue::Str(att)),
                        ("direction", StateValue::Int(dir)),
                    ],
                ));
            }
        }

        // Loom: direction (Int 0-3)
        let mut loom = [0u32; 4];
        for dir in 0..4i32 {
            loom[dir as usize] = hash_block_state_with_int("minecraft:loom", "direction", dir);
        }

        // Anvil: minecraft:cardinal_direction (4 dirs) × damage (3 states)
        let damage_states = ["undamaged", "slightly_damaged", "very_damaged"];
        let mut anvil = Vec::with_capacity(12);
        for dmg in &damage_states {
            for dir in &CARDINAL_DIRS {
                anvil.push(hash_block_state_with_props(
                    "minecraft:anvil",
                    &[
                        ("damage", StateValue::Str(dmg)),
                        ("minecraft:cardinal_direction", StateValue::Str(dir)),
                    ],
                ));
            }
        }

        Self {
            standing_sign,
            wall_sign,
            chest,
            furnace,
            lit_furnace,
            blast_furnace,
            lit_blast_furnace,
            smoker,
            lit_smoker,
            enchanting_table,
            bookshelf,
            stonecutter,
            grindstone,
            loom,
            anvil,
        }
    }

    /// Check if a block runtime ID is any sign variant.
    pub fn is_sign(&self, rid: u32) -> bool {
        self.standing_sign.contains(&rid) || self.wall_sign.contains(&rid)
    }

    /// Check if a block runtime ID is a chest.
    pub fn is_chest(&self, rid: u32) -> bool {
        self.chest.contains(&rid)
    }

    /// Check if a block runtime ID is any furnace variant (lit or unlit).
    pub fn is_furnace(&self, rid: u32) -> bool {
        self.furnace.contains(&rid)
            || self.lit_furnace.contains(&rid)
            || self.blast_furnace.contains(&rid)
            || self.lit_blast_furnace.contains(&rid)
            || self.smoker.contains(&rid)
            || self.lit_smoker.contains(&rid)
    }

    /// Check if a block runtime ID is an enchanting table.
    pub fn is_enchanting_table(&self, rid: u32) -> bool {
        rid == self.enchanting_table
    }

    /// Check if a block runtime ID is a bookshelf.
    pub fn is_bookshelf(&self, rid: u32) -> bool {
        rid == self.bookshelf
    }

    /// Check if a block runtime ID is a stonecutter.
    pub fn is_stonecutter(&self, rid: u32) -> bool {
        self.stonecutter.contains(&rid)
    }

    /// Check if a block runtime ID is a grindstone.
    pub fn is_grindstone(&self, rid: u32) -> bool {
        self.grindstone.contains(&rid)
    }

    /// Check if a block runtime ID is a loom.
    pub fn is_loom(&self, rid: u32) -> bool {
        self.loom.contains(&rid)
    }

    /// Check if a block runtime ID is an anvil.
    pub fn is_anvil(&self, rid: u32) -> bool {
        self.anvil.contains(&rid)
    }

    /// Check if a block runtime ID is a lit furnace variant.
    pub fn is_lit_furnace(&self, rid: u32) -> bool {
        self.lit_furnace.contains(&rid)
            || self.lit_blast_furnace.contains(&rid)
            || self.lit_smoker.contains(&rid)
    }

    /// Get the furnace variant for a runtime ID.
    pub fn furnace_variant(&self, rid: u32) -> Option<FurnaceVariant> {
        if self.furnace.contains(&rid) || self.lit_furnace.contains(&rid) {
            Some(FurnaceVariant::Furnace)
        } else if self.blast_furnace.contains(&rid) || self.lit_blast_furnace.contains(&rid) {
            Some(FurnaceVariant::BlastFurnace)
        } else if self.smoker.contains(&rid) || self.lit_smoker.contains(&rid) {
            Some(FurnaceVariant::Smoker)
        } else {
            None
        }
    }

    /// Get the furnace hash for a player yaw (furnace faces the player).
    pub fn furnace_from_yaw(&self, variant: FurnaceVariant, yaw: f32, lit: bool) -> u32 {
        let y = yaw.rem_euclid(360.0);
        // cardinal_direction: east=0, north=1, south=2, west=3
        // Furnace faces opposite to player look direction
        let idx = if (315.0..360.0).contains(&y) || y < 45.0 {
            1 // north (player looks south → furnace faces north)
        } else if (45.0..135.0).contains(&y) {
            3 // west
        } else if (135.0..225.0).contains(&y) {
            2 // south
        } else {
            0 // east
        };
        let arr = match (variant, lit) {
            (FurnaceVariant::Furnace, false) => &self.furnace,
            (FurnaceVariant::Furnace, true) => &self.lit_furnace,
            (FurnaceVariant::BlastFurnace, false) => &self.blast_furnace,
            (FurnaceVariant::BlastFurnace, true) => &self.lit_blast_furnace,
            (FurnaceVariant::Smoker, false) => &self.smoker,
            (FurnaceVariant::Smoker, true) => &self.lit_smoker,
        };
        arr[idx]
    }

    /// Get the lit hash corresponding to an unlit furnace hash.
    pub fn lit_hash_for(&self, rid: u32) -> Option<u32> {
        for i in 0..4 {
            if self.furnace[i] == rid {
                return Some(self.lit_furnace[i]);
            }
            if self.blast_furnace[i] == rid {
                return Some(self.lit_blast_furnace[i]);
            }
            if self.smoker[i] == rid {
                return Some(self.lit_smoker[i]);
            }
        }
        None
    }

    /// Get the unlit hash corresponding to a lit furnace hash.
    pub fn unlit_hash_for(&self, rid: u32) -> Option<u32> {
        for i in 0..4 {
            if self.lit_furnace[i] == rid {
                return Some(self.furnace[i]);
            }
            if self.lit_blast_furnace[i] == rid {
                return Some(self.blast_furnace[i]);
            }
            if self.lit_smoker[i] == rid {
                return Some(self.smoker[i]);
            }
        }
        None
    }

    /// Get the standing sign hash for a given player yaw.
    /// Returns `(hash, direction_index)`.
    pub fn standing_sign_direction(&self, yaw: f32) -> (u32, i32) {
        let dir = (((yaw + 180.0) * 16.0 / 360.0).floor() as i32).rem_euclid(16);
        (self.standing_sign[dir as usize], dir)
    }

    /// Get the wall sign hash for a given face (2=north, 3=south, 4=west, 5=east).
    /// Returns `None` for invalid faces (top/bottom).
    pub fn wall_sign_face(&self, face: i32) -> Option<u32> {
        if (2..=5).contains(&face) {
            Some(self.wall_sign[(face - 2) as usize])
        } else {
            None
        }
    }

    /// Get the chest hash for a given facing direction (2-5).
    /// Returns `None` for invalid faces.
    pub fn chest_face(&self, face: i32) -> Option<u32> {
        if (2..=5).contains(&face) {
            Some(self.chest[(face - 2) as usize])
        } else {
            None
        }
    }

    /// Get the chest hash for a player yaw (used when placing from top).
    /// The chest faces the player.
    pub fn chest_from_yaw(&self, yaw: f32) -> u32 {
        // Normalize yaw to 0-360
        let y = yaw.rem_euclid(360.0);
        // facing_direction: 2=north(z-), 3=south(z+), 4=west(x-), 5=east(x+)
        // Chest faces the player, so we pick the direction the player is looking FROM
        let face = if (315.0..360.0).contains(&y) || y < 45.0 {
            2 // north
        } else if (45.0..135.0).contains(&y) {
            5 // east
        } else if (135.0..225.0).contains(&y) {
            3 // south
        } else {
            4 // west
        };
        self.chest[(face - 2) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_empty() {
        assert_eq!(fnv1a_32(&[]), FNV1_32_INIT);
    }

    #[test]
    fn fnv1a_known_vectors() {
        // FNV-1a test vectors from the spec
        assert_eq!(fnv1a_32(b""), 0x811c_9dc5);
        assert_eq!(fnv1a_32(b"a"), 0xe40c_292c);
        assert_eq!(fnv1a_32(b"foobar"), 0xbf9c_f968);
    }

    #[test]
    fn block_state_nbt_starts_with_compound() {
        let nbt = serialize_block_state_nbt("minecraft:air");
        assert_eq!(nbt[0], 0x0A, "should start with TAG_Compound");
    }

    #[test]
    fn block_state_nbt_contains_name() {
        let nbt = serialize_block_state_nbt("minecraft:air");
        let nbt_str = String::from_utf8_lossy(&nbt);
        assert!(nbt_str.contains("minecraft:air"));
    }

    #[test]
    fn all_flat_blocks_are_distinct() {
        let blocks = FlatWorldBlocks::compute();
        let hashes = [blocks.air, blocks.bedrock, blocks.dirt, blocks.grass_block];
        for i in 0..hashes.len() {
            for j in (i + 1)..hashes.len() {
                assert_ne!(
                    hashes[i], hashes[j],
                    "block hashes must be distinct: {} vs {}",
                    i, j
                );
            }
        }
    }

    #[test]
    fn hash_is_deterministic() {
        let h1 = hash_block_state("minecraft:stone");
        let h2 = hash_block_state("minecraft:stone");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_is_nonzero() {
        let blocks = FlatWorldBlocks::compute();
        assert_ne!(blocks.air, 0);
        assert_ne!(blocks.bedrock, 0);
        assert_ne!(blocks.dirt, 0);
        assert_ne!(blocks.grass_block, 0);
    }

    #[test]
    fn world_blocks_all_nonzero() {
        let wb = WorldBlocks::compute();
        // Spot-check a few critical blocks
        assert_ne!(wb.stone, 0);
        assert_ne!(wb.water, 0);
        assert_ne!(wb.lava, 0);
        assert_ne!(wb.oak_log, 0);
        assert_ne!(wb.diamond_ore, 0);
        assert_ne!(wb.deepslate, 0);
    }

    #[test]
    fn world_blocks_distinct_from_each_other() {
        let wb = WorldBlocks::compute();
        let hashes = [
            wb.air,
            wb.stone,
            wb.dirt,
            wb.grass_block,
            wb.sand,
            wb.water,
            wb.lava,
            wb.bedrock,
            wb.oak_log,
            wb.coal_ore,
            wb.iron_ore,
            wb.diamond_ore,
            wb.deepslate,
            wb.gravel,
        ];
        for i in 0..hashes.len() {
            for j in (i + 1)..hashes.len() {
                assert_ne!(hashes[i], hashes[j], "collision between index {i} and {j}");
            }
        }
    }

    #[test]
    fn world_blocks_by_name() {
        let wb = WorldBlocks::compute();
        assert_eq!(wb.by_name("minecraft:stone"), wb.stone);
        assert_eq!(wb.by_name("minecraft:sand"), wb.sand);
        assert_eq!(wb.by_name("minecraft:water"), wb.water);
        assert_eq!(wb.by_name("unknown"), wb.air);
    }

    #[test]
    fn world_blocks_compatible_with_flat() {
        let flat = FlatWorldBlocks::compute();
        let world = WorldBlocks::compute();
        assert_eq!(flat.air, world.air);
        assert_eq!(flat.bedrock, world.bedrock);
        assert_eq!(flat.dirt, world.dirt);
        assert_eq!(flat.grass_block, world.grass_block);
    }

    #[test]
    fn hash_with_int_differs_from_empty_states() {
        let empty = hash_block_state("minecraft:water");
        let with_state = hash_block_state_with_int("minecraft:water", "liquid_depth", 0);
        // With an explicit state property, the NBT is different from empty states
        assert_ne!(empty, with_state);
    }

    #[test]
    fn hash_with_int_is_deterministic() {
        let h1 = hash_block_state_with_int("minecraft:wheat", "growth", 3);
        let h2 = hash_block_state_with_int("minecraft:wheat", "growth", 3);
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_with_int_different_values_differ() {
        let h0 = hash_block_state_with_int("minecraft:wheat", "growth", 0);
        let h1 = hash_block_state_with_int("minecraft:wheat", "growth", 1);
        let h7 = hash_block_state_with_int("minecraft:wheat", "growth", 7);
        assert_ne!(h0, h1);
        assert_ne!(h0, h7);
        assert_ne!(h1, h7);
    }

    #[test]
    fn tick_blocks_all_nonzero() {
        let tb = TickBlocks::compute();
        assert_ne!(tb.air, 0);
        assert_ne!(tb.dirt, 0);
        assert_ne!(tb.grass_block, 0);
        assert_ne!(tb.sand, 0);
        assert_ne!(tb.obsidian, 0);
        for i in 0..8 {
            assert_ne!(tb.wheat[i], 0, "wheat[{i}]");
        }
        for i in 0..16 {
            assert_ne!(tb.water[i], 0, "water[{i}]");
            assert_ne!(tb.lava[i], 0, "lava[{i}]");
        }
    }

    #[test]
    fn tick_blocks_water_depths_all_distinct() {
        let tb = TickBlocks::compute();
        for i in 0..16 {
            for j in (i + 1)..16 {
                assert_ne!(tb.water[i], tb.water[j], "water[{i}] == water[{j}]");
            }
        }
    }

    #[test]
    fn tick_blocks_crop_growth_lookup() {
        let tb = TickBlocks::compute();
        // Wheat growth 3
        assert_eq!(tb.crop_growth(tb.wheat[3]), Some((CropType::Wheat, 3)));
        // Beetroot growth 2
        assert_eq!(
            tb.crop_growth(tb.beetroot[2]),
            Some((CropType::Beetroot, 2))
        );
        // Air is not a crop
        assert_eq!(tb.crop_growth(tb.air), None);
    }

    #[test]
    fn tick_blocks_is_leaf_and_log() {
        let tb = TickBlocks::compute();
        assert!(tb.is_leaf(tb.oak_leaves));
        assert!(tb.is_leaf(tb.birch_leaves));
        assert!(!tb.is_leaf(tb.oak_log));
        assert!(tb.is_log(tb.oak_log));
        assert!(tb.is_log(tb.spruce_log));
        assert!(!tb.is_log(tb.oak_leaves));
    }

    #[test]
    fn tick_blocks_compatible_with_world_blocks() {
        let tb = TickBlocks::compute();
        let wb = WorldBlocks::compute();
        assert_eq!(tb.air, wb.air);
        assert_eq!(tb.dirt, wb.dirt);
        assert_eq!(tb.grass_block, wb.grass_block);
        assert_eq!(tb.sand, wb.sand);
        assert_eq!(tb.gravel, wb.gravel);
        assert_eq!(tb.oak_log, wb.oak_log);
        assert_eq!(tb.oak_leaves, wb.oak_leaves);
        assert_eq!(tb.stone, wb.stone);
    }

    #[test]
    fn block_entity_hashes_nonzero() {
        let beh = BlockEntityHashes::compute();
        for h in &beh.standing_sign {
            assert_ne!(*h, 0);
        }
        for h in &beh.wall_sign {
            assert_ne!(*h, 0);
        }
        for h in &beh.chest {
            assert_ne!(*h, 0);
        }
    }

    #[test]
    fn block_entity_is_sign() {
        let beh = BlockEntityHashes::compute();
        assert!(beh.is_sign(beh.standing_sign[0]));
        assert!(beh.is_sign(beh.wall_sign[0]));
        assert!(!beh.is_sign(beh.chest[0]));
        assert!(!beh.is_sign(0));
    }

    #[test]
    fn block_entity_is_chest() {
        let beh = BlockEntityHashes::compute();
        assert!(beh.is_chest(beh.chest[0]));
        assert!(!beh.is_chest(beh.standing_sign[0]));
        assert!(!beh.is_chest(0));
    }

    #[test]
    fn standing_sign_direction_from_yaw() {
        let beh = BlockEntityHashes::compute();
        // yaw=0 (looking south) → direction should be 8 ((0+180)*16/360 = 8)
        let (hash, dir) = beh.standing_sign_direction(0.0);
        assert_eq!(dir, 8);
        assert_eq!(hash, beh.standing_sign[8]);
    }

    #[test]
    fn wall_sign_face_valid() {
        let beh = BlockEntityHashes::compute();
        assert!(beh.wall_sign_face(2).is_some());
        assert!(beh.wall_sign_face(5).is_some());
        assert!(beh.wall_sign_face(0).is_none());
        assert!(beh.wall_sign_face(1).is_none());
    }

    #[test]
    fn furnace_hashes_nonzero() {
        let beh = BlockEntityHashes::compute();
        for h in &beh.furnace {
            assert_ne!(*h, 0, "furnace hash");
        }
        for h in &beh.lit_furnace {
            assert_ne!(*h, 0, "lit_furnace hash");
        }
        for h in &beh.blast_furnace {
            assert_ne!(*h, 0, "blast_furnace hash");
        }
        for h in &beh.smoker {
            assert_ne!(*h, 0, "smoker hash");
        }
    }

    #[test]
    fn furnace_is_detected() {
        let beh = BlockEntityHashes::compute();
        assert!(beh.is_furnace(beh.furnace[0]));
        assert!(beh.is_furnace(beh.lit_furnace[1]));
        assert!(beh.is_furnace(beh.blast_furnace[2]));
        assert!(beh.is_furnace(beh.smoker[3]));
        assert!(!beh.is_furnace(beh.chest[0]));
        assert!(!beh.is_furnace(0));
    }

    #[test]
    fn furnace_variant_detection() {
        let beh = BlockEntityHashes::compute();
        assert_eq!(
            beh.furnace_variant(beh.furnace[0]),
            Some(FurnaceVariant::Furnace)
        );
        assert_eq!(
            beh.furnace_variant(beh.blast_furnace[1]),
            Some(FurnaceVariant::BlastFurnace)
        );
        assert_eq!(
            beh.furnace_variant(beh.lit_smoker[2]),
            Some(FurnaceVariant::Smoker)
        );
        assert_eq!(beh.furnace_variant(beh.chest[0]), None);
    }

    #[test]
    fn furnace_lit_unlit_conversion() {
        let beh = BlockEntityHashes::compute();
        for i in 0..4 {
            assert_eq!(beh.lit_hash_for(beh.furnace[i]), Some(beh.lit_furnace[i]));
            assert_eq!(beh.unlit_hash_for(beh.lit_furnace[i]), Some(beh.furnace[i]));
            assert_eq!(
                beh.lit_hash_for(beh.blast_furnace[i]),
                Some(beh.lit_blast_furnace[i])
            );
            assert_eq!(beh.unlit_hash_for(beh.lit_smoker[i]), Some(beh.smoker[i]));
        }
        assert_eq!(beh.lit_hash_for(beh.chest[0]), None);
    }

    #[test]
    fn enchanting_table_hash_nonzero() {
        let beh = BlockEntityHashes::compute();
        assert_ne!(beh.enchanting_table, 0);
        assert_ne!(beh.bookshelf, 0);
        assert_ne!(beh.enchanting_table, beh.bookshelf);
    }

    #[test]
    fn enchanting_table_detection() {
        let beh = BlockEntityHashes::compute();
        assert!(beh.is_enchanting_table(beh.enchanting_table));
        assert!(!beh.is_enchanting_table(beh.bookshelf));
        assert!(!beh.is_enchanting_table(beh.chest[0]));
        assert!(beh.is_bookshelf(beh.bookshelf));
        assert!(!beh.is_bookshelf(beh.enchanting_table));
    }

    #[test]
    fn stonecutter_detection() {
        let beh = BlockEntityHashes::compute();
        for &h in &beh.stonecutter {
            assert!(beh.is_stonecutter(h));
            assert!(!beh.is_chest(h));
        }
        assert!(!beh.is_stonecutter(beh.enchanting_table));
    }

    #[test]
    fn grindstone_detection() {
        let beh = BlockEntityHashes::compute();
        assert_eq!(beh.grindstone.len(), 16); // 4 dirs × 4 attachments
        for &h in &beh.grindstone {
            assert!(beh.is_grindstone(h));
            assert!(!beh.is_furnace(h));
        }
        assert!(!beh.is_grindstone(beh.enchanting_table));
    }

    #[test]
    fn loom_detection() {
        let beh = BlockEntityHashes::compute();
        for &h in &beh.loom {
            assert!(beh.is_loom(h));
            assert!(!beh.is_stonecutter(h));
        }
        assert!(!beh.is_loom(beh.enchanting_table));
    }

    #[test]
    fn anvil_detection() {
        let beh = BlockEntityHashes::compute();
        assert_eq!(beh.anvil.len(), 12); // 4 dirs × 3 damage states
        for &h in &beh.anvil {
            assert_ne!(h, 0);
            assert!(beh.is_anvil(h));
            assert!(!beh.is_stonecutter(h));
            assert!(!beh.is_loom(h));
        }
        assert!(!beh.is_anvil(beh.enchanting_table));
    }
}
