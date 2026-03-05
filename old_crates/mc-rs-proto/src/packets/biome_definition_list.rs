//! BiomeDefinitionList (0x7A) -- Server -> Client.
//!
//! Protocol 924 uses a structured binary format (NOT NBT).
//! Format: array of BiomeDefinitionData entries + string table.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use bytes::BufMut;
use serde::Deserialize;

use crate::codec::ProtoEncode;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonColor {
    a: u8,
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonBiomeDef {
    temperature: f32,
    downfall: f32,
    #[serde(default)]
    foliage_snow: f32,
    #[serde(default)]
    depth: f32,
    #[serde(default)]
    scale: f32,
    #[serde(rename = "mapWaterColour")]
    map_water_colour: JsonColor,
    rain: bool,
    #[serde(default)]
    tags: Option<Vec<String>>,
    id: u16,
}

const BIOME_DEFINITIONS_JSON: &str = include_str!("../../data/biome_definitions.json");

struct StringTable {
    strings: Vec<String>,
    lookup: BTreeMap<String, u16>,
}

impl StringTable {
    fn new() -> Self {
        Self {
            strings: Vec::new(),
            lookup: BTreeMap::new(),
        }
    }

    fn add(&mut self, s: &str) -> u16 {
        if let Some(&idx) = self.lookup.get(s) {
            return idx;
        }
        let idx = self.strings.len() as u16;
        self.strings.push(s.to_string());
        self.lookup.insert(s.to_string(), idx);
        idx
    }
}

fn build_canonical_payload() -> Vec<u8> {
    let defs: BTreeMap<String, JsonBiomeDef> =
        serde_json::from_str(BIOME_DEFINITIONS_JSON).expect("invalid biome_definitions.json");

    let mut string_table = StringTable::new();

    struct BiomeEntry {
        name_index: u16,
        id: u16,
        temperature: f32,
        downfall: f32,
        foliage_snow: f32,
        depth: f32,
        scale: f32,
        water_color_argb: u32,
        rain: bool,
        tag_indexes: Option<Vec<u16>>,
    }

    let mut entries: Vec<BiomeEntry> = Vec::with_capacity(defs.len());

    for (name, def) in &defs {
        let name_idx = string_table.add(name);
        let tag_indexes = def.tags.as_ref().map(|tags| {
            tags.iter().map(|t| string_table.add(t)).collect::<Vec<_>>()
        });

        let c = &def.map_water_colour;
        let argb = (c.a as u32) << 24 | (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32;

        entries.push(BiomeEntry {
            name_index: name_idx,
            id: def.id,
            temperature: def.temperature,
            downfall: def.downfall,
            foliage_snow: def.foliage_snow,
            depth: def.depth,
            scale: def.scale,
            water_color_argb: argb,
            rain: def.rain,
            tag_indexes,
        });
    }

    let mut buf = Vec::with_capacity(16384);

    write_varuint32(&mut buf, entries.len() as u32);
    for entry in &entries {
        buf.put_u16_le(entry.name_index);
        buf.put_u16_le(entry.id);
        buf.put_f32_le(entry.temperature);
        buf.put_f32_le(entry.downfall);
        buf.put_f32_le(entry.foliage_snow);
        buf.put_f32_le(entry.depth);
        buf.put_f32_le(entry.scale);
        buf.put_u32_le(entry.water_color_argb);
        buf.push(if entry.rain { 1 } else { 0 });

        // Optional<tags>
        match &entry.tag_indexes {
            Some(tags) => {
                buf.push(1); // present
                write_varuint32(&mut buf, tags.len() as u32);
                for &tag_idx in tags {
                    buf.put_u16_le(tag_idx);
                }
            }
            None => {
                buf.push(0); // absent
            }
        }

        // Optional<chunkGenData> = absent
        buf.push(0);
    }

    write_varuint32(&mut buf, string_table.strings.len() as u32);
    for s in &string_table.strings {
        write_varuint32(&mut buf, s.len() as u32);
        buf.extend_from_slice(s.as_bytes());
    }

    buf
}

fn write_varuint32(buf: &mut Vec<u8>, mut value: u32) {
    loop {
        if value & !0x7F == 0 {
            buf.push(value as u8);
            return;
        }
        buf.push((value & 0x7F | 0x80) as u8);
        value >>= 7;
    }
}

fn canonical_payload() -> &'static [u8] {
    static PAYLOAD: OnceLock<Vec<u8>> = OnceLock::new();
    PAYLOAD.get_or_init(build_canonical_payload)
}

/// BiomeDefinitionList packet — structured binary format (protocol 924).
#[derive(Debug, Clone)]
pub struct BiomeDefinitionList {
    payload: &'static [u8],
}

impl BiomeDefinitionList {
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

    fn decode_varuint32(data: &[u8]) -> (u32, usize) {
        let mut value = 0u32;
        let mut shift = 0;
        for (i, &b) in data.iter().enumerate() {
            value |= ((b & 0x7F) as u32) << shift;
            if b & 0x80 == 0 {
                return (value, i + 1);
            }
            shift += 7;
        }
        panic!("unterminated varint");
    }

    #[test]
    fn canonical_payload_starts_with_definition_count() {
        let payload = canonical_payload();
        let (count, _) = decode_varuint32(payload);
        assert!(count > 50, "should have many biome definitions, got {count}");
    }

    #[test]
    fn canonical_payload_is_not_empty() {
        let payload = canonical_payload();
        assert!(
            payload.len() > 100,
            "payload too small: {} bytes",
            payload.len()
        );
    }

    #[test]
    fn encode_produces_output() {
        let pkt = BiomeDefinitionList::canonical();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf.len(), pkt.payload.len());
    }

    #[test]
    fn string_table_contains_biome_names() {
        let payload = canonical_payload();

        let (def_count, mut offset) = decode_varuint32(payload);
        for _ in 0..def_count {
            offset += 2 + 2 + 4 * 5 + 4 + 1; // name_idx + id + 5 floats + color + rain

            // Optional<tags>
            let present = payload[offset];
            offset += 1;
            if present != 0 {
                let (tag_count, used) = decode_varuint32(&payload[offset..]);
                offset += used;
                offset += tag_count as usize * 2;
            }

            // Optional<chunkGenData>
            let present = payload[offset];
            offset += 1;
            assert_eq!(present, 0, "chunkGenData should be absent");
        }

        let (string_count, used) = decode_varuint32(&payload[offset..]);
        offset += used;
        assert!(
            string_count > 50,
            "should have many strings, got {string_count}"
        );

        let mut found_plains = false;
        for _ in 0..string_count {
            let (len, used) = decode_varuint32(&payload[offset..]);
            offset += used;
            let s = std::str::from_utf8(&payload[offset..offset + len as usize]).unwrap();
            if s == "minecraft:plains" {
                found_plains = true;
            }
            offset += len as usize;
        }
        assert!(found_plains, "string table should contain minecraft:plains");
        assert_eq!(offset, payload.len(), "payload should be fully consumed");
    }
}
