//! Armor trim patterns — template item → name mapping.

pub const ALL_TRIMS: &[(&str, &str)] = &[
    ("minecraft:coast_armor_trim_smithing_template", "coast"),
    ("minecraft:sentry_armor_trim_smithing_template", "sentry"),
    ("minecraft:dune_armor_trim_smithing_template", "dune"),
    ("minecraft:wild_armor_trim_smithing_template", "wild"),
    ("minecraft:ward_armor_trim_smithing_template", "ward"),
    ("minecraft:eye_armor_trim_smithing_template", "eye"),
    ("minecraft:vex_armor_trim_smithing_template", "vex"),
    ("minecraft:tide_armor_trim_smithing_template", "tide"),
    ("minecraft:snout_armor_trim_smithing_template", "snout"),
    ("minecraft:rib_armor_trim_smithing_template", "rib"),
    ("minecraft:spire_armor_trim_smithing_template", "spire"),
    ("minecraft:silence_armor_trim_smithing_template", "silence"),
    ("minecraft:wayfinder_armor_trim_smithing_template", "wayfinder"),
    ("minecraft:shaper_armor_trim_smithing_template", "shaper"),
    ("minecraft:raiser_armor_trim_smithing_template", "raiser"),
    ("minecraft:host_armor_trim_smithing_template", "host"),
    ("minecraft:flow_armor_trim_smithing_template", "flow"),
    ("minecraft:bolt_armor_trim_smithing_template", "bolt"),
];

pub fn trim_name(template: &str) -> Option<&'static str> {
    ALL_TRIMS.iter().find(|(t, _)| *t == template).map(|(_, n)| *n)
}

/// Smithing duplicates template with material + base block.
pub fn duplicate_template_ingredients(template: &str) -> (&'static str, &'static str) {
    let base_block = match trim_name(template) {
        Some("coast") => "minecraft:cobblestone",
        Some("sentry") => "minecraft:cobblestone",
        Some("dune") => "minecraft:sandstone",
        Some("wild") => "minecraft:mossy_cobblestone",
        Some("ward") => "minecraft:cobbled_deepslate",
        Some("eye") => "minecraft:end_stone",
        Some("vex") => "minecraft:cobblestone",
        Some("tide") => "minecraft:prismarine",
        Some("snout") => "minecraft:blackstone",
        Some("rib") => "minecraft:netherrack",
        Some("spire") => "minecraft:purpur_block",
        Some("silence") => "minecraft:cobbled_deepslate",
        Some("wayfinder") => "minecraft:terracotta",
        Some("shaper") => "minecraft:terracotta",
        Some("raiser") => "minecraft:terracotta",
        Some("host") => "minecraft:terracotta",
        Some("flow") => "minecraft:breeze_rod",
        Some("bolt") => "minecraft:breeze_rod",
        _ => "minecraft:diamond",
    };
    ("minecraft:diamond", base_block)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coast_is_trim() {
        assert_eq!(trim_name("minecraft:coast_armor_trim_smithing_template"), Some("coast"));
    }
}
