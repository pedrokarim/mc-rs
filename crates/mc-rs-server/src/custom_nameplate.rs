//! Custom nameplate / display name above entity.

#[derive(Debug, Clone)]
pub struct CustomNameplate {
    pub text: String,
    pub font: NameplateFont,
    pub color: u8,
    pub bold: bool,
    pub italic: bool,
    pub visible_through_walls: bool,
    pub offset_y: f32,
    pub scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameplateFont {
    Default,
    Alt,
    Illageralt,
    Rune,
}

impl CustomNameplate {
    pub fn new(text: String) -> Self {
        Self {
            text,
            font: NameplateFont::Default,
            color: 15, // white
            bold: false,
            italic: false,
            visible_through_walls: false,
            offset_y: 0.3,
            scale: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_white() {
        assert_eq!(CustomNameplate::new("test".into()).color, 15);
    }
}
