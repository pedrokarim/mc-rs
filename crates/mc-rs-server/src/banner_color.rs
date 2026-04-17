//! Banner colors (DyeColor mapping).

use crate::dyes::DyeColor;

pub fn banner_base_colors() -> Vec<DyeColor> {
    vec![
        DyeColor::White,
        DyeColor::Orange,
        DyeColor::Magenta,
        DyeColor::LightBlue,
        DyeColor::Yellow,
        DyeColor::Lime,
        DyeColor::Pink,
        DyeColor::Gray,
        DyeColor::LightGray,
        DyeColor::Cyan,
        DyeColor::Purple,
        DyeColor::Blue,
        DyeColor::Brown,
        DyeColor::Green,
        DyeColor::Red,
        DyeColor::Black,
    ]
}

/// Block ID range for banners.
pub const BANNER_BLOCK_START: u16 = 176;
pub const BANNER_BLOCK_END: u16 = 191;

pub fn is_banner_block(block_id: u16) -> bool {
    (BANNER_BLOCK_START..=BANNER_BLOCK_END).contains(&block_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixteen_colors() {
        assert_eq!(banner_base_colors().len(), 16);
    }
}
