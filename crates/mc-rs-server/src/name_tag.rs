//! Name tag — apply custom name to mob, optionally with renamer anvil.

#[derive(Debug, Clone)]
pub struct NameTag {
    pub custom_name: String,
    pub visible: bool,
    pub always_show: bool,
}

/// Max length of mob custom name.
pub const MAX_LENGTH: usize = 32;

impl NameTag {
    pub fn new(name: String) -> Self {
        Self {
            custom_name: name.chars().take(MAX_LENGTH).collect(),
            visible: true,
            always_show: false,
        }
    }

    pub fn is_dinnerbone_easter_egg(&self) -> bool {
        matches!(self.custom_name.as_str(), "Dinnerbone" | "Grumm")
    }

    pub fn is_jeb_easter_egg(&self) -> bool {
        self.custom_name == "jeb_"
    }

    pub fn is_toast_easter_egg(&self) -> bool {
        self.custom_name == "Toast"
    }

    /// Mobs named with a nametag won't despawn.
    pub fn prevents_despawn() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dinnerbone_flips_mob() {
        let n = NameTag::new("Dinnerbone".to_string());
        assert!(n.is_dinnerbone_easter_egg());
    }

    #[test]
    fn truncates_long_names() {
        let long = "a".repeat(100);
        let n = NameTag::new(long);
        assert_eq!(n.custom_name.len(), MAX_LENGTH);
    }
}
