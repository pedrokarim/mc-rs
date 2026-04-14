//! Dyes — port PMMP `src/block/utils/DyeColor.php`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DyeColor {
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

impl DyeColor {
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::White => "white",
            Self::Orange => "orange",
            Self::Magenta => "magenta",
            Self::LightBlue => "light_blue",
            Self::Yellow => "yellow",
            Self::Lime => "lime",
            Self::Pink => "pink",
            Self::Gray => "gray",
            Self::LightGray => "light_gray",
            Self::Cyan => "cyan",
            Self::Purple => "purple",
            Self::Blue => "blue",
            Self::Brown => "brown",
            Self::Green => "green",
            Self::Red => "red",
            Self::Black => "black",
        }
    }

    /// RGB couleur pour le rendu / signs / banners.
    pub fn rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::White => (255, 255, 255),
            Self::Orange => (216, 127, 51),
            Self::Magenta => (178, 76, 216),
            Self::LightBlue => (102, 153, 216),
            Self::Yellow => (229, 229, 51),
            Self::Lime => (127, 204, 25),
            Self::Pink => (242, 127, 165),
            Self::Gray => (76, 76, 76),
            Self::LightGray => (153, 153, 153),
            Self::Cyan => (76, 127, 153),
            Self::Purple => (127, 63, 178),
            Self::Blue => (51, 76, 178),
            Self::Brown => (102, 76, 51),
            Self::Green => (102, 127, 51),
            Self::Red => (153, 51, 51),
            Self::Black => (25, 25, 25),
        }
    }

    pub fn all() -> [Self; 16] {
        [
            Self::White, Self::Orange, Self::Magenta, Self::LightBlue,
            Self::Yellow, Self::Lime, Self::Pink, Self::Gray,
            Self::LightGray, Self::Cyan, Self::Purple, Self::Blue,
            Self::Brown, Self::Green, Self::Red, Self::Black,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_16_dyes() {
        assert_eq!(DyeColor::all().len(), 16);
    }

    #[test]
    fn red_rgb() {
        assert_eq!(DyeColor::Red.rgb(), (153, 51, 51));
    }
}
