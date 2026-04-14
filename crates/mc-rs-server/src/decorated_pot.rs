//! Decorated pot — 4 sherds, potential inventory (1.21+).

#[derive(Debug, Clone)]
pub struct DecoratedPot {
    pub sherds: [Option<&'static str>; 4], // N/E/S/W
    pub inventory: Option<(u16, u16)>,
    pub cracked: bool,
}

/// Valid sherds (vanilla).
pub fn valid_sherds() -> &'static [&'static str] {
    &[
        "minecraft:angler_pottery_sherd",
        "minecraft:archer_pottery_sherd",
        "minecraft:arms_up_pottery_sherd",
        "minecraft:blade_pottery_sherd",
        "minecraft:brewer_pottery_sherd",
        "minecraft:burn_pottery_sherd",
        "minecraft:danger_pottery_sherd",
        "minecraft:explorer_pottery_sherd",
        "minecraft:flow_pottery_sherd",
        "minecraft:friend_pottery_sherd",
        "minecraft:guster_pottery_sherd",
        "minecraft:heart_pottery_sherd",
        "minecraft:heartbreak_pottery_sherd",
        "minecraft:howl_pottery_sherd",
        "minecraft:miner_pottery_sherd",
        "minecraft:mourner_pottery_sherd",
        "minecraft:plenty_pottery_sherd",
        "minecraft:prize_pottery_sherd",
        "minecraft:scrape_pottery_sherd",
        "minecraft:sheaf_pottery_sherd",
        "minecraft:shelter_pottery_sherd",
        "minecraft:skull_pottery_sherd",
        "minecraft:snort_pottery_sherd",
    ]
}

impl DecoratedPot {
    pub fn new() -> Self {
        Self {
            sherds: [None; 4],
            inventory: None,
            cracked: false,
        }
    }

    pub fn is_valid_sherd(name: &str) -> bool {
        valid_sherds().contains(&name)
    }

    /// Pot breaks when hit with non-tool item — drops sherds.
    pub fn break_returns_sherds(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        for slot in &self.sherds {
            if let Some(name) = slot {
                out.push(*name);
            }
        }
        out
    }
}

impl Default for DecoratedPot {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skull_is_valid() {
        assert!(DecoratedPot::is_valid_sherd("minecraft:skull_pottery_sherd"));
    }

    #[test]
    fn break_drops_sherds() {
        let mut p = DecoratedPot::new();
        p.sherds[0] = Some("minecraft:heart_pottery_sherd");
        assert_eq!(p.break_returns_sherds().len(), 1);
    }
}
