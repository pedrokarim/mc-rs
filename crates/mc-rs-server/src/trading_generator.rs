//! Villager trade generator — vanilla trade tables per profession + level.

use crate::trading::{MerchantOffer, VillagerProfession};
use mc_rs_proto::packets::player::ItemStack;

/// Retourne les offers pour (profession, level 1-5) en noms d'items.
/// Format : (buy_a_name, buy_a_count, buy_b_name_opt, buy_b_count, sell_name, sell_count, xp_player, xp_villager)
pub fn trades_for(profession: VillagerProfession, level: u8) -> Vec<TradeTemplate> {
    match (profession, level) {
        // Farmer
        (VillagerProfession::Farmer, 1) => vec![
            TradeTemplate { buy_a: "minecraft:wheat", buy_a_count: 20, buy_b: None, sell: "minecraft:emerald", sell_count: 1, xp_player: 1, xp_villager: 2 },
            TradeTemplate { buy_a: "minecraft:potato", buy_a_count: 26, buy_b: None, sell: "minecraft:emerald", sell_count: 1, xp_player: 1, xp_villager: 2 },
            TradeTemplate { buy_a: "minecraft:carrot", buy_a_count: 22, buy_b: None, sell: "minecraft:emerald", sell_count: 1, xp_player: 1, xp_villager: 2 },
            TradeTemplate { buy_a: "minecraft:beetroot", buy_a_count: 15, buy_b: None, sell: "minecraft:emerald", sell_count: 1, xp_player: 1, xp_villager: 2 },
            TradeTemplate { buy_a: "minecraft:emerald", buy_a_count: 1, buy_b: None, sell: "minecraft:bread", sell_count: 6, xp_player: 1, xp_villager: 1 },
        ],
        (VillagerProfession::Farmer, 2) => vec![
            TradeTemplate { buy_a: "minecraft:pumpkin", buy_a_count: 6, buy_b: None, sell: "minecraft:emerald", sell_count: 1, xp_player: 1, xp_villager: 10 },
            TradeTemplate { buy_a: "minecraft:emerald", buy_a_count: 1, buy_b: None, sell: "minecraft:pumpkin_pie", sell_count: 4, xp_player: 1, xp_villager: 5 },
            TradeTemplate { buy_a: "minecraft:emerald", buy_a_count: 1, buy_b: None, sell: "minecraft:apple", sell_count: 4, xp_player: 1, xp_villager: 5 },
        ],
        (VillagerProfession::Farmer, 3) => vec![
            TradeTemplate { buy_a: "minecraft:melon", buy_a_count: 4, buy_b: None, sell: "minecraft:emerald", sell_count: 1, xp_player: 1, xp_villager: 20 },
            TradeTemplate { buy_a: "minecraft:emerald", buy_a_count: 1, buy_b: None, sell: "minecraft:cookie", sell_count: 18, xp_player: 1, xp_villager: 10 },
        ],
        (VillagerProfession::Farmer, 4) => vec![
            TradeTemplate { buy_a: "minecraft:emerald", buy_a_count: 3, buy_b: None, sell: "minecraft:cake", sell_count: 1, xp_player: 1, xp_villager: 15 },
            TradeTemplate { buy_a: "minecraft:emerald", buy_a_count: 1, buy_b: None, sell: "minecraft:suspicious_stew", sell_count: 1, xp_player: 1, xp_villager: 15 },
        ],
        (VillagerProfession::Farmer, 5) => vec![
            TradeTemplate { buy_a: "minecraft:emerald", buy_a_count: 3, buy_b: None, sell: "minecraft:golden_carrot", sell_count: 3, xp_player: 1, xp_villager: 30 },
            TradeTemplate { buy_a: "minecraft:emerald", buy_a_count: 4, buy_b: None, sell: "minecraft:glistering_melon_slice", sell_count: 3, xp_player: 1, xp_villager: 30 },
        ],
        // Librarian (partial)
        (VillagerProfession::Librarian, 1) => vec![
            TradeTemplate { buy_a: "minecraft:paper", buy_a_count: 24, buy_b: None, sell: "minecraft:emerald", sell_count: 1, xp_player: 1, xp_villager: 2 },
            TradeTemplate { buy_a: "minecraft:emerald", buy_a_count: 9, buy_b: Some("minecraft:book"), sell: "minecraft:enchanted_book", sell_count: 1, xp_player: 1, xp_villager: 10 },
            TradeTemplate { buy_a: "minecraft:emerald", buy_a_count: 4, buy_b: None, sell: "minecraft:bookshelf", sell_count: 1, xp_player: 1, xp_villager: 5 },
        ],
        (VillagerProfession::Cleric, 1) => vec![
            TradeTemplate { buy_a: "minecraft:rotten_flesh", buy_a_count: 32, buy_b: None, sell: "minecraft:emerald", sell_count: 1, xp_player: 1, xp_villager: 2 },
            TradeTemplate { buy_a: "minecraft:emerald", buy_a_count: 1, buy_b: None, sell: "minecraft:redstone", sell_count: 2, xp_player: 1, xp_villager: 1 },
        ],
        (VillagerProfession::Armorer, 1) => vec![
            TradeTemplate { buy_a: "minecraft:coal", buy_a_count: 15, buy_b: None, sell: "minecraft:emerald", sell_count: 1, xp_player: 1, xp_villager: 2 },
            TradeTemplate { buy_a: "minecraft:emerald", buy_a_count: 5, buy_b: None, sell: "minecraft:iron_leggings", sell_count: 1, xp_player: 1, xp_villager: 1 },
        ],
        _ => vec![],
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TradeTemplate {
    pub buy_a: &'static str,
    pub buy_a_count: u16,
    pub buy_b: Option<&'static str>,
    pub sell: &'static str,
    pub sell_count: u16,
    pub xp_player: u32,
    pub xp_villager: u32,
}

impl TradeTemplate {
    /// Convertit en MerchantOffer runtime (nécessite registry résolue).
    pub fn to_merchant_offer(&self, max_uses: u32) -> Option<MerchantOffer> {
        use crate::item_registry::network_id;
        let buy_a_id = network_id(self.buy_a)?;
        let sell_id = network_id(self.sell)?;
        let buy_b = if let Some(name) = self.buy_b {
            Some(ItemStack::new(network_id(name)?, 1, 0))
        } else {
            None
        };
        Some(MerchantOffer {
            buy_a: ItemStack::new(buy_a_id, self.buy_a_count, 0),
            buy_b,
            sell: ItemStack::new(sell_id, self.sell_count, 0),
            uses: 0,
            max_uses,
            xp_given_to_player: self.xp_player,
            xp_given_to_villager: self.xp_villager,
            price_multiplier: 0.05,
            required_level: 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn farmer_level_1_has_trades() {
        let t = trades_for(VillagerProfession::Farmer, 1);
        assert!(!t.is_empty());
    }

    #[test]
    fn unknown_profession_no_trades() {
        let t = trades_for(VillagerProfession::Nitwit, 1);
        assert!(t.is_empty());
    }
}
