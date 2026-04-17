//! Block entities — ports de `.reference/PocketMine-MP/src/block/tile/*`.
//!
//! Data model pour chest, furnace, sign, bed, etc. Les block entities sont
//! des blocs avec état persistant (inventaire, texte, recettes en cours...).
//! Ils sont stockés séparément dans le chunk (`BlockActor` en vanilla) et
//! syncés au client via `BlockActorDataPacket`.

use mc_rs_proto::packets::player::ItemStack;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockEntityKind {
    Chest,
    TrappedChest,
    DoubleChest, // déduit par paire lors du chargement
    Furnace,
    BlastFurnace,
    Smoker,
    Barrel,
    Hopper,
    Dispenser,
    Dropper,
    BrewingStand,
    EnchantingTable,
    Sign,
    Bed,
    Banner,
    MobSpawner,
    Jukebox,
    Skull,
    FlowerPot,
    EnderChest,
    ItemFrame,
    Lectern,
    EndGateway,
    ShulkerBox,
    Campfire,
    StructureBlock,
    Beacon,
    Conduit,
    DaylightSensor,
    NoteBlock,
    PistonArm,
    Cauldron,
    Bell,
}

impl BlockEntityKind {
    /// Identifiant string PMMP `TileFactory::TILE_*`.
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Chest | Self::DoubleChest => "Chest",
            Self::TrappedChest => "TrappedChest",
            Self::Furnace => "Furnace",
            Self::BlastFurnace => "BlastFurnace",
            Self::Smoker => "Smoker",
            Self::Barrel => "Barrel",
            Self::Hopper => "Hopper",
            Self::Dispenser => "Dispenser",
            Self::Dropper => "Dropper",
            Self::BrewingStand => "BrewingStand",
            Self::EnchantingTable => "EnchantTable",
            Self::Sign => "Sign",
            Self::Bed => "Bed",
            Self::Banner => "Banner",
            Self::MobSpawner => "MobSpawner",
            Self::Jukebox => "Music",
            Self::Skull => "Skull",
            Self::FlowerPot => "FlowerPot",
            Self::EnderChest => "EnderChest",
            Self::ItemFrame => "ItemFrame",
            Self::Lectern => "Lectern",
            Self::EndGateway => "EndGateway",
            Self::ShulkerBox => "ShulkerBox",
            Self::Campfire => "Campfire",
            Self::StructureBlock => "StructureBlock",
            Self::Beacon => "Beacon",
            Self::Conduit => "Conduit",
            Self::DaylightSensor => "DaylightDetector",
            Self::NoteBlock => "Music",
            Self::PistonArm => "PistonArm",
            Self::Cauldron => "Cauldron",
            Self::Bell => "Bell",
        }
    }

    /// Taille d'inventaire pour les block entities contenants.
    /// PMMP `Inventory::getDefaultSize()` de chaque block inventory.
    pub fn inventory_size(&self) -> Option<usize> {
        match self {
            Self::Chest | Self::TrappedChest | Self::Barrel | Self::ShulkerBox => Some(27),
            Self::DoubleChest => Some(54),
            Self::Furnace | Self::BlastFurnace | Self::Smoker => Some(3), // input, fuel, output
            Self::Hopper => Some(5),
            Self::Dispenser | Self::Dropper => Some(9),
            Self::BrewingStand => Some(5), // 3 potions + fuel + ingredient
            _ => None,
        }
    }
}

/// État partagé d'un block entity.
#[derive(Debug, Clone)]
pub struct BlockEntity {
    pub kind: BlockEntityKind,
    pub position: [i32; 3],
    /// Items (si inventory-backed). Size = kind.inventory_size().unwrap_or(0).
    pub items: Vec<ItemStack>,
    /// Texte pour signs (4 lignes + flags). Pour signs seulement.
    pub sign_text: Option<[String; 4]>,
    /// Custom name (pour chest / furnace avec renaming anvil).
    pub custom_name: Option<String>,
    /// Smelt progress ticks (pour furnaces). 0..200.
    pub cook_time: u16,
    /// Burn time remaining ticks (pour furnaces).
    pub burn_time: u16,
    /// Musique disc (pour jukebox) — network ID de l'item record en cours.
    pub record_item_id: Option<i32>,
}

impl BlockEntity {
    pub fn new(kind: BlockEntityKind, position: [i32; 3]) -> Self {
        let items = vec![ItemStack::AIR; kind.inventory_size().unwrap_or(0)];
        Self {
            kind,
            position,
            items,
            sign_text: None,
            custom_name: None,
            cook_time: 0,
            burn_time: 0,
            record_item_id: None,
        }
    }

    pub fn new_sign(position: [i32; 3]) -> Self {
        Self {
            kind: BlockEntityKind::Sign,
            position,
            items: vec![],
            sign_text: Some([String::new(), String::new(), String::new(), String::new()]),
            custom_name: None,
            cook_time: 0,
            burn_time: 0,
            record_item_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chest_default_27_slots() {
        let be = BlockEntity::new(BlockEntityKind::Chest, [0, 64, 0]);
        assert_eq!(be.items.len(), 27);
    }

    #[test]
    fn furnace_has_3_slots() {
        let be = BlockEntity::new(BlockEntityKind::Furnace, [0, 64, 0]);
        assert_eq!(be.items.len(), 3);
    }

    #[test]
    fn sign_has_no_items_but_text() {
        let be = BlockEntity::new_sign([0, 64, 0]);
        assert_eq!(be.items.len(), 0);
        assert!(be.sign_text.is_some());
    }

    #[test]
    fn identifier_matches_pmmp() {
        assert_eq!(BlockEntityKind::Chest.identifier(), "Chest");
        assert_eq!(BlockEntityKind::Furnace.identifier(), "Furnace");
        assert_eq!(BlockEntityKind::EnderChest.identifier(), "EnderChest");
    }
}
