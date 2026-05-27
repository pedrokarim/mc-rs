//! Sign editing — port PMMP `src/block/tile/Sign.php` + `SignText.php`.

const MAX_LINE_LENGTH: usize = 50; // Bedrock allows longer lines than Java.
const MAX_LINES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignFace {
    Front,
    Back, // 1.20+ hanging signs can have 2 sides
}

#[derive(Debug, Clone, Default)]
pub struct SignText {
    pub lines: [String; 4],
    pub glow: bool,
    pub color: u32, // 0xRRGGBB
}

impl SignText {
    pub fn set_line(&mut self, index: usize, text: impl Into<String>) -> bool {
        if index >= MAX_LINES {
            return false;
        }
        let mut t = text.into();
        if t.len() > MAX_LINE_LENGTH {
            t.truncate(MAX_LINE_LENGTH);
        }
        self.lines[index] = t;
        true
    }

    pub fn line(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(|s| s.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|l| l.is_empty())
    }
}

#[derive(Debug, Clone)]
pub struct Sign {
    pub position: [i32; 3],
    pub front: SignText,
    pub back: SignText,
    pub is_waxed: bool, // honeycomb → no edit
}

impl Sign {
    pub fn new(position: [i32; 3]) -> Self {
        Self {
            position,
            front: SignText::default(),
            back: SignText::default(),
            is_waxed: false,
        }
    }

    pub fn edit(&mut self, face: SignFace, text: SignText) -> bool {
        if self.is_waxed {
            return false;
        }
        match face {
            SignFace::Front => self.front = text,
            SignFace::Back => self.back = text,
        }
        true
    }

    pub fn wax(&mut self) -> bool {
        if self.is_waxed {
            false
        } else {
            self.is_waxed = true;
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_line_truncates_long() {
        let mut t = SignText::default();
        let long = "a".repeat(100);
        t.set_line(0, long);
        assert_eq!(t.lines[0].len(), MAX_LINE_LENGTH);
    }

    #[test]
    fn waxed_cannot_edit() {
        let mut s = Sign::new([0, 64, 0]);
        s.wax();
        assert!(!s.edit(SignFace::Front, SignText::default()));
    }

    #[test]
    fn wax_once() {
        let mut s = Sign::new([0, 64, 0]);
        assert!(s.wax());
        assert!(!s.wax());
    }
}
