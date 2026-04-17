//! Ocelot — Trust + fish lure.

#[derive(Debug, Clone)]
pub struct Ocelot {
    pub trusted: bool,
    pub trust_progress: u8,
    pub age: i32,
}

/// Trust chance per raw fish (1/3).
pub const TRUST_CHANCE: f32 = 1.0 / 3.0;
/// Fish qui gagne trust.
pub fn trust_items() -> &'static [&'static str] {
    &["minecraft:raw_cod", "minecraft:raw_salmon"]
}

impl Ocelot {
    pub fn new() -> Self {
        Self {
            trusted: false,
            trust_progress: 0,
            age: 0,
        }
    }

    pub fn feed_fish(&mut self) -> bool {
        if self.trusted {
            return true;
        }
        use rand::Rng;
        let mut rng = rand::thread_rng();
        if rng.gen::<f32>() < TRUST_CHANCE {
            self.trusted = true;
            return true;
        }
        false
    }
}

impl Default for Ocelot {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_once_already_trusted() {
        let mut o = Ocelot::new();
        o.trusted = true;
        assert!(o.feed_fish());
    }
}
