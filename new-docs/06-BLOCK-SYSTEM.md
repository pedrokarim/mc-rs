# 06 - Block System

## PocketMine : Système de blocs

### Architecture

```
RuntimeBlockStateRegistry
  └─ maps state_id → Block instance

Block
  ├─ BlockIdentifier (type_id, tile_class?)
  ├─ BlockTypeInfo (name, break_info, tags)
  ├─ State properties (facing, open, lit, waterlogged, etc.)
  └─ Tile? (BlockEntity : Chest, Furnace, Sign, etc.)
```

### State ID

```
state_id = (type_id << 11) | (state_data ^ mask)
```

- `type_id` : identifiant du type de bloc (ex: STONE=1, DIRT=2, etc.)
- 11 bits pour les données d'état interne (2048 combinaisons max)
- Chaque bloc définit ses propriétés d'état

### Block Runtime IDs

Pour le réseau, les blocs sont identifiés par des **runtime IDs** (pas les state IDs internes).
- Mappés via `BlockTranslator`
- `block_network_ids_are_hashes = true` dans StartGame → le client utilise des hash FNV1 au lieu d'une palette envoyée

### Extraction De La Palette Canonique

Pour `mc-rs`, la source des runtime IDs séquentiels et la méthode d’extraction depuis BDS sont documentées dans `25-BLOCK-PALETTE-EXTRACTION.md`.

### Propriétés de blocs

Exemples de propriétés d'état :

| Propriété | Type | Blocs concernés |
|---|---|---|
| `facing` | BlockFace (0-5) | Stairs, Logs, Pistons |
| `open` | bool | Doors, Trapdoors, FenceGates |
| `lit` | bool | Furnace, Campfire, Redstone |
| `waterlogged` | bool | Slabs, Stairs, Fences (layer 2) |
| `half` | Top/Bottom | Slabs, Doors |
| `axis` | X/Y/Z | Logs, Pillars |
| `age` | 0-7/0-15 | Crops, Saplings |
| `power` | 0-15 | Redstone wire |
| `color` | DyeColor (16) | Wool, Concrete, Glass |

### Block Behaviors

Chaque bloc définit ses comportements :

```php
class Block {
    // Interaction
    onInteract(Item, face, clickVector, Player) → bool
    onBreak(Item, Player) → bool
    place(BlockTransaction, Item, blockReplace, blockClicked, face, clickVector, Player) → bool

    // Tick
    onScheduledUpdate() → void      // tick planifié (redstone, crops)
    onNearbyBlockChange() → void    // bloc voisin changé
    onRandomTick() → void           // tick aléatoire (growth)

    // Drops
    getDrops(Item) → Item[]         // items droppés au cassage
    getDropsForCompatibleTool(Item) → Item[]
    getXpDropForTool(Item) → int
    getSilkTouchDrops(Item) → Item[]

    // Physics
    hasEntityCollision() → bool
    onEntityInside(Entity) → bool    // lave, eau, cactus
    getCollisionBoxes() → AxisAlignedBB[]

    // Properties
    getBreakInfo() → BlockBreakInfo  // hardness, tool requirements
    getLightLevel() → int            // lumière émise (0-15)
    getLightFilter() → int           // lumière filtrée
    isTransparent() → bool
    isSolid() → bool
    getFlammability() → int
    getFireEncouragement() → int
}
```

### Block Break Info

```php
class BlockBreakInfo {
    hardness: float          // temps de cassage de base
    toolType: int            // NONE, SHOVEL, PICKAXE, AXE, SHEARS, HOE, SWORD
    toolHarvestLevel: int    // WOOD=1, STONE=2, IRON=3, DIAMOND=4, NETHERITE=5
    blastResistance: float   // résistance aux explosions
}
```

### Block Entities (Tiles)

Certains blocs ont un `Tile` associé qui stocke des données supplémentaires :

| Tile | Bloc | Données |
|---|---|---|
| Chest | Chest, TrappedChest | Inventaire (27 slots) |
| Furnace | Furnace, BlastFurnace, Smoker | Input, fuel, output, cook time |
| Sign | Sign, WallSign | Texte (4 lignes) |
| Bed | Bed | Couleur |
| Banner | Banner | Patterns, couleur de base |
| FlowerPot | FlowerPot | Plante contenue |
| Skull | Skull | Type, rotation |
| EnchantTable | EnchantingTable | Nom custom |
| Hopper | Hopper | Inventaire (5 slots), cooldown |
| Dropper/Dispenser | Dropper, Dispenser | Inventaire (9 slots) |
| BrewingStand | BrewingStand | 3 potions + ingrédient + fuel |
| Lectern | Lectern | Livre, page |
| Jukebox | Jukebox | Disque |
| Barrel | Barrel | Inventaire (27 slots) |
| ShulkerBox | ShulkerBox | Inventaire (27 slots) |
| Campfire | Campfire | 4 items de cuisson |
| Bell | Bell | (rien de spécial) |
| Beacon | Beacon | Niveaux, effets |

### Fichiers PocketMine de référence

```
src/block/Block.php                    → Classe de base
src/block/BlockIdentifier.php          → Identifiant
src/block/BlockTypeIds.php             → Tous les type IDs
src/block/BlockTypeInfo.php            → Info (nom, break)
src/block/BlockBreakInfo.php           → Cassage
src/block/RuntimeBlockStateRegistry.php → Registre runtime
generated/block/VanillaBlocks.php      → Blocs vanilla
src/block/tile/Tile.php                → Base tile
src/block/tile/TileFactory.php         → Factory
src/block/tile/*.php                   → ~38 tiles
src/network/mcpe/convert/BlockTranslator.php → Conversion réseau
```

---

## Équivalent Rust

### Crate : `mc-rs-block`

```rust
/// Identifiant de type de bloc
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockTypeId(pub u32);

/// État complet d'un bloc
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockState {
    pub type_id: BlockTypeId,
    pub state_data: u16,  // 11 bits de données d'état
}

impl BlockState {
    pub fn runtime_id(&self) -> u32 {
        (self.type_id.0 << 11) | (self.state_data as u32)
    }
}

/// Trait pour tous les comportements de blocs
pub trait BlockBehavior: Send + Sync {
    fn on_interact(&self, ctx: &mut BlockContext) -> bool { false }
    fn on_break(&self, ctx: &mut BlockContext) -> Vec<ItemStack> { vec![] }
    fn on_place(&self, ctx: &mut BlockContext) -> bool { true }
    fn on_scheduled_update(&self, ctx: &mut BlockContext) {}
    fn on_random_tick(&self, ctx: &mut BlockContext) {}
    fn on_nearby_block_change(&self, ctx: &mut BlockContext) {}
    fn on_entity_inside(&self, ctx: &mut BlockContext, entity: EntityId) -> bool { false }

    fn break_info(&self) -> &BlockBreakInfo;
    fn light_level(&self) -> u8 { 0 }
    fn light_filter(&self) -> u8 { 15 }
    fn is_solid(&self) -> bool { true }
    fn is_transparent(&self) -> bool { false }
    fn collision_boxes(&self) -> Vec<Aabb> { vec![Aabb::FULL_BLOCK] }
}

pub struct BlockBreakInfo {
    pub hardness: f32,
    pub tool_type: ToolType,
    pub tool_harvest_level: u8,
    pub blast_resistance: f32,
    pub requires_correct_tool: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ToolType {
    None,
    Sword,
    Shovel,
    Pickaxe,
    Axe,
    Shears,
    Hoe,
}

/// Registre global des blocs
pub struct BlockRegistry {
    behaviors: HashMap<BlockTypeId, Box<dyn BlockBehavior>>,
    state_to_network: HashMap<BlockState, u32>,  // state → network runtime ID
    network_to_state: HashMap<u32, BlockState>,  // network runtime ID → state
}
```

### Block Entities (Tiles)

```rust
pub trait BlockEntity: Send + Sync {
    fn id(&self) -> &str;
    fn tick(&mut self, world: &mut World) {}
    fn save_nbt(&self) -> NbtCompound;
    fn load_nbt(&mut self, nbt: &NbtCompound);
    fn spawn_data(&self) -> Option<NbtCompound> { None }  // envoyé au client
}

pub struct ChestBlockEntity {
    pub inventory: Inventory<27>,
    pub custom_name: Option<String>,
}

pub struct FurnaceBlockEntity {
    pub input: ItemStack,
    pub fuel: ItemStack,
    pub output: ItemStack,
    pub burn_time: i16,
    pub cook_time: i16,
    pub max_burn_time: i16,
}

pub struct SignBlockEntity {
    pub front_text: [String; 4],
    pub back_text: [String; 4],
    pub is_waxed: bool,
}
// ... etc pour chaque type
```
