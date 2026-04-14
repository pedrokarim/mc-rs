//! Game rules catalog.

#[derive(Debug, Clone)]
pub enum GameRuleValue {
    Bool(bool),
    Int(i32),
    Float(f32),
}

#[derive(Debug, Clone)]
pub struct GameRule {
    pub name: &'static str,
    pub default: GameRuleValue,
    pub editable: bool,
}

pub fn all_rules() -> Vec<GameRule> {
    use GameRuleValue::*;
    vec![
        GameRule { name: "doDaylightCycle", default: Bool(true), editable: true },
        GameRule { name: "keepInventory", default: Bool(false), editable: true },
        GameRule { name: "doMobSpawning", default: Bool(true), editable: true },
        GameRule { name: "doMobLoot", default: Bool(true), editable: true },
        GameRule { name: "doFireTick", default: Bool(true), editable: true },
        GameRule { name: "doTileDrops", default: Bool(true), editable: true },
        GameRule { name: "doWeatherCycle", default: Bool(true), editable: true },
        GameRule { name: "doImmediateRespawn", default: Bool(false), editable: true },
        GameRule { name: "doInsomnia", default: Bool(true), editable: true },
        GameRule { name: "doLimitedCrafting", default: Bool(false), editable: true },
        GameRule { name: "doPatrolSpawning", default: Bool(true), editable: true },
        GameRule { name: "doTraderSpawning", default: Bool(true), editable: true },
        GameRule { name: "doWardenSpawning", default: Bool(true), editable: true },
        GameRule { name: "doEntityDrops", default: Bool(true), editable: true },
        GameRule { name: "mobGriefing", default: Bool(true), editable: true },
        GameRule { name: "naturalRegeneration", default: Bool(true), editable: true },
        GameRule { name: "pvp", default: Bool(true), editable: true },
        GameRule { name: "sendCommandFeedback", default: Bool(true), editable: true },
        GameRule { name: "showCoordinates", default: Bool(false), editable: true },
        GameRule { name: "showDeathMessages", default: Bool(true), editable: true },
        GameRule { name: "showTags", default: Bool(true), editable: true },
        GameRule { name: "commandBlockOutput", default: Bool(true), editable: true },
        GameRule { name: "commandBlocksEnabled", default: Bool(true), editable: true },
        GameRule { name: "logAdminCommands", default: Bool(true), editable: true },
        GameRule { name: "disableElytraMovementCheck", default: Bool(false), editable: true },
        GameRule { name: "drowningDamage", default: Bool(true), editable: true },
        GameRule { name: "fallDamage", default: Bool(true), editable: true },
        GameRule { name: "fireDamage", default: Bool(true), editable: true },
        GameRule { name: "freezeDamage", default: Bool(true), editable: true },
        GameRule { name: "randomTickSpeed", default: Int(3), editable: true },
        GameRule { name: "maxCommandChainLength", default: Int(65536), editable: true },
        GameRule { name: "playersSleepingPercentage", default: Int(100), editable: true },
        GameRule { name: "respawnBlocksExplode", default: Bool(true), editable: true },
        GameRule { name: "spawnRadius", default: Int(10), editable: true },
        GameRule { name: "tntExplodes", default: Bool(true), editable: true },
        GameRule { name: "functionCommandLimit", default: Int(10000), editable: true },
        GameRule { name: "maxEntityCramming", default: Int(24), editable: true },
        GameRule { name: "spawnChunkRadius", default: Int(2), editable: true },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_keep_inventory() {
        assert!(all_rules().iter().any(|r| r.name == "keepInventory"));
    }
}
