//! Block entities — ports de `.reference/PocketMine-MP/src/block/tile/*`.
//!
//! Data model pour chest, furnace, sign, bed, etc. Les block entities sont
//! des blocs avec état persistant (inventaire, texte, recettes en cours...).
//! Ils sont stockés séparément dans le chunk (`BlockActor` en vanilla) et
//! syncés au client via `BlockActorDataPacket`.

use mc_rs_nbt::{NbtCompound, NbtRoot, NbtTag};
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
    /// Pour MobSpawner : identifiant de l'entité à faire apparaître
    /// (`minecraft:blaze`…). Sert au rendu (mob qui tourne) et au tick de spawn.
    pub spawn_entity: Option<String>,
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
            spawn_entity: None,
        }
    }

    /// Spawner de mobs pré-rempli avec l'entité à faire apparaître (`minecraft:blaze`).
    pub fn new_spawner(position: [i32; 3], entity: &str) -> Self {
        let mut be = Self::new(BlockEntityKind::MobSpawner, position);
        be.spawn_entity = Some(entity.to_string());
        be
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
            spawn_entity: None,
        }
    }

    /// Compound NBT vanilla de ce block entity (champs communs + spécifiques).
    fn to_compound(&self) -> NbtCompound {
        let mut c = NbtCompound::new();
        c.insert("id".into(), NbtTag::String(self.kind.identifier().into()));
        c.insert("x".into(), NbtTag::Int(self.position[0]));
        c.insert("y".into(), NbtTag::Int(self.position[1]));
        c.insert("z".into(), NbtTag::Int(self.position[2]));
        c.insert("isMovable".into(), NbtTag::Byte(1));
        if let Some(name) = &self.custom_name {
            c.insert("CustomName".into(), NbtTag::String(name.clone()));
        }
        match self.kind {
            BlockEntityKind::MobSpawner => {
                // Champs vanilla (réf PMMP `MonsterSpawner.php`).
                if let Some(entity) = &self.spawn_entity {
                    c.insert("EntityIdentifier".into(), NbtTag::String(entity.clone()));
                }
                c.insert("Delay".into(), NbtTag::Short(0));
                c.insert("MinSpawnDelay".into(), NbtTag::Short(200));
                c.insert("MaxSpawnDelay".into(), NbtTag::Short(800));
                c.insert("SpawnCount".into(), NbtTag::Short(4));
                c.insert("SpawnRange".into(), NbtTag::Short(4));
                c.insert("MaxNearbyEntities".into(), NbtTag::Short(6));
                c.insert("RequiredPlayerRange".into(), NbtTag::Short(16));
                // Dimensions de l'entité affichée (mob qui tourne dans le bloc).
                // Sans elles le client ne rend pas le mob. Valeurs blaze (~0.6×1.8).
                c.insert("DisplayEntityWidth".into(), NbtTag::Float(0.6));
                c.insert("DisplayEntityHeight".into(), NbtTag::Float(1.8));
                c.insert("DisplayEntityScale".into(), NbtTag::Float(1.0));
            }
            _ if self.kind.inventory_size().is_some() => {
                // Inventaire (coffres…). v1 : liste vide si pas d'items (butin = suite).
                let items: Vec<NbtTag> = self
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, it)| !it.is_air())
                    .filter_map(|(slot, it)| item_compound(it, slot as u8).map(NbtTag::Compound))
                    .collect();
                c.insert("Items".into(), NbtTag::List(items));
            }
            _ => {}
        }
        c
    }

    /// Reconstruit un block entity depuis un compound NBT (chargement disque).
    fn from_compound(c: &NbtCompound) -> Option<Self> {
        let id = match c.get("id") {
            Some(NbtTag::String(s)) => s.as_str(),
            _ => return None,
        };
        let geti = |k: &str| match c.get(k) {
            Some(NbtTag::Int(v)) => *v,
            _ => 0,
        };
        let kind = kind_from_identifier(id)?;
        let mut be = BlockEntity::new(kind, [geti("x"), geti("y"), geti("z")]);
        if let Some(NbtTag::String(name)) = c.get("CustomName") {
            be.custom_name = Some(name.clone());
        }
        if let Some(NbtTag::String(entity)) = c.get("EntityIdentifier") {
            be.spawn_entity = Some(entity.clone());
        }
        Some(be)
    }
}

/// Compound NBT d'un item d'inventaire (slot + id string + count). `None` si l'item
/// n'a pas de nom connu (les ids réseau sont convertis ailleurs ; v1 = sauté).
fn item_compound(_item: &ItemStack, _slot: u8) -> Option<NbtCompound> {
    // Le mapping id réseau -> nom string vanilla n'est pas encore branché ici ; les
    // coffres générés sont vides en v1 (le butin viendra avec le tick/ouverture).
    None
}

/// Identifiant string vanilla -> `BlockEntityKind` (sous-ensemble utile à la gen).
fn kind_from_identifier(id: &str) -> Option<BlockEntityKind> {
    Some(match id {
        "Chest" => BlockEntityKind::Chest,
        "TrappedChest" => BlockEntityKind::TrappedChest,
        "MobSpawner" => BlockEntityKind::MobSpawner,
        "Furnace" => BlockEntityKind::Furnace,
        "Sign" => BlockEntityKind::Sign,
        "EnderChest" => BlockEntityKind::EnderChest,
        "Barrel" => BlockEntityKind::Barrel,
        _ => return None,
    })
}

/// Sérialise une liste de block entities en NBT **réseau** concaténée (pour le payload
/// du LevelChunk : après le byte border-blocks, sans compteur — réf PMMP ChunkSerializer).
pub fn serialize_network(bes: &[BlockEntity]) -> Vec<u8> {
    let mut buf = Vec::new();
    for be in bes {
        mc_rs_nbt::write_nbt_network(&mut buf, &NbtRoot::new("", be.to_compound()));
    }
    buf
}

/// Sérialise une liste de block entities en NBT **little-endian** concaténée (clé
/// LevelDB '1' = prefix 49), une compound par block entity.
pub fn serialize_le(bes: &[BlockEntity]) -> Vec<u8> {
    let mut buf = Vec::new();
    for be in bes {
        mc_rs_nbt::write_nbt_le(&mut buf, &NbtRoot::new("", be.to_compound()));
    }
    buf
}

/// Parse une liste de block entities depuis le NBT little-endian concaténé (disque).
pub fn parse_le(data: &[u8]) -> Vec<BlockEntity> {
    let mut out = Vec::new();
    let mut buf: &[u8] = data;
    while !buf.is_empty() {
        match mc_rs_nbt::read_nbt_le(&mut buf) {
            Ok(root) => {
                if let Some(be) = BlockEntity::from_compound(&root.compound) {
                    out.push(be);
                }
            }
            Err(_) => break,
        }
    }
    out
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

    #[test]
    fn le_roundtrip_chest_and_spawner() {
        let chest = BlockEntity::new(BlockEntityKind::Chest, [10, 64, -20]);
        let spawner = BlockEntity::new_spawner([11, 65, -20], "minecraft:blaze");
        let blob = serialize_le(&[chest, spawner]);
        let parsed = parse_le(&blob);
        assert_eq!(parsed.len(), 2, "2 block entities round-trip");
        assert_eq!(parsed[0].kind, BlockEntityKind::Chest);
        assert_eq!(parsed[0].position, [10, 64, -20]);
        assert_eq!(parsed[1].kind, BlockEntityKind::MobSpawner);
        assert_eq!(parsed[1].spawn_entity.as_deref(), Some("minecraft:blaze"));
    }

    #[test]
    fn network_serialization_non_empty() {
        let be = BlockEntity::new(BlockEntityKind::Chest, [0, 64, 0]);
        assert!(!serialize_network(&[be]).is_empty());
    }
}
