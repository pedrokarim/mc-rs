//! Leads — laisse pour attacher mobs à fence/player.

#[derive(Debug, Clone)]
pub struct Lead {
    pub holder: LeadHolder,
    pub leashed_entity: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeadHolder {
    Player(u64),
    Fence(i32, i32, i32),
}

/// Max lead range (10 blocs = breakaway).
pub const MAX_LEAD_DISTANCE: f64 = 10.0;
/// Lead breaks at this distance.
pub const BREAK_DISTANCE: f64 = 10.0;

/// Mobs that can be leashed (vanilla + PMMP).
pub fn leashable_mobs() -> &'static [&'static str] {
    &[
        "minecraft:cow",
        "minecraft:pig",
        "minecraft:sheep",
        "minecraft:horse",
        "minecraft:mule",
        "minecraft:donkey",
        "minecraft:llama",
        "minecraft:trader_llama",
        "minecraft:chicken",
        "minecraft:dolphin",
        "minecraft:wolf",
        "minecraft:cat",
        "minecraft:ocelot",
        "minecraft:skeleton_horse",
        "minecraft:zombie_horse",
        "minecraft:parrot",
        "minecraft:rabbit",
        "minecraft:squid",
        "minecraft:fox",
        "minecraft:panda",
        "minecraft:strider",
        "minecraft:goat",
        "minecraft:frog",
        "minecraft:sniffer",
        "minecraft:camel",
        "minecraft:happy_ghast",
    ]
}

impl Lead {
    pub fn new(holder: LeadHolder, entity: u64) -> Self {
        Self { holder, leashed_entity: entity }
    }

    pub fn can_leash(mob: &str) -> bool {
        leashable_mobs().contains(&mob)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cow_is_leashable() {
        assert!(Lead::can_leash("minecraft:cow"));
    }

    #[test]
    fn zombie_not_leashable() {
        assert!(!Lead::can_leash("minecraft:zombie"));
    }
}
