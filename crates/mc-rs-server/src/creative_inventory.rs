//! Creative inventory tabs/groups (full Bedrock list).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreativeTab {
    Construction,
    Nature,
    Equipment,
    Items,
    Search,
    Survival,
    Hotbar,
    SettableHotbar,
    Inventory,
}

impl CreativeTab {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Construction => "construction",
            Self::Nature => "nature",
            Self::Equipment => "equipment",
            Self::Items => "items",
            Self::Search => "search",
            Self::Survival => "survival",
            Self::Hotbar => "hotbar",
            Self::SettableHotbar => "settable_hotbar",
            Self::Inventory => "inventory",
        }
    }
}

/// Number of creative groups (categories).
pub const TAB_COUNT: usize = 9;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nature_name() {
        assert_eq!(CreativeTab::Nature.name(), "nature");
    }
}
