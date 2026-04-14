//! Adventure mode restrictions.

/// Can break blocks?
pub const CAN_BREAK_BLOCKS: bool = false;
/// Can place blocks?
pub const CAN_PLACE_BLOCKS: bool = false;
/// Can open doors/chests?
pub const CAN_INTERACT: bool = true;
/// Can attack entities?
pub const CAN_ATTACK: bool = true;

/// Tool/item CanDestroy NBT whitelist.
/// Each item can be marked with this to allow breaking specific blocks.
#[derive(Debug, Clone, Default)]
pub struct CanDestroyTag {
    pub blocks: Vec<String>,
}

/// Tool/item CanPlaceOn NBT whitelist.
#[derive(Debug, Clone, Default)]
pub struct CanPlaceOnTag {
    pub blocks: Vec<String>,
}

impl CanDestroyTag {
    pub fn allows(&self, block: &str) -> bool {
        self.blocks.iter().any(|b| b == block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_destroy_denies_all() {
        let c = CanDestroyTag::default();
        assert!(!c.allows("minecraft:stone"));
    }
}
