//! Smoker + BlastFurnace — variantes de furnace.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmokerType {
    Smoker,        // 200 ticks (double speed), food only
    BlastFurnace,  // 200 ticks (double speed), ores/iron only
}

/// Smoker/BlastFurnace cooks 2x faster than furnace (200 vs 400 default ticks).
pub const COOK_TICKS: u32 = 100;

/// Recipes applicable per smoker type.
pub fn smoker_recipes() -> &'static [(&'static str, &'static str)] {
    &[
        ("minecraft:raw_beef", "minecraft:cooked_beef"),
        ("minecraft:raw_chicken", "minecraft:cooked_chicken"),
        ("minecraft:raw_porkchop", "minecraft:cooked_porkchop"),
        ("minecraft:raw_rabbit", "minecraft:cooked_rabbit"),
        ("minecraft:raw_mutton", "minecraft:cooked_mutton"),
        ("minecraft:raw_cod", "minecraft:cooked_cod"),
        ("minecraft:raw_salmon", "minecraft:cooked_salmon"),
        ("minecraft:potato", "minecraft:baked_potato"),
        ("minecraft:kelp", "minecraft:dried_kelp"),
    ]
}

pub fn blast_furnace_recipes() -> &'static [(&'static str, &'static str)] {
    &[
        ("minecraft:iron_ore", "minecraft:iron_ingot"),
        ("minecraft:gold_ore", "minecraft:gold_ingot"),
        ("minecraft:copper_ore", "minecraft:copper_ingot"),
        ("minecraft:deepslate_iron_ore", "minecraft:iron_ingot"),
        ("minecraft:deepslate_gold_ore", "minecraft:gold_ingot"),
        ("minecraft:deepslate_copper_ore", "minecraft:copper_ingot"),
        ("minecraft:raw_iron", "minecraft:iron_ingot"),
        ("minecraft:raw_gold", "minecraft:gold_ingot"),
        ("minecraft:raw_copper", "minecraft:copper_ingot"),
        ("minecraft:iron_sword", "minecraft:iron_nugget"),
        ("minecraft:iron_pickaxe", "minecraft:iron_nugget"),
        ("minecraft:iron_axe", "minecraft:iron_nugget"),
        ("minecraft:iron_shovel", "minecraft:iron_nugget"),
        ("minecraft:iron_hoe", "minecraft:iron_nugget"),
        ("minecraft:iron_helmet", "minecraft:iron_nugget"),
        ("minecraft:iron_chestplate", "minecraft:iron_nugget"),
        ("minecraft:iron_leggings", "minecraft:iron_nugget"),
        ("minecraft:iron_boots", "minecraft:iron_nugget"),
    ]
}

pub fn result_for(smoker_type: SmokerType, input: &str) -> Option<&'static str> {
    let table = match smoker_type {
        SmokerType::Smoker => smoker_recipes(),
        SmokerType::BlastFurnace => blast_furnace_recipes(),
    };
    table.iter().find(|(i, _)| *i == input).map(|(_, o)| *o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoker_cooks_beef() {
        assert_eq!(
            result_for(SmokerType::Smoker, "minecraft:raw_beef"),
            Some("minecraft:cooked_beef")
        );
    }

    #[test]
    fn blast_furnace_smelts_iron() {
        assert_eq!(
            result_for(SmokerType::BlastFurnace, "minecraft:iron_ore"),
            Some("minecraft:iron_ingot")
        );
    }

    #[test]
    fn smoker_cant_smelt_iron() {
        assert!(result_for(SmokerType::Smoker, "minecraft:iron_ore").is_none());
    }
}
