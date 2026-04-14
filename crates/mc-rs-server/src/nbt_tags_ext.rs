//! NBT tag helpers — read / write convenient wrappers.

use mc_rs_nbt::NbtTag;

/// Extract string or default.
pub fn get_string(tag: &NbtTag, key: &str) -> Option<String> {
    if let NbtTag::Compound(map) = tag {
        if let Some(NbtTag::String(v)) = map.get(key) {
            return Some(v.clone());
        }
    }
    None
}

/// Extract int or default.
pub fn get_int(tag: &NbtTag, key: &str) -> Option<i32> {
    if let NbtTag::Compound(map) = tag {
        match map.get(key) {
            Some(NbtTag::Int(v)) => Some(*v),
            Some(NbtTag::Short(v)) => Some(*v as i32),
            Some(NbtTag::Byte(v)) => Some(*v as i32),
            _ => None,
        }
    } else {
        None
    }
}

/// Check if compound contains key.
pub fn has_key(tag: &NbtTag, key: &str) -> bool {
    if let NbtTag::Compound(map) = tag {
        map.contains_key(key)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn read_string() {
        let mut m = HashMap::new();
        m.insert("name".to_string(), NbtTag::String("steve".into()));
        let tag = NbtTag::Compound(m);
        assert_eq!(get_string(&tag, "name"), Some("steve".into()));
    }
}
