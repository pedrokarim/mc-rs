//! Cow — milk with bucket.

#[derive(Debug, Clone)]
pub struct Cow {
    pub age: i32,
}

pub const BREEDING_ITEM: &str = "minecraft:wheat";

impl Cow {
    pub fn new_adult() -> Self {
        Self { age: 0 }
    }

    pub fn new_baby() -> Self {
        Self { age: -24000 }
    }

    pub fn is_baby(&self) -> bool { self.age < 0 }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
    }

    /// Milk only obtainable from adult cow.
    pub fn can_be_milked(&self) -> bool {
        !self.is_baby()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baby_cant_be_milked() {
        let c = Cow::new_baby();
        assert!(!c.can_be_milked());
    }
}
