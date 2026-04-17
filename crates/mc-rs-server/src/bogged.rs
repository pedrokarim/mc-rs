//! Bogged — 1.21 skeleton qui shoot arrows avec poison.

#[derive(Debug, Clone)]
pub struct Bogged {
    pub attack_cooldown: u32,
    pub target_entity: Option<u64>,
    pub missing_mushroom: bool, // when sheared, loses mushroom
}

/// Poison duration per arrow hit (3s easy, 4s normal, 5s hard).
pub fn poison_duration(difficulty: u8) -> u32 {
    match difficulty {
        0 | 1 => 3 * 20,
        2 => 4 * 20,
        _ => 5 * 20,
    }
}

/// Drop mushroom when sheared.
pub fn shear_drop() -> &'static str {
    "minecraft:red_mushroom"
}

impl Bogged {
    pub fn new() -> Self {
        Self {
            attack_cooldown: 0,
            target_entity: None,
            missing_mushroom: false,
        }
    }

    pub fn shear(&mut self) -> Option<&'static str> {
        if self.missing_mushroom {
            return None;
        }
        self.missing_mushroom = true;
        Some(shear_drop())
    }
}

impl Default for Bogged {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shears_once() {
        let mut b = Bogged::new();
        assert!(b.shear().is_some());
        assert!(b.shear().is_none());
    }
}
