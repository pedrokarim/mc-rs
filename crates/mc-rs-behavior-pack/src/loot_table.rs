//! Loot table parsing and evaluation (loot_tables/*.json).

use rand::Rng;
use serde::Deserialize;

/// A loot table with one or more pools.
#[derive(Debug, Clone, Deserialize)]
pub struct LootTableFile {
    #[serde(default)]
    pub pools: Vec<LootPool>,
}

/// A pool of loot entries rolled a number of times.
#[derive(Debug, Clone, Deserialize)]
pub struct LootPool {
    pub rolls: RollsValue,
    pub entries: Vec<LootEntry>,
}

/// Number of rolls — fixed or random range.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RollsValue {
    Fixed(u32),
    Range { min: u32, max: u32 },
}

impl RollsValue {
    /// Evaluate the number of rolls.
    pub fn roll(&self) -> u32 {
        match self {
            RollsValue::Fixed(n) => *n,
            RollsValue::Range { min, max } => {
                let mut rng = rand::thread_rng();
                rng.gen_range(*min..=*max)
            }
        }
    }
}

/// A single entry in a loot pool.
#[derive(Debug, Clone, Deserialize)]
pub struct LootEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default = "default_weight")]
    pub weight: u32,
    #[serde(default)]
    pub functions: Vec<LootFunction>,
}

/// A function that modifies the loot result.
#[derive(Debug, Clone, Deserialize)]
pub struct LootFunction {
    pub function: String,
    #[serde(default)]
    pub count: Option<CountValue>,
}

/// Count value — fixed or random range.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CountValue {
    Fixed(u32),
    Range { min: u32, max: u32 },
}

impl CountValue {
    /// Evaluate the count.
    pub fn roll(&self) -> u32 {
        match self {
            CountValue::Fixed(n) => *n,
            CountValue::Range { min, max } => {
                let mut rng = rand::thread_rng();
                rng.gen_range(*min..=*max)
            }
        }
    }
}

fn default_weight() -> u32 {
    1
}

/// A single item drop result.
#[derive(Debug, Clone)]
pub struct LootDrop {
    pub item_name: String,
    pub count: u32,
}

impl LootTableFile {
    /// Parse from a JSON string.
    pub fn parse_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid loot table JSON: {e}"))
    }

    /// Roll all pools and collect drops.
    pub fn roll(&self) -> Vec<LootDrop> {
        let mut drops = Vec::new();
        for pool in &self.pools {
            let n = pool.rolls.roll();
            for _ in 0..n {
                if let Some(drop) = roll_pool(pool) {
                    drops.push(drop);
                }
            }
        }
        drops
    }
}

/// Select one entry from a pool using weighted random selection.
fn roll_pool(pool: &LootPool) -> Option<LootDrop> {
    if pool.entries.is_empty() {
        return None;
    }

    let total_weight: u32 = pool.entries.iter().map(|e| e.weight).sum();
    if total_weight == 0 {
        return None;
    }

    let mut rng = rand::thread_rng();
    let mut roll = rng.gen_range(0..total_weight);

    for entry in &pool.entries {
        if roll < entry.weight {
            return entry_to_drop(entry);
        }
        roll -= entry.weight;
    }

    None
}

/// Convert a selected entry into a drop.
fn entry_to_drop(entry: &LootEntry) -> Option<LootDrop> {
    match entry.entry_type.as_str() {
        "item" => {
            let name = entry.name.as_ref()?;
            let mut count = 1u32;

            // Apply set_count function if present.
            for func in &entry.functions {
                if func.function == "set_count" {
                    if let Some(ref cv) = func.count {
                        count = cv.roll();
                    }
                }
            }

            Some(LootDrop {
                item_name: name.clone(),
                count,
            })
        }
        "empty" => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_loot_table() {
        let json = r#"{
            "pools": [
                {
                    "rolls": 1,
                    "entries": [
                        {
                            "type": "item",
                            "name": "minecraft:diamond",
                            "weight": 1,
                            "functions": [
                                { "function": "set_count", "count": { "min": 1, "max": 3 } }
                            ]
                        },
                        {
                            "type": "empty",
                            "weight": 3
                        }
                    ]
                }
            ]
        }"#;
        let table = LootTableFile::parse_json(json).unwrap();
        assert_eq!(table.pools.len(), 1);
        assert_eq!(table.pools[0].entries.len(), 2);
    }

    #[test]
    fn roll_fixed() {
        let json = r#"{
            "pools": [
                {
                    "rolls": 1,
                    "entries": [
                        {
                            "type": "item",
                            "name": "minecraft:stick",
                            "weight": 1,
                            "functions": [
                                { "function": "set_count", "count": 5 }
                            ]
                        }
                    ]
                }
            ]
        }"#;
        let table = LootTableFile::parse_json(json).unwrap();
        let drops = table.roll();
        assert_eq!(drops.len(), 1);
        assert_eq!(drops[0].item_name, "minecraft:stick");
        assert_eq!(drops[0].count, 5);
    }

    #[test]
    fn roll_empty_pool() {
        let json = r#"{ "pools": [] }"#;
        let table = LootTableFile::parse_json(json).unwrap();
        let drops = table.roll();
        assert!(drops.is_empty());
    }
}
