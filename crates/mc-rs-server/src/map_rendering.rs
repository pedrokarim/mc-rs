//! Map rendering — chunk scan, color extraction, pixel update.

pub type MapColorId = u8;

/// Vanilla map color palette (partial — base colors).
pub fn color_palette() -> &'static [(&'static str, [u8; 3])] {
    &[
        ("grass", [127, 178, 56]),
        ("sand", [247, 233, 163]),
        ("wool", [199, 199, 199]),
        ("fire", [255, 0, 0]),
        ("ice", [160, 160, 255]),
        ("metal", [167, 167, 167]),
        ("plant", [0, 124, 0]),
        ("snow", [255, 255, 255]),
        ("clay", [164, 168, 184]),
        ("dirt", [151, 109, 77]),
        ("stone", [112, 112, 112]),
        ("water", [64, 64, 255]),
        ("wood", [143, 119, 72]),
        ("quartz", [255, 252, 245]),
        ("color_orange", [216, 127, 51]),
        ("color_magenta", [178, 76, 216]),
        ("color_light_blue", [102, 153, 216]),
        ("color_yellow", [229, 229, 51]),
        ("color_light_green", [127, 204, 25]),
        ("color_pink", [242, 127, 165]),
        ("color_gray", [76, 76, 76]),
        ("color_light_gray", [153, 153, 153]),
        ("color_cyan", [76, 127, 153]),
        ("color_purple", [127, 63, 178]),
        ("color_blue", [51, 76, 178]),
        ("color_brown", [102, 76, 51]),
        ("color_green", [102, 127, 51]),
        ("color_red", [153, 51, 51]),
        ("color_black", [25, 25, 25]),
        ("gold", [250, 238, 77]),
        ("diamond", [92, 219, 213]),
        ("lapis", [74, 128, 255]),
        ("emerald", [0, 217, 58]),
        ("podzol", [129, 86, 49]),
        ("nether", [112, 2, 0]),
    ]
}

/// Map resolution (128×128 pixels).
pub const MAP_SIZE: usize = 128;

/// Map scale levels (0=1:1, 1=1:2, ..., 4=1:16).
pub const MAX_SCALE: u8 = 4;

/// Scale to pixels-per-block.
pub fn scale_to_pixels_per_block(scale: u8) -> u32 {
    1 << scale.min(MAX_SCALE)
}

/// Scan a chunk, yield map pixels for each x,z at scale.
pub fn render_pixel_color(top_block_id: u16, above_water: bool) -> u8 {
    // Very simplified: just return some palette index.
    if above_water {
        match top_block_id {
            2 | 3 => 1,   // grass/dirt
            12 | 24 => 2, // sand/sandstone
            1 | 4 => 11,  // stone/cobble
            8 | 9 => 12,  // water
            _ => 3,
        }
    } else {
        12
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_not_empty() {
        assert!(!color_palette().is_empty());
    }

    #[test]
    fn scale_4_is_16() {
        assert_eq!(scale_to_pixels_per_block(4), 16);
    }
}
