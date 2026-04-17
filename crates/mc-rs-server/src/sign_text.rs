//! Sign — 4 lines text, glow dye, wax.

#[derive(Debug, Clone)]
pub struct SignData {
    pub front_lines: [String; 4],
    pub back_lines: [String; 4],
    pub front_color: u8,
    pub back_color: u8,
    pub front_glowing: bool,
    pub back_glowing: bool,
    pub waxed: bool,
    pub editor: Option<u64>, // Player currently editing
}

/// Max characters per line (PMMP/vanilla 16).
pub const MAX_LINE_LENGTH: usize = 16;

impl SignData {
    pub fn new() -> Self {
        Self {
            front_lines: Default::default(),
            back_lines: Default::default(),
            front_color: 0,
            back_color: 0,
            front_glowing: false,
            back_glowing: false,
            waxed: false,
            editor: None,
        }
    }

    pub fn can_edit(&self, player: u64) -> bool {
        !self.waxed && (self.editor.is_none() || self.editor == Some(player))
    }

    pub fn wax(&mut self) {
        self.waxed = true;
    }

    pub fn set_front_line(&mut self, idx: usize, line: String) -> bool {
        if idx >= 4 || self.waxed {
            return false;
        }
        self.front_lines[idx] = line.chars().take(MAX_LINE_LENGTH).collect();
        true
    }
}

impl Default for SignData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wax_blocks_edit() {
        let mut s = SignData::new();
        s.wax();
        assert!(!s.can_edit(1));
    }

    #[test]
    fn line_truncates() {
        let mut s = SignData::new();
        s.set_front_line(0, "a".repeat(100));
        assert_eq!(s.front_lines[0].len(), MAX_LINE_LENGTH);
    }
}
