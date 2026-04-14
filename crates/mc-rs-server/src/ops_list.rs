//! Server operator list.

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct OpsList {
    /// Name → op level (1-4).
    pub ops: HashMap<String, u8>,
}

impl OpsList {
    pub fn op(&mut self, name: &str, level: u8) {
        self.ops.insert(name.to_lowercase(), level.clamp(1, 4));
    }

    pub fn deop(&mut self, name: &str) -> bool {
        self.ops.remove(&name.to_lowercase()).is_some()
    }

    pub fn level(&self, name: &str) -> u8 {
        *self.ops.get(&name.to_lowercase()).unwrap_or(&0)
    }

    pub fn is_op(&self, name: &str) -> bool {
        self.level(name) > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_sets_level() {
        let mut l = OpsList::default();
        l.op("Steve", 4);
        assert_eq!(l.level("steve"), 4);
    }
}
