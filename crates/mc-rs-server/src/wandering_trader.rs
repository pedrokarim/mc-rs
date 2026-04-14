//! Wandering Trader — villageois nomade + 2 llamas.

#[derive(Debug, Clone)]
pub struct WanderingTrader {
    pub life_ticks: u32,
    pub trades: Vec<TradeSimple>,
    pub drinking_ticks: u32,
    pub llamas: Vec<u64>, // Entity IDs of accompanying llamas
}

#[derive(Debug, Clone, Copy)]
pub struct TradeSimple {
    pub buy_emeralds: u32,
    pub sell_item: &'static str,
    pub sell_count: u32,
    pub max_uses: u32,
}

/// Life duration (40-60 min = 48000-72000 ticks).
pub const LIFE_DURATION_MIN: u32 = 48_000;
pub const LIFE_DURATION_MAX: u32 = 72_000;

/// Vanilla wandering trader common trades.
pub fn common_trades() -> Vec<TradeSimple> {
    vec![
        TradeSimple { buy_emeralds: 1, sell_item: "minecraft:acacia_sapling", sell_count: 1, max_uses: 8 },
        TradeSimple { buy_emeralds: 1, sell_item: "minecraft:birch_sapling", sell_count: 1, max_uses: 8 },
        TradeSimple { buy_emeralds: 1, sell_item: "minecraft:dark_oak_sapling", sell_count: 1, max_uses: 8 },
        TradeSimple { buy_emeralds: 1, sell_item: "minecraft:jungle_sapling", sell_count: 1, max_uses: 8 },
        TradeSimple { buy_emeralds: 5, sell_item: "minecraft:oak_sapling", sell_count: 1, max_uses: 8 },
        TradeSimple { buy_emeralds: 5, sell_item: "minecraft:spruce_sapling", sell_count: 1, max_uses: 8 },
        TradeSimple { buy_emeralds: 1, sell_item: "minecraft:red_dye", sell_count: 3, max_uses: 12 },
        TradeSimple { buy_emeralds: 1, sell_item: "minecraft:white_dye", sell_count: 3, max_uses: 12 },
        TradeSimple { buy_emeralds: 1, sell_item: "minecraft:blue_dye", sell_count: 3, max_uses: 12 },
        TradeSimple { buy_emeralds: 1, sell_item: "minecraft:green_dye", sell_count: 3, max_uses: 12 },
    ]
}

pub fn rare_trades() -> Vec<TradeSimple> {
    vec![
        TradeSimple { buy_emeralds: 1, sell_item: "minecraft:potion", sell_count: 1, max_uses: 1 },
        TradeSimple { buy_emeralds: 6, sell_item: "minecraft:blue_ice", sell_count: 1, max_uses: 1 },
        TradeSimple { buy_emeralds: 1, sell_item: "minecraft:podzol", sell_count: 3, max_uses: 1 },
    ]
}

impl WanderingTrader {
    pub fn new() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let life = rng.gen_range(LIFE_DURATION_MIN..=LIFE_DURATION_MAX);
        let mut trades = common_trades();
        trades.extend(rare_trades().into_iter().take(1));
        Self {
            life_ticks: life,
            trades,
            drinking_ticks: 0,
            llamas: Vec::new(),
        }
    }

    pub fn tick(&mut self) -> bool {
        if self.life_ticks > 0 {
            self.life_ticks -= 1;
        }
        self.life_ticks == 0
    }

    /// Wandering trader drinks invisibility at night.
    pub fn drink_invisibility(&mut self) {
        self.drinking_ticks = 60;
    }
}

impl Default for WanderingTrader {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn despawns_after_life() {
        let mut w = WanderingTrader::new();
        w.life_ticks = 1;
        assert!(w.tick());
    }

    #[test]
    fn has_trades() {
        let w = WanderingTrader::new();
        assert!(!w.trades.is_empty());
    }
}
