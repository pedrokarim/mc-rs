//! NBT tag types.

use std::collections::HashMap;
use std::fmt;

/// A compound tag: map of name -> tag.
pub type NbtCompound = HashMap<String, NbtTag>;

/// A named root compound (the root always has a name, often empty string).
#[derive(Debug, Clone, PartialEq)]
pub struct NbtRoot {
    pub name: String,
    pub compound: NbtCompound,
}

impl NbtRoot {
    pub fn new(name: impl Into<String>, compound: NbtCompound) -> Self {
        Self {
            name: name.into(),
            compound,
        }
    }
}

/// Represents any NBT value.
#[derive(Debug, Clone, PartialEq)]
pub enum NbtTag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<NbtTag>),
    Compound(NbtCompound),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl NbtTag {
    /// Returns the numeric tag type ID (0-12). TAG_End is 0 but not representable here.
    pub fn tag_type_id(&self) -> u8 {
        match self {
            NbtTag::Byte(_) => 1,
            NbtTag::Short(_) => 2,
            NbtTag::Int(_) => 3,
            NbtTag::Long(_) => 4,
            NbtTag::Float(_) => 5,
            NbtTag::Double(_) => 6,
            NbtTag::ByteArray(_) => 7,
            NbtTag::String(_) => 8,
            NbtTag::List(_) => 9,
            NbtTag::Compound(_) => 10,
            NbtTag::IntArray(_) => 11,
            NbtTag::LongArray(_) => 12,
        }
    }

    pub fn as_byte(&self) -> Option<i8> {
        match self {
            NbtTag::Byte(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_short(&self) -> Option<i16> {
        match self {
            NbtTag::Short(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i32> {
        match self {
            NbtTag::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_long(&self) -> Option<i64> {
        match self {
            NbtTag::Long(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f32> {
        match self {
            NbtTag::Float(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_double(&self) -> Option<f64> {
        match self {
            NbtTag::Double(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            NbtTag::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_compound(&self) -> Option<&NbtCompound> {
        match self {
            NbtTag::Compound(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[NbtTag]> {
        match self {
            NbtTag::List(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_byte_array(&self) -> Option<&[i8]> {
        match self {
            NbtTag::ByteArray(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_int_array(&self) -> Option<&[i32]> {
        match self {
            NbtTag::IntArray(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_long_array(&self) -> Option<&[i64]> {
        match self {
            NbtTag::LongArray(v) => Some(v),
            _ => None,
        }
    }
}

impl fmt::Display for NbtTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NbtTag::Byte(v) => write!(f, "{v}b"),
            NbtTag::Short(v) => write!(f, "{v}s"),
            NbtTag::Int(v) => write!(f, "{v}"),
            NbtTag::Long(v) => write!(f, "{v}L"),
            NbtTag::Float(v) => write!(f, "{v}f"),
            NbtTag::Double(v) => write!(f, "{v}d"),
            NbtTag::ByteArray(v) => write!(f, "[B; {} elements]", v.len()),
            NbtTag::String(v) => write!(f, "\"{v}\""),
            NbtTag::List(v) => write!(f, "[{} elements]", v.len()),
            NbtTag::Compound(v) => write!(f, "{{{} entries}}", v.len()),
            NbtTag::IntArray(v) => write!(f, "[I; {} elements]", v.len()),
            NbtTag::LongArray(v) => write!(f, "[L; {} elements]", v.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_type_ids() {
        assert_eq!(NbtTag::Byte(0).tag_type_id(), 1);
        assert_eq!(NbtTag::Short(0).tag_type_id(), 2);
        assert_eq!(NbtTag::Int(0).tag_type_id(), 3);
        assert_eq!(NbtTag::Long(0).tag_type_id(), 4);
        assert_eq!(NbtTag::Float(0.0).tag_type_id(), 5);
        assert_eq!(NbtTag::Double(0.0).tag_type_id(), 6);
        assert_eq!(NbtTag::ByteArray(vec![]).tag_type_id(), 7);
        assert_eq!(NbtTag::String(String::new()).tag_type_id(), 8);
        assert_eq!(NbtTag::List(vec![]).tag_type_id(), 9);
        assert_eq!(NbtTag::Compound(NbtCompound::new()).tag_type_id(), 10);
        assert_eq!(NbtTag::IntArray(vec![]).tag_type_id(), 11);
        assert_eq!(NbtTag::LongArray(vec![]).tag_type_id(), 12);
    }

    #[test]
    fn accessors() {
        assert_eq!(NbtTag::Byte(42).as_byte(), Some(42));
        assert_eq!(NbtTag::Int(42).as_byte(), None);
        assert_eq!(NbtTag::String("hello".into()).as_string(), Some("hello"));
        assert_eq!(NbtTag::Int(5).as_string(), None);
    }
}
