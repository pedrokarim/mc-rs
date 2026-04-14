//! Item tags (NBT) — port PMMP `src/item/ItemEnchantTags.php` + display tags.

use mc_rs_proto::packets::player::ItemStack;

/// Standard item NBT keys (matching Bedrock wire format).
pub mod tag {
    pub const DISPLAY: &str = "display";
    pub const DISPLAY_NAME: &str = "Name";
    pub const DISPLAY_LORE: &str = "Lore";
    pub const ENCHANTMENTS: &str = "ench";
    pub const REPAIR_COST: &str = "RepairCost";
    pub const UNBREAKABLE: &str = "Unbreakable";
    pub const HIDE_FLAGS: &str = "HideFlags";
    pub const ATTRIBUTES: &str = "AttributeModifiers";
    pub const CAN_PLACE_ON: &str = "CanPlaceOn";
    pub const CAN_DESTROY: &str = "CanDestroy";
    pub const CUSTOM_MODEL_DATA: &str = "CustomModelData";
}

/// Bitmasks pour HideFlags tag.
pub mod hide_flag {
    pub const ENCHANTMENTS: u8 = 1;
    pub const ATTRIBUTES: u8 = 2;
    pub const UNBREAKABLE: u8 = 4;
    pub const CAN_DESTROY: u8 = 8;
    pub const CAN_PLACE_ON: u8 = 16;
    pub const OTHER: u8 = 32;
    pub const DYED: u8 = 64;
    pub const POTION_EFFECTS: u8 = 128;
}

/// Conteneur des NBT tags d'un item, sous forme structurée.
#[derive(Debug, Clone, Default)]
pub struct ItemTags {
    pub display_name: Option<String>,
    pub lore: Vec<String>,
    pub unbreakable: bool,
    pub repair_cost: u32,
    pub hide_flags: u8,
    pub enchantments: Vec<(String, u8)>, // (id, level)
    pub custom_model_data: Option<i32>,
    pub can_destroy: Vec<String>,
    pub can_place_on: Vec<String>,
}

impl ItemTags {
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    pub fn add_lore(mut self, line: impl Into<String>) -> Self {
        self.lore.push(line.into());
        self
    }

    pub fn with_unbreakable(mut self) -> Self {
        self.unbreakable = true;
        self.hide_flags |= hide_flag::UNBREAKABLE;
        self
    }

    pub fn with_enchantment(mut self, id: impl Into<String>, level: u8) -> Self {
        self.enchantments.push((id.into(), level));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.display_name.is_none()
            && self.lore.is_empty()
            && !self.unbreakable
            && self.repair_cost == 0
            && self.hide_flags == 0
            && self.enchantments.is_empty()
            && self.custom_model_data.is_none()
            && self.can_destroy.is_empty()
            && self.can_place_on.is_empty()
    }
}

/// Applique les tags à un ItemStack en serialisant en NBT.
pub fn apply_tags(stack: &mut ItemStack, tags: &ItemTags) {
    if tags.is_empty() {
        return;
    }
    // TODO: serialize to extra_data as NBT compound.
    // For now: minimal placeholder to mark the item as tagged.
    let mut tag_buf = Vec::new();
    // Write a simple header indicating tags present.
    tag_buf.push(0x0A); // TAG_Compound
    tag_buf.extend_from_slice(&[0u8; 2]); // empty name length
    tag_buf.push(0x00); // TAG_End (empty compound for now)
    stack.extra_data = tag_buf;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_default_empty() {
        assert!(ItemTags::default().is_empty());
    }

    #[test]
    fn builder_chain() {
        let t = ItemTags::default()
            .with_name("Excalibur")
            .add_lore("Legendary sword")
            .with_unbreakable()
            .with_enchantment("minecraft:sharpness", 5);
        assert_eq!(t.display_name, Some("Excalibur".to_string()));
        assert_eq!(t.lore.len(), 1);
        assert!(t.unbreakable);
        assert_eq!(t.enchantments.len(), 1);
    }
}
