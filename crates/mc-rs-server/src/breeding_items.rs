//! Breeding items — items qui déclenchent love mode par mob.

use crate::mob_ai::MobKind;

pub fn breeding_item(mob: MobKind) -> &'static [&'static str] {
    match mob {
        MobKind::Cow | MobKind::Sheep => &["minecraft:wheat"],
        MobKind::Pig => &["minecraft:carrot", "minecraft:potato", "minecraft:beetroot"],
        MobKind::Chicken => &[
            "minecraft:wheat_seeds",
            "minecraft:melon_seeds",
            "minecraft:pumpkin_seeds",
            "minecraft:beetroot_seeds",
        ],
        MobKind::Rabbit => &[
            "minecraft:dandelion",
            "minecraft:carrot",
            "minecraft:golden_carrot",
        ],
        MobKind::Wolf => &[
            "minecraft:beef",
            "minecraft:porkchop",
            "minecraft:chicken",
            "minecraft:rabbit",
            "minecraft:mutton",
            "minecraft:cooked_beef",
            "minecraft:cooked_porkchop",
            "minecraft:cooked_chicken",
            "minecraft:cooked_rabbit",
            "minecraft:cooked_mutton",
        ],
        MobKind::Cat | MobKind::Ocelot => &["minecraft:raw_cod", "minecraft:raw_salmon"],
        MobKind::Horse | MobKind::Donkey => &["minecraft:golden_apple", "minecraft:golden_carrot"],
        MobKind::Llama => &["minecraft:hay_block"],
        MobKind::Fox => &["minecraft:sweet_berries", "minecraft:glow_berries"],
        MobKind::Panda => &["minecraft:bamboo"],
        MobKind::Turtle => &["minecraft:seagrass"],
        MobKind::Dolphin => &["minecraft:raw_cod", "minecraft:cooked_cod"],
        MobKind::Villager => &[
            "minecraft:bread",
            "minecraft:carrot",
            "minecraft:potato",
            "minecraft:beetroot",
        ],
        MobKind::Parrot => &[
            "minecraft:wheat_seeds",
            "minecraft:melon_seeds",
            "minecraft:pumpkin_seeds",
            "minecraft:beetroot_seeds",
        ],
        _ => &[],
    }
}

/// Temping items (attirer sans love mode, pour suivre joueur).
pub fn temping_items(mob: MobKind) -> &'static [&'static str] {
    match mob {
        MobKind::Pig => &["minecraft:carrot", "minecraft:potato", "minecraft:beetroot"],
        MobKind::Sheep | MobKind::Cow => &["minecraft:wheat"],
        _ => breeding_item(mob),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cow_likes_wheat() {
        let items = breeding_item(MobKind::Cow);
        assert!(items.contains(&"minecraft:wheat"));
    }

    #[test]
    fn wolf_likes_meat() {
        let items = breeding_item(MobKind::Wolf);
        assert!(items.contains(&"minecraft:beef"));
    }
}
