//! Minecraft text colors + formatting codes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextColor {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
    MinecoinGold,   // Bedrock only
    MaterialQuartz,
    MaterialIron,
    MaterialNetherite,
    MaterialRedstone,
    MaterialCopper,
    MaterialGold,
    MaterialEmerald,
    MaterialDiamond,
    MaterialLapis,
    MaterialAmethyst,
    MaterialResin,  // 1.21
    MaterialPale,
}

impl TextColor {
    pub fn code(&self) -> char {
        match self {
            Self::Black => '0',
            Self::DarkBlue => '1',
            Self::DarkGreen => '2',
            Self::DarkAqua => '3',
            Self::DarkRed => '4',
            Self::DarkPurple => '5',
            Self::Gold => '6',
            Self::Gray => '7',
            Self::DarkGray => '8',
            Self::Blue => '9',
            Self::Green => 'a',
            Self::Aqua => 'b',
            Self::Red => 'c',
            Self::LightPurple => 'd',
            Self::Yellow => 'e',
            Self::White => 'f',
            Self::MinecoinGold => 'g',
            Self::MaterialQuartz => 'h',
            Self::MaterialIron => 'i',
            Self::MaterialNetherite => 'j',
            Self::MaterialRedstone => 'm',
            Self::MaterialCopper => 'n',
            Self::MaterialGold => 'p',
            Self::MaterialEmerald => 'q',
            Self::MaterialDiamond => 's',
            Self::MaterialLapis => 't',
            Self::MaterialAmethyst => 'u',
            Self::MaterialResin => 'v',
            Self::MaterialPale => 'w',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFormat {
    Obfuscated,
    Bold,
    Strikethrough,
    Underline,
    Italic,
    Reset,
}

impl TextFormat {
    pub fn code(&self) -> char {
        match self {
            Self::Obfuscated => 'k',
            Self::Bold => 'l',
            Self::Strikethrough => 'm',
            Self::Underline => 'n',
            Self::Italic => 'o',
            Self::Reset => 'r',
        }
    }
}

/// Prefix for Minecraft format codes (§).
pub const FORMAT_PREFIX: char = '§';

pub fn colorize(text: &str, color: TextColor) -> String {
    format!("{}{}{}", FORMAT_PREFIX, color.code(), text)
}

pub fn strip_formatting(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == FORMAT_PREFIX {
            let _ = chars.next(); // skip format code
            continue;
        }
        result.push(c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gold_code_is_6() {
        assert_eq!(TextColor::Gold.code(), '6');
    }

    #[test]
    fn strip_removes_codes() {
        let s = "§6hello§r world";
        assert_eq!(strip_formatting(s), "hello world");
    }
}
