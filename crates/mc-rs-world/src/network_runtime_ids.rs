//! Mapping from internal block-state hashes to canonical Bedrock runtime IDs.
//!
//! mc-rs stores blocks as FNV-1a hashes of network block-state NBT. Bedrock
//! clients expect runtime IDs matching the canonical Bedrock palette order when
//! `StartGame.block_network_ids_are_hashes = false` (PocketMine behavior).

use std::collections::HashMap;
use std::sync::OnceLock;

use mc_rs_nbt::read_nbt_network;

use crate::block_hash::{
    fnv1a_32, hash_block_state, hash_default_bedrock, normalize_legacy_bedrock_hash,
    LEGACY_BEDROCK_HASH_EMPTY_STATES,
};

/// Canonical block states from BedrockData (same source used by PocketMine).
const CANONICAL_BLOCK_STATES_NBT: &[u8] =
    include_bytes!("../../../.reference/BedrockData/canonical_block_states.nbt");

static HASH_TO_RUNTIME_ID: OnceLock<HashMap<u32, u32>> = OnceLock::new();
static AIR_RUNTIME_ID: OnceLock<u32> = OnceLock::new();

/// Convert an internal block hash to a canonical Bedrock runtime ID.
///
/// Unknown hashes are mapped to air runtime ID as a safe fallback.
pub fn to_network_runtime_id(internal_hash: u32) -> u32 {
    let map = HASH_TO_RUNTIME_ID.get_or_init(build_hash_to_runtime_id_map);
    let normalized = normalize_legacy_bedrock_hash(internal_hash);
    if let Some(&rid) = map.get(&normalized) {
        return rid;
    }

    *AIR_RUNTIME_ID.get_or_init(|| {
        let air_hash = hash_block_state("minecraft:air");
        map.get(&air_hash).copied().unwrap_or(0)
    })
}

fn build_hash_to_runtime_id_map() -> HashMap<u32, u32> {
    let mut map: HashMap<u32, u32> = HashMap::with_capacity(16_384);

    let data = CANONICAL_BLOCK_STATES_NBT;
    let mut cursor = &data[..];
    let mut offset = 0usize;
    let mut runtime_id = 0u32;

    while !cursor.is_empty() {
        let before = cursor.len();
        if let Err(e) = read_nbt_network(&mut cursor) {
            panic!(
                "failed to parse canonical_block_states.nbt at entry {} (byte offset {}): {e}",
                runtime_id, offset
            );
        }
        let consumed = before.saturating_sub(cursor.len());
        if consumed == 0 {
            panic!(
                "failed to advance while parsing canonical_block_states.nbt at entry {}",
                runtime_id
            );
        }

        let hash = fnv1a_32(&data[offset..offset + consumed]);
        map.insert(hash, runtime_id);

        offset += consumed;
        runtime_id += 1;
    }

    if runtime_id == 0 {
        panic!("canonical_block_states.nbt produced zero entries");
    }

    // Keep compatibility with old persisted chunks that used bedrock empty-states hash.
    if let Some(&default_bedrock_rid) = map.get(&hash_default_bedrock()) {
        map.insert(LEGACY_BEDROCK_HASH_EMPTY_STATES, default_bedrock_rid);
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_hash::hash_block_state;

    #[test]
    fn canonical_hashes_are_mapped() {
        let air = hash_block_state("minecraft:air");
        let dirt = hash_block_state("minecraft:dirt");
        let grass = hash_block_state("minecraft:grass_block");
        let bedrock = hash_default_bedrock();

        let air_rid = to_network_runtime_id(air);
        let dirt_rid = to_network_runtime_id(dirt);
        let grass_rid = to_network_runtime_id(grass);
        let bedrock_rid = to_network_runtime_id(bedrock);

        assert_ne!(air_rid, dirt_rid);
        assert_ne!(air_rid, grass_rid);
        assert_ne!(air_rid, bedrock_rid);
    }

    #[test]
    fn legacy_bedrock_maps_to_default_bedrock_runtime_id() {
        let legacy = LEGACY_BEDROCK_HASH_EMPTY_STATES;
        let canonical = hash_default_bedrock();
        assert_eq!(
            to_network_runtime_id(legacy),
            to_network_runtime_id(canonical)
        );
    }
}
