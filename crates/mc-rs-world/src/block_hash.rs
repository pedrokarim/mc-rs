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
}
