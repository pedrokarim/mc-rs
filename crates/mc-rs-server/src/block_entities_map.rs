//! Block entity types registry.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockEntityKind {
    Chest,
    TrappedChest,
    EnderChest,
    Furnace,
    BlastFurnace,
    Smoker,
    Sign,
    HangingSign,
    Spawner,
    Beacon,
    Bed,
    Cauldron,
    BrewingStand,
    Hopper,
    Dispenser,
    Dropper,
    Jukebox,
    Banner,
    Skull,
    DaylightDetector,
    Comparator,
    FlowerPot,
    ShulkerBox,
    Barrel,
    Campfire,
    Lectern,
    JigsawBlock,
    StructureBlock,
    ConduitBlock,
    Bell,
    BeeNest,
    Beehive,
    SculkSensor,
    CalibratedSculkSensor,
    SculkCatalyst,
    SculkShrieker,
    PigletFlame,
    EndGateway,
    EndPortal,
    EnchantmentTable,
    DecoratedPot,
    ChiseledBookshelf,
    TrialSpawner,
    Vault,
    Crafter,
    CreakingHeart,
}

impl BlockEntityKind {
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Chest => "Chest",
            Self::TrappedChest => "Chest",
            Self::EnderChest => "EnderChest",
            Self::Furnace => "Furnace",
            Self::BlastFurnace => "BlastFurnace",
            Self::Smoker => "Smoker",
            Self::Sign => "Sign",
            Self::HangingSign => "HangingSign",
            Self::Spawner => "MobSpawner",
            Self::Beacon => "Beacon",
            Self::Bed => "Bed",
            Self::Cauldron => "Cauldron",
            Self::BrewingStand => "BrewingStand",
            Self::Hopper => "Hopper",
            Self::Dispenser => "Dispenser",
            Self::Dropper => "Dropper",
            Self::Jukebox => "Jukebox",
            Self::Banner => "Banner",
            Self::Skull => "Skull",
            Self::DaylightDetector => "DaylightDetector",
            Self::Comparator => "Comparator",
            Self::FlowerPot => "FlowerPot",
            Self::ShulkerBox => "ShulkerBox",
            Self::Barrel => "Barrel",
            Self::Campfire => "Campfire",
            Self::Lectern => "Lectern",
            Self::JigsawBlock => "JigsawBlock",
            Self::StructureBlock => "StructureBlock",
            Self::ConduitBlock => "Conduit",
            Self::Bell => "Bell",
            Self::BeeNest => "BeeNest",
            Self::Beehive => "Beehive",
            Self::SculkSensor => "SculkSensor",
            Self::CalibratedSculkSensor => "CalibratedSculkSensor",
            Self::SculkCatalyst => "SculkCatalyst",
            Self::SculkShrieker => "SculkShrieker",
            Self::PigletFlame => "PigletFlame",
            Self::EndGateway => "EndGateway",
            Self::EndPortal => "EndPortal",
            Self::EnchantmentTable => "EnchantTable",
            Self::DecoratedPot => "DecoratedPot",
            Self::ChiseledBookshelf => "ChiseledBookshelf",
            Self::TrialSpawner => "TrialSpawner",
            Self::Vault => "Vault",
            Self::Crafter => "Crafter",
            Self::CreakingHeart => "CreakingHeart",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chest_identifier() {
        assert_eq!(BlockEntityKind::Chest.identifier(), "Chest");
    }
}
