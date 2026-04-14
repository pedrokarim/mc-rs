//! Banner patterns — port PMMP `src/block/Banner.php` + patterns.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerColor {
    White = 0,
    Orange = 1,
    Magenta = 2,
    LightBlue = 3,
    Yellow = 4,
    Lime = 5,
    Pink = 6,
    Gray = 7,
    LightGray = 8,
    Cyan = 9,
    Purple = 10,
    Blue = 11,
    Brown = 12,
    Green = 13,
    Red = 14,
    Black = 15,
}

/// PMMP `BannerPatternType` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerPatternType {
    BottomStripe,
    TopStripe,
    LeftStripe,
    RightStripe,
    CenterStripe,
    MiddleStripe,
    DownRightStripe,
    DownLeftStripe,
    SmallStripes,
    DiagonalCrosspatch,
    SquareCrosspatch,
    LeftOfDiagonal,
    RightOfUpsideDownDiagonal,
    LeftOfUpsideDownDiagonal,
    RightOfDiagonal,
    VerticalHalfLeft,
    VerticalHalfRight,
    HorizontalHalfTop,
    HorizontalHalfBottom,
    BottomLeftCorner,
    BottomRightCorner,
    TopLeftCorner,
    TopRightCorner,
    BottomTriangle,
    TopTriangle,
    BottomSmallTriangles,
    TopSmallTriangles,
    MiddleCircle,
    MiddleRhombus,
    Border,
    CurlyBorder,
    Brick,
    Gradient,
    GradientUpsideDown,
    Creeper,
    Skull,
    Flower,
    Mojang,
    Globe,
    Piglin,
}

impl BannerPatternType {
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::BottomStripe => "bs",
            Self::TopStripe => "ts",
            Self::LeftStripe => "ls",
            Self::RightStripe => "rs",
            Self::CenterStripe => "cs",
            Self::MiddleStripe => "ms",
            Self::DownRightStripe => "drs",
            Self::DownLeftStripe => "dls",
            Self::SmallStripes => "ss",
            Self::DiagonalCrosspatch => "cr",
            Self::SquareCrosspatch => "sc",
            Self::LeftOfDiagonal => "ld",
            Self::RightOfUpsideDownDiagonal => "rud",
            Self::LeftOfUpsideDownDiagonal => "lud",
            Self::RightOfDiagonal => "rd",
            Self::VerticalHalfLeft => "vh",
            Self::VerticalHalfRight => "vhr",
            Self::HorizontalHalfTop => "hh",
            Self::HorizontalHalfBottom => "hhb",
            Self::BottomLeftCorner => "bl",
            Self::BottomRightCorner => "br",
            Self::TopLeftCorner => "tl",
            Self::TopRightCorner => "tr",
            Self::BottomTriangle => "bt",
            Self::TopTriangle => "tt",
            Self::BottomSmallTriangles => "bts",
            Self::TopSmallTriangles => "tts",
            Self::MiddleCircle => "mc",
            Self::MiddleRhombus => "mr",
            Self::Border => "bo",
            Self::CurlyBorder => "cbo",
            Self::Brick => "bri",
            Self::Gradient => "gra",
            Self::GradientUpsideDown => "gru",
            Self::Creeper => "cre",
            Self::Skull => "sku",
            Self::Flower => "flo",
            Self::Mojang => "moj",
            Self::Globe => "glb",
            Self::Piglin => "pig",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BannerPattern {
    pub pattern_type: BannerPatternType,
    pub color: BannerColor,
}

#[derive(Debug, Clone)]
pub struct Banner {
    pub base_color: BannerColor,
    /// Liste ordonnée des patterns (appliqués de bas en haut).
    pub patterns: Vec<BannerPattern>,
}

impl Banner {
    pub fn new(base_color: BannerColor) -> Self {
        Self {
            base_color,
            patterns: Vec::new(),
        }
    }

    pub fn add_pattern(&mut self, p: BannerPattern) {
        self.patterns.push(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_add_patterns() {
        let mut b = Banner::new(BannerColor::White);
        b.add_pattern(BannerPattern {
            pattern_type: BannerPatternType::Creeper,
            color: BannerColor::Green,
        });
        assert_eq!(b.patterns.len(), 1);
    }

    #[test]
    fn identifier_creeper() {
        assert_eq!(BannerPatternType::Creeper.identifier(), "cre");
    }
}
