//! Loom logic — banner pattern application.

use mc_rs_nbt::tag::{NbtCompound, NbtRoot, NbtTag};

/// All known banner pattern IDs and their display names.
pub const LOOM_PATTERNS: &[(&str, &str)] = &[
    ("bo", "Border"),
    ("bri", "Bricks"),
    ("cr", "Cross"),
    ("cbo", "Curly Border"),
    ("dls", "Left Diagonal"),
    ("drs", "Right Diagonal"),
    ("gra", "Gradient"),
    ("gru", "Gradient Upside-Down"),
    ("hh", "Top Half Horizontal"),
    ("hhb", "Bottom Half Horizontal"),
    ("mr", "Rhombus"),
    ("ms", "Middle Stripe"),
    ("ss", "Small Stripes"),
    ("tl", "Top Left Square"),
    ("tr", "Top Right Square"),
    ("bl", "Bottom Left Square"),
    ("br", "Bottom Right Square"),
    ("ts", "Top Stripe"),
    ("bs", "Bottom Stripe"),
    ("ls", "Left Stripe"),
    ("rs", "Right Stripe"),
    ("tt", "Top Triangle"),
    ("bt", "Bottom Triangle"),
    ("tts", "Top Triangle Sawtooth"),
    ("bts", "Bottom Triangle Sawtooth"),
    ("vh", "Left Vertical Half"),
    ("vhr", "Right Vertical Half"),
    // Patterns requiring special banner pattern items:
    ("cre", "Creeper"),
    ("flo", "Flower"),
    ("mc", "Mojang"),
    ("sc", "Skull"),
    ("glb", "Globe"),
    ("pig", "Piglin"),
];

/// Maximum banner pattern layers (Bedrock limit).
pub const MAX_LAYERS: usize = 6;

/// Patterns that require a special banner pattern item in the pattern slot.
const SPECIAL_PATTERNS: &[&str] = &["cre", "flo", "mc", "sc", "glb", "pig"];

/// Check if a pattern ID is valid.
pub fn is_valid_pattern(pattern_id: &str) -> bool {
    LOOM_PATTERNS.iter().any(|(id, _)| *id == pattern_id)
}

/// Check if a pattern requires a special banner pattern item.
pub fn pattern_needs_item(pattern_id: &str) -> bool {
    SPECIAL_PATTERNS.contains(&pattern_id)
}

/// Map a dye item name to the Bedrock banner dye color index (0-15).
pub fn dye_color_from_item(item_name: &str) -> Option<i32> {
    match item_name {
        "minecraft:white_dye" | "minecraft:bone_meal" => Some(0),
        "minecraft:orange_dye" => Some(1),
        "minecraft:magenta_dye" => Some(2),
        "minecraft:light_blue_dye" => Some(3),
        "minecraft:yellow_dye" => Some(4),
        "minecraft:lime_dye" => Some(5),
        "minecraft:pink_dye" => Some(6),
        "minecraft:gray_dye" => Some(7),
        "minecraft:light_gray_dye" => Some(8),
        "minecraft:cyan_dye" => Some(9),
        "minecraft:purple_dye" => Some(10),
        "minecraft:blue_dye" | "minecraft:lapis_lazuli" => Some(11),
        "minecraft:brown_dye" | "minecraft:cocoa_beans" => Some(12),
        "minecraft:green_dye" => Some(13),
        "minecraft:red_dye" => Some(14),
        "minecraft:black_dye" | "minecraft:ink_sac" => Some(15),
        _ => None,
    }
}

/// Apply a banner pattern to existing banner NBT data.
///
/// If `banner_nbt` is empty, starts a fresh pattern list.
/// Returns the new NBT data with the pattern appended, or `None` if
/// the banner already has the maximum number of layers.
pub fn apply_pattern(banner_nbt: &[u8], pattern_id: &str, dye_color: i32) -> Option<Vec<u8>> {
    let mut patterns: Vec<NbtTag> = Vec::new();

    // Parse existing patterns if present
    if !banner_nbt.is_empty() {
        if let Ok(root) = mc_rs_nbt::read_nbt_le(&mut &banner_nbt[..]) {
            if let Some(NbtTag::List(list)) = root.compound.get("Patterns") {
                if list.len() >= MAX_LAYERS {
                    return None; // Already at max
                }
                patterns = list.clone();
            }
        }
    }

    if patterns.len() >= MAX_LAYERS {
        return None;
    }

    // Add new pattern
    let mut entry = NbtCompound::new();
    entry.insert("Color".to_string(), NbtTag::Int(dye_color));
    entry.insert(
        "Pattern".to_string(),
        NbtTag::String(pattern_id.to_string()),
    );
    patterns.push(NbtTag::Compound(entry));

    // Build output NBT
    let mut root_c = NbtCompound::new();
    root_c.insert("Patterns".to_string(), NbtTag::List(patterns));
    let root = NbtRoot::new("", root_c);
    let mut buf = Vec::new();
    mc_rs_nbt::write_nbt_le(&mut buf, &root);
    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_pattern_ids() {
        assert!(is_valid_pattern("bo"));
        assert!(is_valid_pattern("cre"));
        assert!(!is_valid_pattern("invalid"));
    }

    #[test]
    fn special_patterns_need_item() {
        assert!(pattern_needs_item("cre"));
        assert!(pattern_needs_item("sc"));
        assert!(!pattern_needs_item("bo"));
        assert!(!pattern_needs_item("bri"));
    }

    #[test]
    fn dye_color_mapping() {
        assert_eq!(dye_color_from_item("minecraft:white_dye"), Some(0));
        assert_eq!(dye_color_from_item("minecraft:red_dye"), Some(14));
        assert_eq!(dye_color_from_item("minecraft:black_dye"), Some(15));
        assert_eq!(dye_color_from_item("minecraft:ink_sac"), Some(15));
        assert_eq!(dye_color_from_item("minecraft:stone"), None);
    }

    #[test]
    fn apply_pattern_to_empty_banner() {
        let result = apply_pattern(&[], "bo", 14).unwrap();
        assert!(!result.is_empty());

        // Parse the result and verify
        let root = mc_rs_nbt::read_nbt_le(&mut &result[..]).unwrap();
        let patterns = root.compound.get("Patterns").unwrap().as_list().unwrap();
        assert_eq!(patterns.len(), 1);
    }

    #[test]
    fn apply_pattern_stacks() {
        let nbt1 = apply_pattern(&[], "bo", 0).unwrap();
        let nbt2 = apply_pattern(&nbt1, "cr", 14).unwrap();

        let root = mc_rs_nbt::read_nbt_le(&mut &nbt2[..]).unwrap();
        let patterns = root.compound.get("Patterns").unwrap().as_list().unwrap();
        assert_eq!(patterns.len(), 2);
    }

    #[test]
    fn apply_pattern_max_layers() {
        let mut nbt = Vec::new();
        for i in 0..MAX_LAYERS {
            nbt = apply_pattern(&nbt, "bo", i as i32).unwrap();
        }
        // 7th layer should fail
        assert!(apply_pattern(&nbt, "bo", 0).is_none());
    }
}
