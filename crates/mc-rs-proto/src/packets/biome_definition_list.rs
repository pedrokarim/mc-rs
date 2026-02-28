//! BiomeDefinitionList (0x7B) -- Server -> Client.
//!
//! Protocol 924+ uses a structured binary format instead of raw NBT.
//! Layout:
//!   VarUInt32(definition_count)
//!   per definition:
//!     u16_le(name_index)  -- index into string table
//!     u16_le(id)          -- biome numeric ID
//!     f32_le(temperature)
//!     f32_le(downfall)
//!     f32_le(foliage_snow)
//!     f32_le(depth)
//!     f32_le(scale)
//!     u32_le(map_water_color) -- ARGB
//!     u8(rain)              -- bool
//!     Optional<tags>:  u8(present) + VarUInt32(count) + u16_le(tag_index)[]
//!     Optional<chunk_gen_data>: u8(0) (not present)
//!   VarUInt32(string_count)
//!   per string: VarUInt32(len) + UTF-8 bytes

use std::collections::BTreeMap;
use std::sync::OnceLock;

use bytes::BufMut;
use serde::Deserialize;

use crate::codec::{write_string, ProtoEncode};
use crate::types::VarUInt32;

// ---------------------------------------------------------------------------
// JSON data structures (from pmmp/BedrockData)
// ---------------------------------------------------------------------------

/// Color in ARGB components, as stored in biome_definitions.json.
#[derive(Debug, Deserialize)]
struct JsonColor {
    a: u8,
    r: u8,
    g: u8,
    b: u8,
}

impl JsonColor {
    /// Pack into ARGB u32 (same as Bedrock's Color::toARGB).
    fn to_argb(&self) -> u32 {
        (self.a as u32) << 24 | (self.r as u32) << 16 | (self.g as u32) << 8 | (self.b as u32)
    }
}

/// A single biome entry from biome_definitions.json.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonBiomeDef {
    temperature: f32,
    downfall: f32,
    foliage_snow: f32,
    depth: f32,
    scale: f32,
    #[serde(rename = "mapWaterColour")]
    map_water_colour: JsonColor,
    rain: bool,
    #[serde(default)]
    tags: Option<Vec<String>>,
    /// Always 65535 in the JSON; we use biome_id_map.json for real IDs.
    #[allow(dead_code)]
    id: u16,
}

// ---------------------------------------------------------------------------
// Pre-encoded canonical payload (built once)
// ---------------------------------------------------------------------------

/// JSON source files embedded at compile time.
const BIOME_DEFINITIONS_JSON: &str = include_str!("../../data/biome_definitions.json");
const BIOME_ID_MAP_JSON: &str = include_str!("../../data/biome_id_map.json");

/// Build the canonical payload bytes once.
fn build_canonical_payload() -> Vec<u8> {
    // Parse JSON data
    let defs: BTreeMap<String, JsonBiomeDef> =
        serde_json::from_str(BIOME_DEFINITIONS_JSON).expect("invalid biome_definitions.json");
    let id_map: BTreeMap<String, u16> =
        serde_json::from_str(BIOME_ID_MAP_JSON).expect("invalid biome_id_map.json");

    // Build string table with deduplication
    let mut string_table: Vec<String> = Vec::new();
    let mut string_index: BTreeMap<String, u16> = BTreeMap::new();

    let mut intern = |s: &str| -> u16 {
        if let Some(&idx) = string_index.get(s) {
            return idx;
        }
        let idx = string_table.len() as u16;
        string_table.push(s.to_string());
        string_index.insert(s.to_string(), idx);
        idx
    };

    // Pre-process: intern all strings and collect structured entries
    struct BiomeEntry {
        name_idx: u16,
        id: u16,
        temperature: f32,
        downfall: f32,
        foliage_snow: f32,
        depth: f32,
        scale: f32,
        map_water_color: u32,
        rain: bool,
        tag_indexes: Option<Vec<u16>>,
    }

    let mut entries: Vec<BiomeEntry> = Vec::new();

    for (name, def) in &defs {
        // Strip "minecraft:" prefix for the biome name used in the id_map
        let short_name = name.strip_prefix("minecraft:").unwrap_or(name.as_str());

        let name_idx = intern(name);
        let id = id_map.get(short_name).copied().unwrap_or(0xFFFF);

        let tag_indexes = def
            .tags
            .as_ref()
            .map(|tags| tags.iter().map(|t| intern(t)).collect::<Vec<u16>>());

        entries.push(BiomeEntry {
            name_idx,
            id,
            temperature: def.temperature,
            downfall: def.downfall,
            foliage_snow: def.foliage_snow,
            depth: def.depth,
            scale: def.scale,
            map_water_color: def.map_water_colour.to_argb(),
            rain: def.rain,
            tag_indexes,
        });
    }

    // Encode into binary
    let mut buf = Vec::with_capacity(8192);

    // Definition count
    VarUInt32(entries.len() as u32).proto_encode(&mut buf);

    for entry in &entries {
        // name_index: u16_le
        buf.put_u16_le(entry.name_idx);
        // id: u16_le
        buf.put_u16_le(entry.id);
        // temperature: f32_le
        buf.put_f32_le(entry.temperature);
        // downfall: f32_le
        buf.put_f32_le(entry.downfall);
        // foliage_snow: f32_le
        buf.put_f32_le(entry.foliage_snow);
        // depth: f32_le
        buf.put_f32_le(entry.depth);
        // scale: f32_le
        buf.put_f32_le(entry.scale);
        // map_water_color: u32_le (ARGB)
        buf.put_u32_le(entry.map_water_color);
        // rain: bool (u8)
        buf.put_u8(u8::from(entry.rain));

        // Optional<tags>
        match &entry.tag_indexes {
            Some(tags) => {
                buf.put_u8(1); // present
                VarUInt32(tags.len() as u32).proto_encode(&mut buf);
                for &idx in tags {
                    buf.put_u16_le(idx);
                }
            }
            None => {
                buf.put_u8(0); // not present
            }
        }

        // Optional<chunk_gen_data> -- not present (server-side only data)
        buf.put_u8(0);
    }

    // String table
    VarUInt32(string_table.len() as u32).proto_encode(&mut buf);
    for s in &string_table {
        write_string(&mut buf, s);
    }

    buf
}

/// Get the pre-built canonical payload (lazily initialized).
fn canonical_payload() -> &'static [u8] {
    static PAYLOAD: OnceLock<Vec<u8>> = OnceLock::new();
    PAYLOAD.get_or_init(build_canonical_payload)
}

// ---------------------------------------------------------------------------
// Packet type
// ---------------------------------------------------------------------------

/// BiomeDefinitionList packet in the new protocol 924+ binary format.
#[derive(Debug, Clone)]
pub struct BiomeDefinitionList {
    payload: &'static [u8],
}

impl BiomeDefinitionList {
    /// Create with the embedded canonical biome definitions.
    pub fn canonical() -> Self {
        Self {
            payload: canonical_payload(),
        }
    }
}

impl Default for BiomeDefinitionList {
    fn default() -> Self {
        Self::canonical()
    }
}

impl ProtoEncode for BiomeDefinitionList {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_slice(self.payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn canonical_payload_is_non_empty() {
        let pkt = BiomeDefinitionList::canonical();
        assert!(
            pkt.payload.len() > 100,
            "biome payload too small: {} bytes",
            pkt.payload.len()
        );
    }

    #[test]
    fn canonical_starts_with_varuint_count() {
        let payload = canonical_payload();
        // First byte(s) should be a VarUInt32 for the definition count.
        // 87 biomes = 0x57, fits in one byte.
        assert!(payload[0] > 0, "definition count should be non-zero");
    }

    #[test]
    fn encode_produces_output() {
        let pkt = BiomeDefinitionList::canonical();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf.len(), pkt.payload.len());
    }

    #[test]
    fn string_table_at_end() {
        // Verify we can decode the definition count and skip definitions
        // to find the string table.
        use crate::codec::ProtoDecode;
        use bytes::Buf;

        let payload = canonical_payload();
        let mut cursor = payload;

        // Read definition count
        let def_count = VarUInt32::proto_decode(&mut cursor).unwrap().0;
        assert!(def_count > 0, "should have biome definitions");

        // Skip all definitions
        for _ in 0..def_count {
            // name_index(2) + id(2) + temp(4) + downfall(4) + foliage(4)
            // + depth(4) + scale(4) + color(4) + rain(1) = 29 bytes fixed
            assert!(cursor.remaining() >= 29);
            cursor.advance(29);

            // Optional<tags>
            let tags_present = cursor.get_u8();
            if tags_present != 0 {
                let tag_count = VarUInt32::proto_decode(&mut cursor).unwrap().0;
                cursor.advance(tag_count as usize * 2); // u16_le each
            }

            // Optional<chunk_gen_data>
            let cgd_present = cursor.get_u8();
            assert_eq!(cgd_present, 0, "chunk_gen_data should be absent");
        }

        // Now we should be at the string table
        let str_count = VarUInt32::proto_decode(&mut cursor).unwrap().0;
        assert!(str_count > 0, "should have strings in the table");

        // Read all strings
        for _ in 0..str_count {
            let len = VarUInt32::proto_decode(&mut cursor).unwrap().0 as usize;
            assert!(cursor.remaining() >= len);
            cursor.advance(len);
        }

        // Should have consumed everything
        assert_eq!(
            cursor.remaining(),
            0,
            "payload should be fully consumed, {} bytes remain",
            cursor.remaining()
        );
    }

    #[test]
    fn biome_count_matches_json() {
        use crate::codec::ProtoDecode;

        let payload = canonical_payload();
        let mut cursor = payload;
        let def_count = VarUInt32::proto_decode(&mut cursor).unwrap().0;

        // biome_definitions.json has 87 entries
        assert_eq!(def_count, 87, "expected 87 biome definitions");
    }
}
