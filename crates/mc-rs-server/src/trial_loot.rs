//! Trial chamber rewards (vault loot).

pub fn vault_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:trial_key", 1, 1, 100),
        ("minecraft:wind_charge", 2, 6, 30),
        ("minecraft:diamond", 1, 3, 5),
        ("minecraft:iron_ingot", 1, 5, 20),
        ("minecraft:emerald", 1, 2, 15),
        ("minecraft:gold_ingot", 1, 3, 10),
        ("minecraft:enchanted_book", 1, 1, 10),
        ("minecraft:experience_bottle", 1, 1, 5),
        ("minecraft:music_disc_creator", 1, 1, 1),
        ("minecraft:flow_armor_trim_smithing_template", 1, 1, 5),
    ]
}

pub fn ominous_vault_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:ominous_trial_key", 1, 1, 50),
        ("minecraft:wind_charge", 4, 12, 30),
        ("minecraft:diamond", 2, 6, 10),
        ("minecraft:enchanted_book", 1, 1, 30),
        ("minecraft:experience_bottle", 2, 5, 15),
        ("minecraft:heavy_core", 1, 1, 5),
        ("minecraft:bolt_armor_trim_smithing_template", 1, 1, 10),
        ("minecraft:music_disc_precipice", 1, 1, 5),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ominous_has_heavy_core() {
        assert!(ominous_vault_loot()
            .iter()
            .any(|(i, _, _, _)| *i == "minecraft:heavy_core"));
    }
}
