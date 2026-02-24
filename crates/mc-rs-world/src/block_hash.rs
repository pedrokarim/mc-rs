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
}
