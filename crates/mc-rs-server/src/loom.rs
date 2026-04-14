//! Loom — banner pattern applicator.

use crate::banner_pattern::BannerPattern;

#[derive(Debug, Clone)]
pub struct LoomOperation {
    pub banner_input: u16,      // Banner item
    pub banner_color: u8,
    pub dye_input: u8,          // Dye applied
    pub pattern: BannerPattern,
    pub existing_layers: u8,
}

impl LoomOperation {
    /// Check if this operation is valid.
    pub fn can_apply(&self) -> bool {
        self.existing_layers < crate::banner_pattern::MAX_LAYERS as u8
    }

    /// Output damage = input damage + 1 layer.
    pub fn output_layer_count(&self) -> u8 {
        self.existing_layers + 1
    }
}

/// Banner patterns available without template.
pub fn free_patterns() -> &'static [BannerPattern] {
    &[
        BannerPattern::BorderBase,
        BannerPattern::BricksBase,
        BannerPattern::CircleMiddle,
        BannerPattern::Cross,
        BannerPattern::CurlyBorder,
        BannerPattern::DiagonalLeft,
        BannerPattern::DiagonalRight,
        BannerPattern::DiagonalUpLeft,
        BannerPattern::DiagonalUpRight,
        BannerPattern::Gradient,
        BannerPattern::GradientUp,
        BannerPattern::HalfHorizontal,
        BannerPattern::HalfHorizontalBottom,
        BannerPattern::HalfVertical,
        BannerPattern::HalfVerticalRight,
        BannerPattern::Rhombus,
        BannerPattern::SmallStripes,
        BannerPattern::SquareBottomLeft,
        BannerPattern::SquareBottomRight,
        BannerPattern::SquareTopLeft,
        BannerPattern::SquareTopRight,
        BannerPattern::StraightCross,
        BannerPattern::StripeBottom,
        BannerPattern::StripeCenter,
        BannerPattern::StripeDownLeft,
        BannerPattern::StripeDownRight,
        BannerPattern::StripeLeft,
        BannerPattern::StripeMiddle,
        BannerPattern::StripeRight,
        BannerPattern::StripeTop,
        BannerPattern::TriangleBottom,
        BannerPattern::TriangleTop,
        BannerPattern::TrianglesBottom,
        BannerPattern::TrianglesTop,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_count_increments() {
        let op = LoomOperation {
            banner_input: 0,
            banner_color: 0,
            dye_input: 0,
            pattern: BannerPattern::Cross,
            existing_layers: 2,
        };
        assert_eq!(op.output_layer_count(), 3);
    }
}
