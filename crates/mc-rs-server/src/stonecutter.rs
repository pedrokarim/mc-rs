//! Stonecutter — 1 input → multiple output options.

#[derive(Debug, Clone)]
pub struct StonecutterRecipe {
    pub input: &'static str,
    pub output: &'static str,
    pub count: u32,
}

/// Subset of stonecutter recipes (stone family, deepslate, etc).
pub fn stonecutter_recipes() -> Vec<StonecutterRecipe> {
    let families: &[(&str, &[&str])] = &[
        (
            "minecraft:stone",
            &[
                "minecraft:stone_bricks",
                "minecraft:chiseled_stone_bricks",
                "minecraft:cracked_stone_bricks",
                "minecraft:stone_slab",
                "minecraft:stone_stairs",
                "minecraft:stone_wall",
            ],
        ),
        (
            "minecraft:cobblestone",
            &[
                "minecraft:cobblestone_slab",
                "minecraft:cobblestone_stairs",
                "minecraft:cobblestone_wall",
            ],
        ),
        (
            "minecraft:sandstone",
            &[
                "minecraft:cut_sandstone",
                "minecraft:chiseled_sandstone",
                "minecraft:sandstone_slab",
                "minecraft:sandstone_stairs",
                "minecraft:sandstone_wall",
            ],
        ),
        (
            "minecraft:blackstone",
            &[
                "minecraft:chiseled_polished_blackstone",
                "minecraft:polished_blackstone",
                "minecraft:polished_blackstone_slab",
                "minecraft:polished_blackstone_stairs",
                "minecraft:polished_blackstone_wall",
            ],
        ),
        (
            "minecraft:deepslate",
            &[
                "minecraft:cobbled_deepslate",
                "minecraft:polished_deepslate",
                "minecraft:deepslate_bricks",
                "minecraft:deepslate_tiles",
            ],
        ),
    ];
    families
        .iter()
        .flat_map(|(input, outs)| {
            outs.iter().map(move |o| StonecutterRecipe {
                input,
                output: o,
                count: 1,
            })
        })
        .collect()
}

pub fn outputs_for(input: &str) -> Vec<&'static str> {
    stonecutter_recipes()
        .into_iter()
        .filter(|r| r.input == input)
        .map(|r| r.output)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stone_has_outputs() {
        let outs = outputs_for("minecraft:stone");
        assert!(!outs.is_empty());
    }
}
