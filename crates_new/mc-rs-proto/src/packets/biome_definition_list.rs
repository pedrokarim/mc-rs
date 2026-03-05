use std::collections::BTreeMap;
use std::sync::OnceLock;

use bytes::{BufMut, BytesMut};
use serde::Deserialize;

use crate::codec::write_unsigned_varint32;

#[derive(Debug, Deserialize)]
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

const BIOME_DEFS_JSON: &str = include_str!("../../data/biome_definitions.json");

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

fn build_payload() -> Vec<u8> {
    let defs: BTreeMap<String, JsonBiomeDef> =
        serde_json::from_str(BIOME_DEFS_JSON).expect("invalid biome_definitions.json");

    let mut st = StringTable::new();

    struct Entry {
        name_idx: u16,
        id: u16,
        temp: f32,
        down: f32,
        snow: f32,
        depth: f32,
        scale: f32,
        water: u32,
        rain: bool,
        tags: Option<Vec<u16>>,
    }

    let mut entries = Vec::with_capacity(defs.len());
    for (name, def) in &defs {
        let ni = st.add(name);
        let ti = def
            .tags
            .as_ref()
            .map(|tags| tags.iter().map(|t| st.add(t)).collect());
        let c = &def.map_water_colour;
        let argb = (c.a as u32) << 24 | (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32;
        entries.push(Entry {
            name_idx: ni,
            id: def.id,
            temp: def.temperature,
            down: def.downfall,
            snow: def.foliage_snow,
            depth: def.depth,
            scale: def.scale,
            water: argb,
            rain: def.rain,
            tags: ti,
        });
    }

    let mut buf = Vec::with_capacity(16384);
    write_varuint32_vec(&mut buf, entries.len() as u32);
    for e in &entries {
        buf.put_u16_le(e.name_idx);
        buf.put_u16_le(e.id);
        buf.put_f32_le(e.temp);
        buf.put_f32_le(e.down);
        buf.put_f32_le(e.snow);
        buf.put_f32_le(e.depth);
        buf.put_f32_le(e.scale);
        buf.put_u32_le(e.water);
        buf.push(e.rain as u8);
        match &e.tags {
            Some(tags) => {
                buf.push(1);
                write_varuint32_vec(&mut buf, tags.len() as u32);
                for &t in tags {
                    buf.put_u16_le(t);
                }
            }
            None => buf.push(0),
        }
        buf.push(0); // chunkGenData = absent
    }
    write_varuint32_vec(&mut buf, st.strings.len() as u32);
    for s in &st.strings {
        write_varuint32_vec(&mut buf, s.len() as u32);
        buf.extend_from_slice(s.as_bytes());
    }
    buf
}

fn write_varuint32_vec(buf: &mut Vec<u8>, mut v: u32) {
    loop {
        if v & !0x7F == 0 {
            buf.push(v as u8);
            return;
        }
        buf.push((v & 0x7F | 0x80) as u8);
        v >>= 7;
    }
}

fn cached_payload() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    P.get_or_init(build_payload)
}

pub fn encode() -> BytesMut {
    BytesMut::from(cached_payload())
}
