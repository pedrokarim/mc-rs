//! Smithing templates (1.20+) — trim + netherite upgrade patterns.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithingTemplate {
    NetheriteUpgrade,
    CoastArmorTrim,
    DuneArmorTrim,
    EyeArmorTrim,
    HostArmorTrim,
    RaiserArmorTrim,
    RibArmorTrim,
    SentryArmorTrim,
    ShaperArmorTrim,
    SilenceArmorTrim,
    SnoutArmorTrim,
    SpireArmorTrim,
    TideArmorTrim,
    VexArmorTrim,
    WardArmorTrim,
    WayfinderArmorTrim,
    WildArmorTrim,
}

impl SmithingTemplate {
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::NetheriteUpgrade => "netherite_upgrade_smithing_template",
            Self::CoastArmorTrim => "coast_armor_trim_smithing_template",
            Self::DuneArmorTrim => "dune_armor_trim_smithing_template",
            Self::EyeArmorTrim => "eye_armor_trim_smithing_template",
            Self::HostArmorTrim => "host_armor_trim_smithing_template",
            Self::RaiserArmorTrim => "raiser_armor_trim_smithing_template",
            Self::RibArmorTrim => "rib_armor_trim_smithing_template",
            Self::SentryArmorTrim => "sentry_armor_trim_smithing_template",
            Self::ShaperArmorTrim => "shaper_armor_trim_smithing_template",
            Self::SilenceArmorTrim => "silence_armor_trim_smithing_template",
            Self::SnoutArmorTrim => "snout_armor_trim_smithing_template",
            Self::SpireArmorTrim => "spire_armor_trim_smithing_template",
            Self::TideArmorTrim => "tide_armor_trim_smithing_template",
            Self::VexArmorTrim => "vex_armor_trim_smithing_template",
            Self::WardArmorTrim => "ward_armor_trim_smithing_template",
            Self::WayfinderArmorTrim => "wayfinder_armor_trim_smithing_template",
            Self::WildArmorTrim => "wild_armor_trim_smithing_template",
        }
    }

    pub fn is_trim(&self) -> bool {
        !matches!(self, Self::NetheriteUpgrade)
    }
}

/// Materials compatibles pour armor trim.
pub fn armor_trim_materials() -> &'static [&'static str] {
    &[
        "minecraft:iron_ingot",
        "minecraft:gold_ingot",
        "minecraft:diamond",
        "minecraft:emerald",
        "minecraft:netherite_ingot",
        "minecraft:redstone",
        "minecraft:lapis_lazuli",
        "minecraft:amethyst_shard",
        "minecraft:quartz",
        "minecraft:copper_ingot",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netherite_is_upgrade_not_trim() {
        assert!(!SmithingTemplate::NetheriteUpgrade.is_trim());
        assert!(SmithingTemplate::DuneArmorTrim.is_trim());
    }

    #[test]
    fn trim_materials_include_diamond() {
        assert!(armor_trim_materials().contains(&"minecraft:diamond"));
    }
}
