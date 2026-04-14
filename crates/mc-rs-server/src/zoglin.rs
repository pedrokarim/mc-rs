//! Zoglin — hoglin zombifié.

#[derive(Debug, Clone)]
pub struct Zoglin {
    pub age: i32,
    pub attack_cooldown: u32,
    pub target_entity: Option<u64>,
}

/// Hostile envers tout sauf zombies (creeper, hoglin etc).
pub fn target_filter(kind: &str) -> bool {
    !matches!(kind,
        "zombie" | "zombie_villager" | "husk" | "drowned" |
        "zoglin" | "piglin_brute" | "piglin" | "zombified_piglin"
    )
}

impl Zoglin {
    pub fn new() -> Self {
        Self { age: 0, attack_cooldown: 0, target_entity: None }
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }
    }

    /// Knockback chance on attack (50%).
    pub fn knockback_chance() -> f32 { 0.5 }
}

impl Default for Zoglin {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attacks_villager() {
        assert!(target_filter("villager"));
    }

    #[test]
    fn doesnt_attack_zombie() {
        assert!(!target_filter("zombie"));
    }
}
