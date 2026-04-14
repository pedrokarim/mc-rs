//! Command blocks — repeating, chain, impulse.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBlockKind {
    Impulse,   // Fires once on pulse
    Repeating, // Fires every tick when powered
    Chain,     // Triggered by previous command block
}

#[derive(Debug, Clone)]
pub struct CommandBlock {
    pub kind: CommandBlockKind,
    pub command: String,
    pub tracking_output: bool,
    pub conditional: bool, // Only fire if previous succeeded
    pub last_output: Option<String>,
    pub success_count: u32,
    pub auto: bool, // needs redstone
    pub facing: u8,
}

impl CommandBlock {
    pub fn new(kind: CommandBlockKind, command: impl Into<String>) -> Self {
        Self {
            kind,
            command: command.into(),
            tracking_output: true,
            conditional: false,
            last_output: None,
            success_count: 0,
            auto: false,
            facing: 0,
        }
    }

    pub fn needs_redstone(&self) -> bool {
        !self.auto
    }

    pub fn run(&mut self, output: Option<String>) {
        if self.tracking_output {
            self.last_output = output;
        }
        self.success_count = self.success_count.saturating_add(1);
    }
}

/// Command block placement requires /op or creative.
pub const NEEDS_CREATIVE_TO_EDIT: bool = true;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_mode_no_redstone() {
        let mut c = CommandBlock::new(CommandBlockKind::Repeating, "say hi");
        c.auto = true;
        assert!(!c.needs_redstone());
    }
}
