//! Tables de trading vanilla (villagers + économies piglins)
//! depuis `data/trading.json` (consolidé à partir de
//! `.reference/bedrock-samples/behavior_pack/trading/`, Mojang 1.26.10.4).
//!
//! Format Mojang :
//! ```json
//! {
//!   "tiers": [
//!     {
//!       "trades": [
//!         {
//!           "wants": [{ "item": "minecraft:wheat", "quantity": {"min": 18, "max": 22} }],
//!           "gives": [{ "item": "minecraft:emerald" }]
//!         }
//!       ]
//!     }
//!   ]
//! }
//! ```

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

const TRADING_JSON: &str = include_str!("../data/trading.json");

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum IntOrRange {
    Int(i32),
    Range { min: i32, max: i32 },
}

impl IntOrRange {
    pub fn as_range(&self) -> (i32, i32) {
        match self {
            IntOrRange::Int(v) => (*v, *v),
            IntOrRange::Range { min, max } => (*min, *max),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TradeItem {
    #[serde(default)]
    pub item: Option<String>,
    #[serde(default)]
    pub quantity: Option<IntOrRange>,
    #[serde(flatten)]
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TradeItemOrArray {
    Single(TradeItem),
    Array(Vec<TradeItem>),
}

#[derive(Deserialize, Debug, Clone)]
pub struct Trade {
    #[serde(default)]
    pub wants: Vec<TradeItemOrArray>,
    #[serde(default)]
    pub gives: Vec<TradeItemOrArray>,
    #[serde(flatten)]
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Trade {
    pub fn flat_wants(&self) -> Vec<&TradeItem> {
        let mut v = Vec::new();
        for w in &self.wants {
            match w {
                TradeItemOrArray::Single(t) => v.push(t),
                TradeItemOrArray::Array(arr) => v.extend(arr.iter()),
            }
        }
        v
    }
    pub fn flat_gives(&self) -> Vec<&TradeItem> {
        let mut v = Vec::new();
        for g in &self.gives {
            match g {
                TradeItemOrArray::Single(t) => v.push(t),
                TradeItemOrArray::Array(arr) => v.extend(arr.iter()),
            }
        }
        v
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Tier {
    #[serde(default)]
    pub trades: Vec<Trade>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TradeTable {
    #[serde(default)]
    pub tiers: Vec<Tier>,
}

static TRADING: LazyLock<HashMap<String, TradeTable>> =
    LazyLock::new(|| serde_json::from_str(TRADING_JSON).expect("valid trading.json"));

pub fn for_profession(name: &str) -> Option<&'static TradeTable> {
    TRADING.get(name)
}

pub fn count() -> usize {
    TRADING.len()
}

pub fn all_professions() -> Vec<&'static String> {
    TRADING.keys().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loaded_trades() {
        assert!(count() >= 20, "count={}", count());
    }

    #[test]
    fn farmer_has_tiers() {
        let t = for_profession("farmer_trades").expect("farmer trades");
        assert!(!t.tiers.is_empty());
        assert!(!t.tiers[0].trades.is_empty());
    }

    #[test]
    fn farmer_first_trade_wants_wheat() {
        let t = for_profession("farmer_trades").expect("farmer trades");
        let first = &t.tiers[0].trades[0];
        let wants = first.flat_wants();
        assert!(wants
            .iter()
            .any(|w| w.item.as_deref() == Some("minecraft:wheat")));
    }
}
