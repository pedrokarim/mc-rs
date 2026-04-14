//! Comportements mob spécifiques — port PMMP + vanilla.
//! Regroupe : taming (wolf/cat), breeding (cow/sheep/pig/horse), villager
//! reputation, hoglin/piglin trade, bee hive state, warden anger.

use std::collections::HashMap;
use std::net::SocketAddr;

// ── Taming ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TamingItem {
    Bone,        // wolf
    Fish,        // cat
    GoldenApple, // horse
    Sugar,       // horse
    Apple,       // horse
    Wheat,       // horse/cow/sheep
    HayBale,     // horse
}

/// Chance de tamer en 1 attempt (0-1).
pub fn taming_chance(item: TamingItem) -> f32 {
    match item {
        TamingItem::Bone => 0.33,
        TamingItem::Fish => 0.33,
        TamingItem::GoldenApple => 0.75,
        TamingItem::Sugar | TamingItem::Apple | TamingItem::Wheat => 0.2,
        TamingItem::HayBale => 0.5,
    }
}

#[derive(Debug, Clone)]
pub struct TameableState {
    pub tamed: bool,
    pub owner: Option<uuid::Uuid>,
    pub sitting: bool,
}

impl TameableState {
    pub fn new() -> Self {
        Self {
            tamed: false,
            owner: None,
            sitting: false,
        }
    }

    pub fn try_tame(&mut self, owner: uuid::Uuid, item: TamingItem) -> bool {
        use rand::Rng;
        if self.tamed {
            return false;
        }
        if rand::thread_rng().gen::<f32>() < taming_chance(item) {
            self.tamed = true;
            self.owner = Some(owner);
            true
        } else {
            false
        }
    }
}

impl Default for TameableState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Breeding ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreedingFood {
    Wheat,
    Carrot,
    GoldenCarrot,
    Seeds,
    Salmon,
    Cod,
    Bamboo,
    Apple,
    SweetBerries,
    GlowBerries,
}

#[derive(Debug, Clone)]
pub struct BreedingState {
    pub love_ticks: u32,
    pub last_bred_cooldown: u32,
}

impl BreedingState {
    pub fn new() -> Self {
        Self {
            love_ticks: 0,
            last_bred_cooldown: 0,
        }
    }

    pub fn enter_love_mode(&mut self) {
        self.love_ticks = 600; // 30s
    }

    pub fn is_in_love(&self) -> bool {
        self.love_ticks > 0 && self.last_bred_cooldown == 0
    }

    pub fn tick(&mut self) {
        if self.love_ticks > 0 {
            self.love_ticks -= 1;
        }
        if self.last_bred_cooldown > 0 {
            self.last_bred_cooldown -= 1;
        }
    }

    pub fn breed_with(&mut self) {
        self.love_ticks = 0;
        self.last_bred_cooldown = 6000; // 5 min
    }
}

impl Default for BreedingState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Villager reputation (per-player) ────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct VillagerReputation {
    pub per_player: HashMap<SocketAddr, i32>,
    /// Global repu from raid victories / losses (village-wide).
    pub global_bonus: i32,
}

impl VillagerReputation {
    pub fn modify(&mut self, addr: SocketAddr, delta: i32) {
        let entry = self.per_player.entry(addr).or_insert(0);
        *entry = (*entry + delta).clamp(-100, 100);
    }

    pub fn get(&self, addr: &SocketAddr) -> i32 {
        self.per_player.get(addr).copied().unwrap_or(0) + self.global_bonus
    }
}

// ── Piglin bartering ────────────────────────────────────────────────────────

/// PMMP N/A — Bedrock feature. Items donnés contre des gold ingots.
pub fn piglin_barter_outputs() -> Vec<(&'static str, u32)> {
    // (item_name, weight)
    vec![
        ("minecraft:crying_obsidian", 20),
        ("minecraft:ender_pearl", 20),
        ("minecraft:splash_water_bottle", 10),
        ("minecraft:iron_boots", 8),
        ("minecraft:fire_charge", 40),
        ("minecraft:leather", 40),
        ("minecraft:nether_brick", 40),
        ("minecraft:obsidian", 40),
        ("minecraft:soul_sand", 40),
        ("minecraft:potion_fire_resistance", 10),
        ("minecraft:gravel", 40),
        ("minecraft:spectral_arrow", 10),
        ("minecraft:blackstone", 40),
        ("minecraft:iron_nugget", 10),
        ("minecraft:string", 20),
        ("minecraft:magma_cream", 20),
    ]
}

// ── Bee hive state ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BeehiveState {
    pub bees_inside: u32, // max 3
    pub honey_level: u8,  // 0-5
}

impl BeehiveState {
    pub const MAX_BEES: u32 = 3;
    pub const MAX_HONEY: u8 = 5;

    pub fn new() -> Self {
        Self {
            bees_inside: 0,
            honey_level: 0,
        }
    }

    pub fn is_full(&self) -> bool {
        self.bees_inside >= Self::MAX_BEES
    }

    pub fn add_bee(&mut self) -> bool {
        if self.is_full() {
            return false;
        }
        self.bees_inside += 1;
        true
    }

    pub fn increment_honey(&mut self) -> bool {
        if self.honey_level < Self::MAX_HONEY {
            self.honey_level += 1;
            true
        } else {
            false
        }
    }

    pub fn harvest_honey(&mut self) -> bool {
        if self.honey_level >= Self::MAX_HONEY {
            self.honey_level = 0;
            true
        } else {
            false
        }
    }
}

impl Default for BeehiveState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Warden anger ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct WardenAnger {
    pub anger_level: u32, // 0-150
    pub target_runtime_id: Option<u64>,
}

impl WardenAnger {
    pub fn anger_at(&mut self, target: u64, amount: u32) {
        self.anger_level = (self.anger_level + amount).min(150);
        self.target_runtime_id = Some(target);
    }

    pub fn tick(&mut self) {
        // Anger decays 1 per tick.
        if self.anger_level > 0 {
            self.anger_level -= 1;
        }
        if self.anger_level == 0 {
            self.target_runtime_id = None;
        }
    }

    pub fn is_angry(&self) -> bool {
        self.anger_level >= 80
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beehive_fills_to_3() {
        let mut h = BeehiveState::new();
        assert!(h.add_bee());
        assert!(h.add_bee());
        assert!(h.add_bee());
        assert!(!h.add_bee());
    }

    #[test]
    fn warden_anger_decay() {
        let mut w = WardenAnger::default();
        w.anger_at(1, 100);
        assert!(w.is_angry());
        for _ in 0..100 {
            w.tick();
        }
        assert!(!w.is_angry());
    }

    #[test]
    fn love_mode_30_seconds() {
        let mut b = BreedingState::new();
        b.enter_love_mode();
        assert!(b.is_in_love());
    }

    #[test]
    fn villager_repu_clamped() {
        use std::net::SocketAddr;
        use std::str::FromStr;
        let mut r = VillagerReputation::default();
        let a = SocketAddr::from_str("127.0.0.1:1001").unwrap();
        r.modify(a, 200);
        assert_eq!(r.get(&a), 100);
        r.modify(a, -300);
        assert_eq!(r.get(&a), -100);
    }
}
