//! Game rules — port PMMP `src/world/GameRule.php`.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum GameRuleValue {
    Bool(bool),
    Int(i32),
    Float(f32),
}

#[derive(Debug, Clone)]
pub struct GameRules {
    pub rules: HashMap<String, GameRuleValue>,
}

impl Default for GameRules {
    fn default() -> Self {
        Self::vanilla_defaults()
    }
}

impl GameRules {
    pub fn vanilla_defaults() -> Self {
        let mut m = HashMap::new();
        m.insert("commandblockoutput".into(), GameRuleValue::Bool(true));
        m.insert("commandblocksenabled".into(), GameRuleValue::Bool(true));
        m.insert("dodaylightcycle".into(), GameRuleValue::Bool(true));
        m.insert("doentitydrops".into(), GameRuleValue::Bool(true));
        m.insert("dofiretick".into(), GameRuleValue::Bool(true));
        m.insert("doimmediaterespawn".into(), GameRuleValue::Bool(false));
        m.insert("doinsomnia".into(), GameRuleValue::Bool(true));
        m.insert("domobloot".into(), GameRuleValue::Bool(true));
        m.insert("domobspawning".into(), GameRuleValue::Bool(true));
        m.insert("dotiledrops".into(), GameRuleValue::Bool(true));
        m.insert("doweathercycle".into(), GameRuleValue::Bool(true));
        m.insert("drowningdamage".into(), GameRuleValue::Bool(true));
        m.insert("falldamage".into(), GameRuleValue::Bool(true));
        m.insert("firedamage".into(), GameRuleValue::Bool(true));
        m.insert("freezedamage".into(), GameRuleValue::Bool(true));
        m.insert("functioncommandlimit".into(), GameRuleValue::Int(10000));
        m.insert("keepinventory".into(), GameRuleValue::Bool(false));
        m.insert("maxcommandchainlength".into(), GameRuleValue::Int(65536));
        m.insert("mobgriefing".into(), GameRuleValue::Bool(true));
        m.insert("naturalregeneration".into(), GameRuleValue::Bool(true));
        m.insert("playersleepingpercentage".into(), GameRuleValue::Int(100));
        m.insert("pvp".into(), GameRuleValue::Bool(true));
        m.insert("randomtickspeed".into(), GameRuleValue::Int(3));
        m.insert("respawnblocksexplode".into(), GameRuleValue::Bool(true));
        m.insert("sendcommandfeedback".into(), GameRuleValue::Bool(true));
        m.insert("showbordereffect".into(), GameRuleValue::Bool(true));
        m.insert("showcoordinates".into(), GameRuleValue::Bool(false));
        m.insert("showdeathmessages".into(), GameRuleValue::Bool(true));
        m.insert("showtags".into(), GameRuleValue::Bool(true));
        m.insert("spawnradius".into(), GameRuleValue::Int(5));
        m.insert("tntexplodes".into(), GameRuleValue::Bool(true));
        m.insert("universalanger".into(), GameRuleValue::Bool(false));
        m.insert("showrecipemessages".into(), GameRuleValue::Bool(true));
        Self { rules: m }
    }

    pub fn get(&self, name: &str) -> Option<&GameRuleValue> {
        self.rules.get(&name.to_lowercase())
    }

    pub fn set(&mut self, name: impl Into<String>, value: GameRuleValue) {
        self.rules.insert(name.into().to_lowercase(), value);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_load() {
        let r = GameRules::vanilla_defaults();
        assert!(r.bool("pvp"));
        assert_eq!(r.int("randomtickspeed"), 3);
    }

    #[test]
    fn set_overrides() {
        let mut r = GameRules::vanilla_defaults();
        r.set("pvp", GameRuleValue::Bool(false));
        assert!(!r.bool("pvp"));
    }

    #[test]
    fn unknown_rule_returns_default() {
        let r = GameRules::vanilla_defaults();
        assert!(!r.bool("nonexistent"));
    }
}
