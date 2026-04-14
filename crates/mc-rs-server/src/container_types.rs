//! Container UI types (window types).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerType {
    Inventory,
    Container,     // chest
    Workbench,
    Furnace,
    BlastFurnace,
    Smoker,
    EnchantingTable,
    BrewingStand,
    Anvil,
    Beacon,
    Dispenser,
    Dropper,
    Hopper,
    ShulkerBox,
    EnderChest,
    Barrel,
    Cartography,
    Grindstone,
    Loom,
    Stonecutter,
    Lectern,
    Smithing,
    HorseInventory,
    Jukebox,
    StructureEditor,
}

impl ContainerType {
    pub fn slot_count(&self) -> usize {
        match self {
            Self::Inventory => 36,
            Self::Container | Self::ShulkerBox | Self::EnderChest | Self::Barrel => 27,
            Self::Workbench => 9,
            Self::Furnace | Self::BlastFurnace | Self::Smoker => 3,
            Self::EnchantingTable => 2,
            Self::BrewingStand => 5,
            Self::Anvil => 3,
            Self::Beacon => 1,
            Self::Dispenser | Self::Dropper => 9,
            Self::Hopper => 5,
            Self::Cartography => 3,
            Self::Grindstone => 3,
            Self::Loom => 4,
            Self::Stonecutter => 2,
            Self::Lectern => 1,
            Self::Smithing => 4,
            Self::HorseInventory => 18,
            Self::Jukebox => 1,
            Self::StructureEditor => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chest_has_27() {
        assert_eq!(ContainerType::Container.slot_count(), 27);
    }

    #[test]
    fn workbench_has_9() {
        assert_eq!(ContainerType::Workbench.slot_count(), 9);
    }
}
