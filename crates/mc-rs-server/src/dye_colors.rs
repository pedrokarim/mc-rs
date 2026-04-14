//! 16 dye colors + extensions.

use crate::dyes::DyeColor;

/// Convert DyeColor to RGB.
pub fn rgb(color: DyeColor) -> (u8, u8, u8) {
    match color {
        DyeColor::White => (240, 240, 240),
        DyeColor::Orange => (216, 127, 51),
        DyeColor::Magenta => (178, 76, 216),
        DyeColor::LightBlue => (102, 153, 216),
        DyeColor::Yellow => (229, 229, 51),
        DyeColor::Lime => (127, 204, 25),
        DyeColor::Pink => (242, 127, 165),
        DyeColor::Gray => (76, 76, 76),
        DyeColor::LightGray => (153, 153, 153),
        DyeColor::Cyan => (76, 127, 153),
        DyeColor::Purple => (127, 63, 178),
        DyeColor::Blue => (51, 76, 178),
        DyeColor::Brown => (102, 76, 51),
        DyeColor::Green => (102, 127, 51),
        DyeColor::Red => (153, 51, 51),
        DyeColor::Black => (25, 25, 25),
    }
}

/// Firework explosion colors.
pub fn firework_color(color: DyeColor) -> u32 {
    let (r, g, b) = rgb(color);
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_bright() {
        let (r, g, b) = rgb(DyeColor::White);
        assert!(r > 200 && g > 200 && b > 200);
    }

    #[test]
    fn black_dark() {
        let (r, g, b) = rgb(DyeColor::Black);
        assert!(r < 50 && g < 50 && b < 50);
    }
}
