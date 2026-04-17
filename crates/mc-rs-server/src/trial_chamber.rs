//! Trial Chamber — 1.21 structure.

pub const MIN_DEPTH: i32 = -40;
pub const MAX_DEPTH: i32 = 30;

/// Trial chambers contain:
/// - Trial spawners (with mobs)
/// - Vaults (with loot)
/// - Ominous vaults (with ominous loot)
/// - Chiseled tuff
/// - Copper
/// - New wind charges

pub fn structure_name() -> &'static str {
    "trial_chambers"
}

/// Mobs spawned in trial spawners.
pub fn trial_mobs() -> &'static [&'static str] {
    &[
        "minecraft:husk",
        "minecraft:zombie",
        "minecraft:skeleton",
        "minecraft:spider",
        "minecraft:cave_spider",
        "minecraft:slime",
        "minecraft:stray",
        "minecraft:bogged",
        "minecraft:breeze",
    ]
}

/// Vault rewards.
pub fn vault_rewards() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:trial_key", 1, 1, 30),
        ("minecraft:arrow", 6, 17, 20),
        ("minecraft:snowball", 4, 12, 20),
        ("minecraft:iron_ingot", 1, 3, 20),
        ("minecraft:crossbow", 1, 1, 15),
        ("minecraft:shield", 1, 1, 15),
        ("minecraft:wind_charge", 2, 6, 15),
        ("minecraft:diamond", 1, 1, 5),
        ("minecraft:heavy_core", 1, 1, 1),
        ("minecraft:golden_apple", 1, 1, 10),
        ("minecraft:emerald", 1, 6, 10),
        ("minecraft:music_disc_creator", 1, 1, 2),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breeze_in_trial_mobs() {
        assert!(trial_mobs().contains(&"minecraft:breeze"));
    }
}
