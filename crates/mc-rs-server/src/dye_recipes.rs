//! Dye crafting (combine 2 dyes).

use crate::dyes::DyeColor;

pub fn combine(a: DyeColor, b: DyeColor) -> Option<DyeColor> {
    use DyeColor::*;
    Some(match (a, b) {
        (White, Black) | (Black, White) => Gray,
        (White, Gray) | (Gray, White) => LightGray,
        (Red, Yellow) | (Yellow, Red) => Orange,
        (Red, White) | (White, Red) => Pink,
        (Blue, Green) | (Green, Blue) => Cyan,
        (Blue, Red) | (Red, Blue) => Purple,
        (Blue, White) | (White, Blue) => LightBlue,
        (Yellow, Green) | (Green, Yellow) => Lime,
        (Pink, Purple) | (Purple, Pink) => Magenta,
        _ => return None,
    })
}

/// Dye applied to wool/glass/concrete/terracotta.
pub fn dyeable_blocks() -> &'static [&'static str] {
    &[
        "minecraft:wool",
        "minecraft:carpet",
        "minecraft:stained_glass",
        "minecraft:stained_glass_pane",
        "minecraft:concrete",
        "minecraft:concrete_powder",
        "minecraft:terracotta",
        "minecraft:bed",
        "minecraft:candle",
        "minecraft:banner",
        "minecraft:shulker_box",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_yellow_orange() {
        assert_eq!(
            combine(DyeColor::Red, DyeColor::Yellow),
            Some(DyeColor::Orange)
        );
    }
}
