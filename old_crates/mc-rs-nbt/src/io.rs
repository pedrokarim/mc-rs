//! Generic NBT read/write engine, parameterized by variant.

use bytes::{Buf, BufMut};

use crate::error::NbtError;
use crate::tag::{NbtCompound, NbtRoot, NbtTag};

/// Maximum nesting depth to prevent stack overflow.
const MAX_DEPTH: usize = 512;

/// Abstraction over the two NBT wire formats.
pub(crate) trait NbtVariant {
    fn write_int(buf: &mut impl BufMut, value: i32);
    fn read_int(buf: &mut impl Buf) -> Result<i32, NbtError>;

    fn write_array_len(buf: &mut impl BufMut, len: i32);
    fn read_array_len(buf: &mut impl Buf) -> Result<i32, NbtError>;

    fn write_string_len(buf: &mut impl BufMut, len: usize);
    fn read_string_len(buf: &mut impl Buf) -> Result<usize, NbtError>;
}

// -----------------------------------------------------------------------
// Reading
// -----------------------------------------------------------------------

pub(crate) fn read_nbt<V: NbtVariant>(buf: &mut impl Buf) -> Result<NbtRoot, NbtError> {
    if !buf.has_remaining() {
        return Err(NbtError::UnexpectedEof);
    }
    let tag_type = buf.get_u8();
    if tag_type != 10 {
        return Err(NbtError::ExpectedCompound { got: tag_type });
    }
    let name = read_string::<V>(buf)?;
    let compound = read_compound::<V>(buf, 0)?;
    Ok(NbtRoot { name, compound })
}

fn read_tag<V: NbtVariant>(
    buf: &mut impl Buf,
    tag_type: u8,
    depth: usize,
) -> Result<NbtTag, NbtError> {
    if depth > MAX_DEPTH {
        return Err(NbtError::NestingTooDeep { limit: MAX_DEPTH });
    }

    match tag_type {
        1 => {
            ensure_remaining(buf, 1)?;
            Ok(NbtTag::Byte(buf.get_i8()))
        }
        2 => {
            ensure_remaining(buf, 2)?;
            Ok(NbtTag::Short(buf.get_i16_le()))
        }
        3 => Ok(NbtTag::Int(V::read_int(buf)?)),
        4 => {
            ensure_remaining(buf, 8)?;
            Ok(NbtTag::Long(buf.get_i64_le()))
        }
        5 => {
            ensure_remaining(buf, 4)?;
            Ok(NbtTag::Float(buf.get_f32_le()))
        }
        6 => {
            ensure_remaining(buf, 8)?;
            Ok(NbtTag::Double(buf.get_f64_le()))
        }
        7 => {
            let len = V::read_array_len(buf)?;
            if len < 0 {
                return Err(NbtError::NegativeLength(len));
            }
            let len = len as usize;
            ensure_remaining(buf, len)?;
            let mut arr = Vec::with_capacity(len);
            for _ in 0..len {
                arr.push(buf.get_i8());
            }
            Ok(NbtTag::ByteArray(arr))
        }
        8 => Ok(NbtTag::String(read_string::<V>(buf)?)),
        9 => {
            ensure_remaining(buf, 1)?;
            let element_type = buf.get_u8();
            let len = V::read_array_len(buf)?;
            if len < 0 {
                return Err(NbtError::NegativeLength(len));
            }
            let len = len as usize;
            let mut list = Vec::with_capacity(len);
            for _ in 0..len {
                list.push(read_tag::<V>(buf, element_type, depth + 1)?);
            }
            Ok(NbtTag::List(list))
        }
        10 => Ok(NbtTag::Compound(read_compound::<V>(buf, depth + 1)?)),
        11 => {
            let len = V::read_array_len(buf)?;
            if len < 0 {
                return Err(NbtError::NegativeLength(len));
            }
            let len = len as usize;
            let mut arr = Vec::with_capacity(len);
            for _ in 0..len {
                arr.push(V::read_int(buf)?);
            }
            Ok(NbtTag::IntArray(arr))
        }
        12 => {
            let len = V::read_array_len(buf)?;
            if len < 0 {
                return Err(NbtError::NegativeLength(len));
            }
            let len = len as usize;
            let mut arr = Vec::with_capacity(len);
            for _ in 0..len {
                ensure_remaining(buf, 8)?;
                arr.push(buf.get_i64_le());
            }
            Ok(NbtTag::LongArray(arr))
        }
        _ => Err(NbtError::UnknownTagType(tag_type)),
    }
}

fn read_compound<V: NbtVariant>(buf: &mut impl Buf, depth: usize) -> Result<NbtCompound, NbtError> {
    if depth > MAX_DEPTH {
        return Err(NbtError::NestingTooDeep { limit: MAX_DEPTH });
    }
    let mut map = NbtCompound::new();
    loop {
        ensure_remaining(buf, 1)?;
        let tag_type = buf.get_u8();
        if tag_type == 0 {
            break; // TAG_End
        }
        let name = read_string::<V>(buf)?;
        let tag = read_tag::<V>(buf, tag_type, depth)?;
        map.insert(name, tag);
    }
    Ok(map)
}

fn read_string<V: NbtVariant>(buf: &mut impl Buf) -> Result<String, NbtError> {
    let len = V::read_string_len(buf)?;
    ensure_remaining(buf, len)?;
    let data = buf.copy_to_bytes(len);
    String::from_utf8(data.to_vec()).map_err(|_| NbtError::InvalidUtf8)
}

fn ensure_remaining(buf: &impl Buf, needed: usize) -> Result<(), NbtError> {
    if buf.remaining() < needed {
        Err(NbtError::UnexpectedEof)
    } else {
        Ok(())
    }
}

// -----------------------------------------------------------------------
// Writing
// -----------------------------------------------------------------------

pub(crate) fn write_nbt<V: NbtVariant>(buf: &mut impl BufMut, root: &NbtRoot) {
    buf.put_u8(10); // TAG_Compound
    write_string::<V>(buf, &root.name);
    write_compound::<V>(buf, &root.compound);
}

fn write_tag<V: NbtVariant>(buf: &mut impl BufMut, tag: &NbtTag) {
    match tag {
        NbtTag::Byte(v) => buf.put_i8(*v),
        NbtTag::Short(v) => buf.put_i16_le(*v),
        NbtTag::Int(v) => V::write_int(buf, *v),
        NbtTag::Long(v) => buf.put_i64_le(*v),
        NbtTag::Float(v) => buf.put_f32_le(*v),
        NbtTag::Double(v) => buf.put_f64_le(*v),
        NbtTag::ByteArray(arr) => {
            V::write_array_len(buf, arr.len() as i32);
            for &b in arr {
                buf.put_i8(b);
            }
        }
        NbtTag::String(s) => write_string::<V>(buf, s),
        NbtTag::List(list) => {
            if list.is_empty() {
                buf.put_u8(0); // TAG_End type for empty list
                V::write_array_len(buf, 0);
            } else {
                buf.put_u8(list[0].tag_type_id());
                V::write_array_len(buf, list.len() as i32);
                for item in list {
                    write_tag::<V>(buf, item);
                }
            }
        }
        NbtTag::Compound(map) => write_compound::<V>(buf, map),
        NbtTag::IntArray(arr) => {
            V::write_array_len(buf, arr.len() as i32);
            for &v in arr {
                V::write_int(buf, v);
            }
        }
        NbtTag::LongArray(arr) => {
            V::write_array_len(buf, arr.len() as i32);
            for &v in arr {
                buf.put_i64_le(v);
            }
        }
    }
}

fn write_compound<V: NbtVariant>(buf: &mut impl BufMut, map: &NbtCompound) {
    for (name, tag) in map {
        buf.put_u8(tag.tag_type_id());
        write_string::<V>(buf, name);
        write_tag::<V>(buf, tag);
    }
    buf.put_u8(0); // TAG_End
}

fn write_string<V: NbtVariant>(buf: &mut impl BufMut, s: &str) {
    V::write_string_len(buf, s.len());
    buf.put_slice(s.as_bytes());
}
