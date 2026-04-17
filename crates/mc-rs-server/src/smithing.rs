//! Smithing table — upgrade nétherite + trim armor.

#[derive(Debug, Clone)]
pub struct SmithingRecipe {
    pub template: Option<&'static str>, // e.g. netherite upgrade template
    pub base: &'static str,             // diamond armor/tool
    pub addition: &'static str,         // netherite ingot
    pub result: &'static str,
}

/// Netherite upgrades.
pub fn netherite_upgrades() -> Vec<SmithingRecipe> {
    let pairs: &[(&str, &str)] = &[
        ("minecraft:diamond_sword", "minecraft:netherite_sword"),
        ("minecraft:diamond_pickaxe", "minecraft:netherite_pickaxe"),
        ("minecraft:diamond_axe", "minecraft:netherite_axe"),
        ("minecraft:diamond_shovel", "minecraft:netherite_shovel"),
        ("minecraft:diamond_hoe", "minecraft:netherite_hoe"),
        ("minecraft:diamond_helmet", "minecraft:netherite_helmet"),
        (
            "minecraft:diamond_chestplate",
            "minecraft:netherite_chestplate",
        ),
        ("minecraft:diamond_leggings", "minecraft:netherite_leggings"),
        ("minecraft:diamond_boots", "minecraft:netherite_boots"),
    ];
    pairs
        .iter()
        .map(|(a, b)| SmithingRecipe {
            template: Some("minecraft:netherite_upgrade_smithing_template"),
            base: a,
            addition: "minecraft:netherite_ingot",
            result: b,
        })
        .collect()
}

/// Armor trim templates.
pub fn trim_templates() -> &'static [&'static str] {
    &[
        "minecraft:coast_armor_trim_smithing_template",
        "minecraft:sentry_armor_trim_smithing_template",
        "minecraft:dune_armor_trim_smithing_template",
        "minecraft:wild_armor_trim_smithing_template",
        "minecraft:ward_armor_trim_smithing_template",
        "minecraft:eye_armor_trim_smithing_template",
        "minecraft:vex_armor_trim_smithing_template",
        "minecraft:tide_armor_trim_smithing_template",
        "minecraft:snout_armor_trim_smithing_template",
        "minecraft:rib_armor_trim_smithing_template",
        "minecraft:spire_armor_trim_smithing_template",
        "minecraft:silence_armor_trim_smithing_template",
        "minecraft:wayfinder_armor_trim_smithing_template",
        "minecraft:shaper_armor_trim_smithing_template",
        "minecraft:raiser_armor_trim_smithing_template",
        "minecraft:host_armor_trim_smithing_template",
        "minecraft:flow_armor_trim_smithing_template",
        "minecraft:bolt_armor_trim_smithing_template",
    ]
}

/// Materials usable for trim.
pub fn trim_materials() -> &'static [&'static str] {
    &[
        "minecraft:iron_ingot",
        "minecraft:copper_ingot",
        "minecraft:gold_ingot",
        "minecraft:lapis_lazuli",
        "minecraft:emerald",
        "minecraft:diamond",
        "minecraft:netherite_ingot",
        "minecraft:redstone",
        "minecraft:amethyst_shard",
        "minecraft:quartz",
        "minecraft:resin_brick",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_netherite_upgrades() {
        assert!(!netherite_upgrades().is_empty());
    }

    #[test]
    fn trim_templates_non_empty() {
        assert!(!trim_templates().is_empty());
    }
}
