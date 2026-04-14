//! Cat morning gifts — PMMP like. Chats tamed déposent des cadeaux au réveil
//! du joueur s'ils dorment dans un lit proche.

use rand::Rng;

/// Chance que le chat apporte un cadeau au réveil (vanilla = ~70%).
pub const MORNING_GIFT_CHANCE: f32 = 0.7;

/// Loot table des cadeaux chat.
pub fn morning_gift_loot() -> &'static [(&'static str, u32)] {
    &[
        ("minecraft:rabbit_hide", 1),
        ("minecraft:rabbit_foot", 1),
        ("minecraft:string", 1),
        ("minecraft:feather", 1),
        ("minecraft:rotten_flesh", 1),
        ("minecraft:phantom_membrane", 1),
    ]
}

pub fn roll_morning_gift() -> Option<&'static str> {
    let mut rng = rand::thread_rng();
    if rng.gen::<f32>() > MORNING_GIFT_CHANCE {
        return None;
    }
    let loot = morning_gift_loot();
    let total: u32 = loot.iter().map(|(_, w)| w).sum();
    let mut roll = rng.gen_range(0..total);
    for (name, weight) in loot {
        if roll < *weight {
            return Some(name);
        }
        roll -= *weight;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gift_loot_not_empty() {
        assert!(!morning_gift_loot().is_empty());
    }
}
