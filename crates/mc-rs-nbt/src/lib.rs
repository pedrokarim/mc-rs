//! NBT (Named Binary Tag) implementation for Minecraft Bedrock Edition.
//!
//! Supports two variants:
//! - **Standard LE**: Used for disk storage and chunk data. Ints are i32_le, string lengths are u16_le.
//! - **Network**: Used in most game packets. Ints are VarInt (ZigZag), string lengths are VarUInt32.

pub mod error;
mod io;
mod le;
mod network;
pub mod tag;

pub use error::NbtError;
pub use tag::{NbtCompound, NbtRoot, NbtTag};

use bytes::{Buf, BufMut};

/// Read standard little-endian NBT from a buffer.
pub fn read_nbt_le(buf: &mut impl Buf) -> Result<NbtRoot, NbtError> {
    io::read_nbt::<le::LeVariant>(buf)
}

/// Write standard little-endian NBT to a buffer.
pub fn write_nbt_le(buf: &mut impl BufMut, root: &NbtRoot) {
    io::write_nbt::<le::LeVariant>(buf, root)
}

/// Read network NBT (VarInt variant) from a buffer.
pub fn read_nbt_network(buf: &mut impl Buf) -> Result<NbtRoot, NbtError> {
    io::read_nbt::<network::NetworkVariant>(buf)
}

/// Write network NBT (VarInt variant) to a buffer.
pub fn write_nbt_network(buf: &mut impl BufMut, root: &NbtRoot) {
    io::write_nbt::<network::NetworkVariant>(buf, root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    fn roundtrip_le(root: &NbtRoot) {
        let mut buf = BytesMut::new();
        write_nbt_le(&mut buf, root);
        let decoded = read_nbt_le(&mut buf.freeze()).unwrap();
        assert_eq!(decoded, *root);
    }

    fn roundtrip_network(root: &NbtRoot) {
        let mut buf = BytesMut::new();
        write_nbt_network(&mut buf, root);
        let decoded = read_nbt_network(&mut buf.freeze()).unwrap();
        assert_eq!(decoded, *root);
    }

    // -- Standard LE tests --

    #[test]
    fn le_empty_compound() {
        roundtrip_le(&NbtRoot::new("", NbtCompound::new()));
    }

    #[test]
    fn le_root_name() {
        roundtrip_le(&NbtRoot::new("hello world", NbtCompound::new()));
    }

    #[test]
    fn le_byte() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::Byte(42));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_short() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::Short(-1234));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_int() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::Int(100_000));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_long() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::Long(i64::MAX));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_float() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::Float(3.125));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_double() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::Double(std::f64::consts::PI));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_string() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::String("hello world".into()));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_string_unicode() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::String("日本語".into()));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_byte_array() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::ByteArray(vec![1, -2, 3, -4, 5]));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_int_array() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::IntArray(vec![100, -200, 300]));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_long_array() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::LongArray(vec![i64::MIN, 0, i64::MAX]));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_list_of_ints() {
        let mut c = NbtCompound::new();
        c.insert(
            "list".into(),
            NbtTag::List(vec![NbtTag::Int(1), NbtTag::Int(2), NbtTag::Int(3)]),
        );
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_empty_list() {
        let mut c = NbtCompound::new();
        c.insert("list".into(), NbtTag::List(vec![]));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_nested_compound() {
        let mut inner = NbtCompound::new();
        inner.insert("x".into(), NbtTag::Int(10));
        inner.insert("y".into(), NbtTag::Int(64));
        inner.insert("z".into(), NbtTag::Int(-10));

        let mut c = NbtCompound::new();
        c.insert("pos".into(), NbtTag::Compound(inner));
        roundtrip_le(&NbtRoot::new("", c));
    }

    #[test]
    fn le_complex_structure() {
        let mut inner = NbtCompound::new();
        inner.insert("name".into(), NbtTag::String("Steve".into()));
        inner.insert("health".into(), NbtTag::Float(20.0));
        inner.insert("xp".into(), NbtTag::Int(1500));

        let mut c = NbtCompound::new();
        c.insert("player".into(), NbtTag::Compound(inner));
        c.insert("version".into(), NbtTag::Int(19133));
        c.insert(
            "inventory".into(),
            NbtTag::List(vec![
                NbtTag::Compound({
                    let mut item = NbtCompound::new();
                    item.insert("id".into(), NbtTag::Short(1));
                    item.insert("count".into(), NbtTag::Byte(64));
                    item
                }),
                NbtTag::Compound({
                    let mut item = NbtCompound::new();
                    item.insert("id".into(), NbtTag::Short(4));
                    item.insert("count".into(), NbtTag::Byte(32));
                    item
                }),
            ]),
        );
        roundtrip_le(&NbtRoot::new("level", c));
    }

    // -- Network variant tests --

    #[test]
    fn network_empty_compound() {
        roundtrip_network(&NbtRoot::new("", NbtCompound::new()));
    }

    #[test]
    fn network_compound_with_int() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::Int(42));
        roundtrip_network(&NbtRoot::new("", c));
    }

    #[test]
    fn network_int_array() {
        let mut c = NbtCompound::new();
        c.insert("arr".into(), NbtTag::IntArray(vec![1, -2, 300]));
        roundtrip_network(&NbtRoot::new("", c));
    }

    #[test]
    fn network_complex() {
        let mut inner = NbtCompound::new();
        inner.insert("x".into(), NbtTag::Int(100));
        inner.insert("y".into(), NbtTag::Int(64));

        let mut c = NbtCompound::new();
        c.insert("pos".into(), NbtTag::Compound(inner));
        c.insert("name".into(), NbtTag::String("test".into()));
        c.insert("data".into(), NbtTag::ByteArray(vec![1, 2, 3]));
        roundtrip_network(&NbtRoot::new("", c));
    }

    #[test]
    fn network_vs_le_different_bytes() {
        let mut c = NbtCompound::new();
        c.insert("val".into(), NbtTag::Int(100));
        let root = NbtRoot::new("", c);

        let mut le_buf = BytesMut::new();
        write_nbt_le(&mut le_buf, &root);

        let mut net_buf = BytesMut::new();
        write_nbt_network(&mut net_buf, &root);

        // The binary representations should differ because:
        // - LE uses u16_le for string lengths, network uses VarUInt32
        // - LE uses i32_le for int, network uses VarInt (ZigZag)
        assert_ne!(le_buf, net_buf);
    }

    #[test]
    fn network_int_uses_varint() {
        // TAG_Int(1) in network variant should use VarInt encoding:
        // ZigZag(1) = 2, LEB128(2) = [0x02] = 1 byte
        // In LE it would be [0x01, 0x00, 0x00, 0x00] = 4 bytes
        let mut c = NbtCompound::new();
        c.insert("v".into(), NbtTag::Int(1));
        let root = NbtRoot::new("", c);

        let mut le_buf = BytesMut::new();
        write_nbt_le(&mut le_buf, &root);

        let mut net_buf = BytesMut::new();
        write_nbt_network(&mut net_buf, &root);

        // Network should be shorter (VarInt is 1 byte for small values vs 4 for i32_le)
        assert!(net_buf.len() < le_buf.len());
    }

    // -- Error cases --

    #[test]
    fn empty_buffer_error() {
        let data = bytes::Bytes::new();
        assert!(read_nbt_le(&mut data.clone()).is_err());
        assert!(read_nbt_network(&mut data.clone()).is_err());
    }

    #[test]
    fn wrong_root_type_error() {
        // TAG_Byte instead of TAG_Compound
        let data = bytes::Bytes::from_static(&[1]);
        assert!(matches!(
            read_nbt_le(&mut data.clone()),
            Err(NbtError::ExpectedCompound { got: 1 })
        ));
    }
}
