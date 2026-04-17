//! Polar Bear — mob passif qui protège ses petits.

#[derive(Debug, Clone)]
pub struct PolarBear {
    pub age: i32,
    pub parent_of: Vec<u64>, // Baby entity IDs
    pub provoked_ticks: u32,
}

/// Parent protection: any attack on baby angers mother.
pub const PROTECT_DURATION: u32 = 600;
/// Attack damage.
pub const ATTACK_DAMAGE: f32 = 6.0;
/// Small baby cub has 10 HP.
pub const BABY_HP: f32 = 10.0;
pub const ADULT_HP: f32 = 30.0;

impl PolarBear {
    pub fn new_adult() -> Self {
        Self {
            age: 0,
            parent_of: Vec::new(),
            provoked_ticks: 0,
        }
    }

    pub fn new_baby() -> Self {
        Self {
            age: -24000,
            parent_of: Vec::new(),
            provoked_ticks: 0,
        }
    }

    pub fn is_baby(&self) -> bool {
        self.age < 0
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.provoked_ticks > 0 {
            self.provoked_ticks -= 1;
        }
    }

    pub fn hp(&self) -> f32 {
        if self.is_baby() {
            BABY_HP
        } else {
            ADULT_HP
        }
    }

    pub fn provoke(&mut self) {
        self.provoked_ticks = PROTECT_DURATION;
    }

    pub fn is_hostile(&self) -> bool {
        !self.parent_of.is_empty() || self.provoked_ticks > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mother_protects() {
        let mut p = PolarBear::new_adult();
        p.parent_of.push(1);
        assert!(p.is_hostile());
    }
}
