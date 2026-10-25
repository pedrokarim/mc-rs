//! Game rules — port PMMP `src/world/GameRule.php`.
//!
//! Liste, ordre, noms et valeurs par défaut de BDS 1.26.52.3 : les 39 règles
//! de son StartGame (capture du 2026-10-25, `dev-docs/34-BDS-RETRO-INGENIERIE.md`).
//! Les noms sont en camelCase sur le réseau ; la recherche reste insensible à
//! la casse (`/gamerule keepinventory`).

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum GameRuleValue {
    Bool(bool),
    Int(i32),
    Float(f32),
}

/// Règles de BDS 1.26.52.3, dans l'ordre de son StartGame.
const VANILLA_RULES: &[(&str, GameRuleValue)] = &[
    ("commandBlockOutput", GameRuleValue::Bool(true)),
    ("doDayLightCycle", GameRuleValue::Bool(true)),
    ("doEntityDrops", GameRuleValue::Bool(true)),
    ("doFireTick", GameRuleValue::Bool(true)),
    ("recipesUnlock", GameRuleValue::Bool(true)),
    ("doLimitedCrafting", GameRuleValue::Bool(false)),
    ("doMobLoot", GameRuleValue::Bool(true)),
    ("doMobSpawning", GameRuleValue::Bool(true)),
    ("doTileDrops", GameRuleValue::Bool(true)),
    ("doWeatherCycle", GameRuleValue::Bool(true)),
    ("drowningDamage", GameRuleValue::Bool(true)),
    ("fallDamage", GameRuleValue::Bool(true)),
    ("fireDamage", GameRuleValue::Bool(true)),
    ("keepInventory", GameRuleValue::Bool(false)),
    ("mobGriefing", GameRuleValue::Bool(true)),
    ("pvp", GameRuleValue::Bool(true)),
    ("showCoordinates", GameRuleValue::Bool(false)),
    ("playerWaypoints", GameRuleValue::Int(1)),
    ("locatorbar", GameRuleValue::Bool(true)),
    ("showDaysPlayed", GameRuleValue::Bool(false)),
    ("naturalRegeneration", GameRuleValue::Bool(true)),
    ("tntExplodes", GameRuleValue::Bool(true)),
    ("sendCommandFeedback", GameRuleValue::Bool(true)),
    ("maxCommandChainLength", GameRuleValue::Int(65535)),
    ("doInsomnia", GameRuleValue::Bool(true)),
    ("commandBlocksEnabled", GameRuleValue::Bool(true)),
    ("randomTickSpeed", GameRuleValue::Int(1)),
    ("doImmediateRespawn", GameRuleValue::Bool(false)),
    ("showDeathMessages", GameRuleValue::Bool(true)),
    ("functionCommandLimit", GameRuleValue::Int(10000)),
    ("spawnRadius", GameRuleValue::Int(10)),
    ("showTags", GameRuleValue::Bool(true)),
    ("freezeDamage", GameRuleValue::Bool(true)),
    ("respawnBlocksExplode", GameRuleValue::Bool(true)),
    ("showBorderEffect", GameRuleValue::Bool(true)),
    ("showRecipeMessages", GameRuleValue::Bool(true)),
    ("playersSleepingPercentage", GameRuleValue::Int(100)),
    ("projectilesCanBreakBlocks", GameRuleValue::Bool(true)),
    ("tntExplosionDropDecay", GameRuleValue::Bool(false)),
];

#[derive(Debug, Clone)]
pub struct GameRules {
    /// `(nom réseau, valeur)`, dans l'ordre de BDS.
    rules: Vec<(String, GameRuleValue)>,
    /// Nom en minuscules → index dans `rules`.
    index: HashMap<String, usize>,
}

impl Default for GameRules {
    fn default() -> Self {
        Self::vanilla_defaults()
    }
}

impl GameRules {
    pub fn vanilla_defaults() -> Self {
        let rules: Vec<(String, GameRuleValue)> = VANILLA_RULES
            .iter()
            .map(|(name, value)| (name.to_string(), value.clone()))
            .collect();
        let index = rules
            .iter()
            .enumerate()
            .map(|(i, (name, _))| (name.to_ascii_lowercase(), i))
            .collect();
        Self { rules, index }
    }

    /// Nom réseau (camelCase) d'une règle connue, quelle que soit la casse.
    pub fn canonical_name(&self, name: &str) -> Option<&str> {
        self.index
            .get(&name.to_ascii_lowercase())
            .map(|&i| self.rules[i].0.as_str())
    }

    pub fn get(&self, name: &str) -> Option<&GameRuleValue> {
        self.index
            .get(&name.to_ascii_lowercase())
            .map(|&i| &self.rules[i].1)
    }

    /// Modifie une règle connue (sans effet sur un nom inconnu).
    pub fn set(&mut self, name: &str, value: GameRuleValue) {
        if let Some(&i) = self.index.get(&name.to_ascii_lowercase()) {
            self.rules[i].1 = value;
        }
    }

    /// `(nom réseau, valeur)`, dans l'ordre de BDS.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &GameRuleValue)> {
        self.rules
            .iter()
            .map(|(name, value)| (name.as_str(), value))
    }

    /// Règles au format réseau (StartGame, GameRulesChanged), toutes
    /// modifiables par le joueur comme chez BDS.
    pub fn wire_rules(&self) -> Vec<mc_rs_proto::packets::world::GameRule> {
        self.iter()
            .map(|(name, value)| wire_rule(name, value))
            .collect()
    }

    pub fn bool(&self, name: &str) -> bool {
        match self.get(name) {
            Some(GameRuleValue::Bool(b)) => *b,
            _ => false,
        }
    }

    pub fn int(&self, name: &str) -> i32 {
        match self.get(name) {
            Some(GameRuleValue::Int(i)) => *i,
            _ => 0,
        }
    }

    pub fn float(&self, name: &str) -> f32 {
        match self.get(name) {
            Some(GameRuleValue::Float(f)) => *f,
            _ => 0.0,
        }
    }
}

/// Une règle au format réseau, modifiable par le joueur (`CanBeModifiedByPlayer`).
pub fn wire_rule(name: &str, value: &GameRuleValue) -> mc_rs_proto::packets::world::GameRule {
    use mc_rs_proto::packets::world::GameRule;
    match value {
        GameRuleValue::Bool(b) => GameRule::Bool(name.to_string(), true, *b),
        GameRuleValue::Int(i) => GameRule::Int(name.to_string(), true, *i),
        GameRuleValue::Float(f) => GameRule::Float(name.to_string(), true, *f),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_load() {
        let r = GameRules::vanilla_defaults();
        assert!(r.bool("pvp"));
        assert_eq!(r.int("randomtickspeed"), 1);
        assert_eq!(r.iter().count(), 39);
    }

    #[test]
    fn lookup_ignores_case_and_keeps_network_name() {
        let r = GameRules::vanilla_defaults();
        assert_eq!(r.canonical_name("keepinventory"), Some("keepInventory"));
        assert_eq!(r.get("KEEPINVENTORY"), Some(&GameRuleValue::Bool(false)));
        assert_eq!(r.iter().next().map(|(n, _)| n), Some("commandBlockOutput"));
    }

    #[test]
    fn set_overrides() {
        let mut r = GameRules::vanilla_defaults();
        r.set("pvp", GameRuleValue::Bool(false));
        assert!(!r.bool("pvp"));
    }

    #[test]
    fn unknown_rule_returns_default() {
        let mut r = GameRules::vanilla_defaults();
        r.set("nonexistent", GameRuleValue::Bool(true));
        assert!(!r.bool("nonexistent"));
        assert_eq!(r.iter().count(), 39);
    }
}
