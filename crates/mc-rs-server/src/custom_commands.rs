//! Custom commands (defined by plugins/behavior packs).

#[derive(Debug, Clone)]
pub struct CommandDefinition {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub usage: String,
    pub permission_level: u8,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct CommandParam {
    pub name: String,
    pub param_type: ParamType,
    pub optional: bool,
    pub enum_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    Int,
    Float,
    String,
    Value,
    WildcardInt,
    Operator,
    Target,
    Position,
    Message,
    Raw,
    Json,
    BlockState,
    Enum,
}

impl CommandDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            description: String::new(),
            usage: String::new(),
            permission_level: 0,
            enabled: true,
        }
    }

    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_by_default() {
        let c = CommandDefinition::new("test");
        assert!(c.enabled);
    }
}
