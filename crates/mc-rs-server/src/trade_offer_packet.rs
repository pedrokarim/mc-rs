//! TradeOffer packet structure.

#[derive(Debug, Clone)]
pub struct TradeOffer {
    pub buy_a: TradeItem,
    pub buy_b: Option<TradeItem>,
    pub sell: TradeItem,
    pub uses: u32,
    pub max_uses: u32,
    pub xp_player: u32,
    pub xp_villager: u32,
    pub price_multiplier: f32,
    pub demand: i32,
    pub special_price: i32,
    pub required_level: u8,
}

#[derive(Debug, Clone)]
pub struct TradeItem {
    pub item_id: u16,
    pub count: u16,
    pub data: u16,
    pub nbt: Option<Vec<u8>>,
}

impl TradeOffer {
    pub fn is_locked(&self) -> bool {
        self.uses >= self.max_uses
    }

    pub fn current_buy_price(&self) -> u16 {
        let demand_increase = (self.demand as f32 * self.price_multiplier).max(0.0);
        let base = self.buy_a.count as f32 + demand_increase + self.special_price as f32;
        base.max(1.0).min(64.0) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_after_uses() {
        let offer = TradeOffer {
            buy_a: TradeItem {
                item_id: 1,
                count: 1,
                data: 0,
                nbt: None,
            },
            buy_b: None,
            sell: TradeItem {
                item_id: 2,
                count: 1,
                data: 0,
                nbt: None,
            },
            uses: 5,
            max_uses: 5,
            xp_player: 1,
            xp_villager: 1,
            price_multiplier: 0.05,
            demand: 0,
            special_price: 0,
            required_level: 1,
        };
        assert!(offer.is_locked());
    }
}
