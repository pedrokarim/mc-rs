use std::collections::BTreeMap;
use std::sync::OnceLock;

use bytes::{BufMut, BytesMut};
use serde::Deserialize;

use crate::codec::{write_signed_varint32, write_string, write_unsigned_varint32};

#[derive(Debug, Deserialize)]
struct ItemEntryJson {
    runtime_id: i16,
    component_based: bool,
    version: i32,
}

const ITEM_LIST_JSON: &str = include_str!("../../data/item_list.json");
const EMPTY_COMPOUND_NBT_NETWORK: [u8; 3] = [0x0A, 0x00, 0x00];

fn build_payload() -> BytesMut {
    let items: BTreeMap<String, ItemEntryJson> =
        serde_json::from_str(ITEM_LIST_JSON).expect("invalid item_list.json");

    let mut buf = BytesMut::with_capacity(96_000);
    write_unsigned_varint32(&mut buf, items.len() as u32);

    for (name, entry) in items {
        write_string(&mut buf, &name);
        buf.put_i16_le(entry.runtime_id);
        buf.put_u8(entry.component_based as u8);
        write_signed_varint32(&mut buf, entry.version);
        // PMMP uses CacheableNbt; when no component_nbt is provided it serializes an empty
        // network compound root here.
        buf.extend_from_slice(&EMPTY_COMPOUND_NBT_NETWORK);
    }

    buf
}

fn cached_payload() -> &'static BytesMut {
    static PAYLOAD: OnceLock<BytesMut> = OnceLock::new();
    PAYLOAD.get_or_init(build_payload)
}

pub fn encode_full() -> BytesMut {
    cached_payload().clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::read_unsigned_varint32;

    #[test]
    fn full_registry_is_not_empty() {
        let pkt = encode_full();
        let mut cursor = std::io::Cursor::new(&pkt[..]);
        let count = read_unsigned_varint32(&mut cursor).expect("valid varuint count");
        assert!(count > 1000, "expected PMMP-like item registry size");
    }
}
