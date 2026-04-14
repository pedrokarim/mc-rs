//! Banner patterns — 28 base patterns + 8 dye colors.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerPattern {
    BorderBase,
    BricksBase,
    CircleMiddle,
    Creeper,
    Cross,
    CurlyBorder,
    DiagonalLeft,
    DiagonalRight,
    DiagonalUpLeft,
    DiagonalUpRight,
    Flower,
    Gradient,
    GradientUp,
    HalfHorizontal,
    HalfHorizontalBottom,
    HalfVertical,
    HalfVerticalRight,
    Mojang,
    Rhombus,
    Skull,
    SmallStripes,
    SquareBottomLeft,
    SquareBottomRight,
    SquareTopLeft,
    SquareTopRight,
    StraightCross,
    StripeBottom,
    StripeCenter,
    StripeDownLeft,
    StripeDownRight,
    StripeLeft,
    StripeMiddle,
    StripeRight,
    StripeTop,
    TriangleBottom,
    TriangleTop,
    TrianglesBottom,
    TrianglesTop,
    Globe,
    Piglin,
    Flow,
    Guster,
}

impl BannerPattern {
    /// Require sherd/trim from Loom?
    pub fn requires_template(&self) -> bool {
        matches!(self,
            Self::Mojang | Self::Skull | Self::Flower | Self::Creeper |
            Self::Globe | Self::Piglin | Self::Flow | Self::Guster
        )
    }

    /// Template items.
    pub fn template_item(&self) -> Option<&'static str> {
        match self {
            Self::Mojang => Some("minecraft:enchanted_golden_apple"),
            Self::Skull => Some("minecraft:wither_skeleton_skull"),
            Self::Flower => Some("minecraft:oxeye_daisy"),
            Self::Creeper => Some("minecraft:creeper_head"),
            Self::Globe => Some("minecraft:globe_banner_pattern"),
            Self::Piglin => Some("minecraft:piglin_banner_pattern"),
            Self::Flow => Some("minecraft:flow_banner_pattern"),
            Self::Guster => Some("minecraft:guster_banner_pattern"),
            _ => None,
        }
    }
}

/// Max layered patterns (6 vanilla).
pub const MAX_LAYERS: usize = 6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skull_needs_template() {
        assert!(BannerPattern::Skull.requires_template());
    }

    #[test]
    fn cross_no_template() {
        assert!(!BannerPattern::Cross.requires_template());
    }
}
